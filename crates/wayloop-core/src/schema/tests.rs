//! Tests of the schema interpreter (SPEC-0001 T4: R1, R12; ADR 0009 C3).
//!
//! Every schema and document of these tests goes through [`check`], which also asks the oracle,
//! a JSON Schema validator used only in tests (plan of SPEC-0001, D5), and fails when the two
//! disagree on the verdict. The oracle checks the named formats with regular expressions
//! written from the specification, independently of the hand-written parsers of the core.

use std::sync::OnceLock;

use proptest::prelude::*;
use regex::Regex;
use serde_json::{Value, json};

use super::{LoadError, MAX_DEPTH, Schema};
use crate::{Code, Error, Pointer, Reason};

const CODE: Code = Code::new("artifact_schema");

// Oracle -----------------------------------------------------------------------------------

fn full(pattern: &str) -> Regex {
    Regex::new(&format!("^(?:{pattern})$")).unwrap()
}

/// The checks of the oracle, by format name, written from SPEC-0001 (Identifiers, fields).
fn oracle_format(name: &str, text: &str) -> bool {
    static SEQ: OnceLock<Regex> = OnceLock::new();
    static MILESTONE: OnceLock<Regex> = OnceLock::new();
    static HANDOVER: OnceLock<Regex> = OnceLock::new();
    static DATE_TIME: OnceLock<Regex> = OnceLock::new();
    static LANGUAGE: OnceLock<Regex> = OnceLock::new();
    static HANDLE: OnceLock<Regex> = OnceLock::new();
    let seq = SEQ.get_or_init(|| full("([A-Z]+)-([0-9]{4}|[1-9][0-9]{4,})"));
    let prefix = |p: &str| seq.captures(text).is_some_and(|c| &c[1] == p);
    let part_of = |p: &str| full(&format!("{p}[1-9][0-9]*")).is_match(text);
    match name {
        "brief-id" => prefix("BRIEF"),
        "council-id" => prefix("COUNCIL"),
        "gate-id" => prefix("GATE"),
        "epic-id" => prefix("EPIC"),
        "story-id" => prefix("STORY"),
        "spec-id" => prefix("SPEC"),
        "change-id" => prefix("CHANGE"),
        "task-id" => prefix("TASK"),
        "milestone-id" => MILESTONE
            .get_or_init(|| full("M(0|[1-9][0-9]*)"))
            .is_match(text),
        "handover-id" => HANDOVER
            .get_or_init(|| full("HANDOVER-[1-9][0-9]*-[1-9][0-9]*"))
            .is_match(text),
        "requirement-id" => part_of("R"),
        "criterion-id" => part_of("AC"),
        "decision-id" => part_of("D"),
        "question-id" => part_of("Q"),
        "utc-date-time" => {
            let re = DATE_TIME.get_or_init(|| {
                full("([0-9]{4}-[0-9]{2}-[0-9]{2})T([01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9](\\.[0-9]{1,9})?Z")
            });
            // The calendar date is checked by the oracle's own `date` format.
            re.captures(text)
                .is_some_and(|c| oracle_date().is_valid(&json!(&c[1])))
        }
        "language-tag" => LANGUAGE
            .get_or_init(|| {
                full("(?i)[a-z]{2,3}(-[a-z]{4})?(-([a-z]{2}|[0-9]{3}))?(-([a-z0-9]{5,8}|[0-9][a-z0-9]{3}))*")
            })
            .is_match(text),
        "tracker-handle" => {
            HANDLE
                .get_or_init(|| full("@[A-Za-z0-9](-?[A-Za-z0-9])*"))
                .is_match(text)
                && text.len() <= 40
        }
        _ => panic!("the oracle has no check for the format {name}"),
    }
}

const FORMATS: [&str; 18] = [
    "date",
    "utc-date-time",
    "language-tag",
    "tracker-handle",
    "brief-id",
    "council-id",
    "gate-id",
    "epic-id",
    "story-id",
    "spec-id",
    "change-id",
    "task-id",
    "milestone-id",
    "handover-id",
    "requirement-id",
    "criterion-id",
    "decision-id",
    "question-id",
];

fn oracle_date() -> &'static jsonschema::Validator {
    static DATE: OnceLock<jsonschema::Validator> = OnceLock::new();
    DATE.get_or_init(|| {
        jsonschema::draft202012::options()
            .should_validate_formats(true)
            .build(&json!({"type": "string", "format": "date"}))
            .unwrap()
    })
}

fn oracle(schema: &Value) -> jsonschema::Validator {
    assert!(
        jsonschema::meta::is_valid(schema),
        "not a JSON Schema: {schema}"
    );
    let mut options = jsonschema::draft202012::options()
        .should_validate_formats(true)
        .should_ignore_unknown_formats(false);
    for name in FORMATS.into_iter().filter(|n| *n != "date") {
        options = options.with_format(name, move |text: &str| oracle_format(name, text));
    }
    options.build(schema).unwrap()
}

/// The errors of `document` against `schema`, after checking that the oracle agrees on the
/// verdict and that a second run gives the same errors.
fn check(schema: &Value, document: &Value) -> Vec<Error> {
    let loaded = Schema::load(schema).unwrap();
    let errors = loaded.validate(document, CODE);
    // The oracle knows no bound on integers: the decision of the maintainer of 2026-11-20 is
    // added to its verdict, checked on every number of the document.
    assert_eq!(
        oracle(schema).is_valid(&without_nulls(document)) && !beyond_safe(document),
        errors.is_empty(),
        "the oracle disagrees on {document} against {schema}: {errors:?}"
    );
    assert_eq!(loaded.validate(document, CODE), errors, "not deterministic");
    errors
}

/// `value` without the members of objects set to null, at any depth; elements of arrays are
/// kept. ADR 0009 C2 reads a member set to null as absent, which JSON Schema does not: the
/// oracle is given the document as the interpreter reads it.
fn without_nulls(value: &Value) -> Value {
    match value {
        Value::Object(members) => Value::Object(
            members
                .iter()
                .filter(|(_, v)| !v.is_null())
                .map(|(k, v)| (k.clone(), without_nulls(v)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(without_nulls).collect()),
        other => other.clone(),
    }
}

/// Whether `value` holds an integer beyond plus or minus (2^53 - 1), checked with the
/// arithmetic of floats, independently of the interpreter. It looks at every number, while
/// the interpreter checks only the numbers a schema describes: it matches the interpreter
/// only for documents that their schema describes fully, as every document of these tests
/// that holds such a number is (see `a_number_no_schema_describes_is_not_checked`).
fn beyond_safe(value: &Value) -> bool {
    match value {
        Value::Number(n) => {
            let f = n.as_f64().unwrap();
            f.fract() == 0.0 && f.abs() > 9_007_199_254_740_991.0
                || n.as_i64()
                    .is_some_and(|i| i.unsigned_abs() > 9_007_199_254_740_991)
                || n.as_u64().is_some_and(|u| u > 9_007_199_254_740_991)
        }
        Value::Array(items) => items.iter().any(beyond_safe),
        Value::Object(members) => members.values().any(beyond_safe),
        _ => false,
    }
}

fn at(pointer: &str) -> Pointer {
    Pointer::parse(pointer).unwrap()
}

/// The pointer and the reason of each error, in order.
fn found(errors: &[Error]) -> Vec<(String, Reason)> {
    errors
        .iter()
        .map(|e| (e.pointer().to_string(), e.reason()))
        .collect()
}

fn only(errors: &[Error]) -> &Error {
    assert_eq!(errors.len(), 1, "{errors:?}");
    &errors[0]
}

// Schemas of the tests ----------------------------------------------------------------------

fn brief() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "A brief, reduced",
        "type": "object",
        "additionalProperties": false,
        "required": ["kind", "title", "status", "problem", "goals"],
        "properties": {
            "kind": {"const": "brief"},
            "title": {"type": "string", "minLength": 1, "maxLength": 120},
            "status": {"enum": ["draft", "approved", "superseded"]},
            "problem": {"type": "string"},
            "owner": {"type": "string", "format": "tracker-handle"},
            "goals": {"type": "array", "minItems": 1, "items": {"type": "string"}},
            "schema_version": {"type": "integer", "minimum": 1},
            "superseded_by": {
                "type": "array",
                "minItems": 1,
                "items": {"type": "string", "format": "brief-id"}
            }
        },
        "allOf": [
            {
                "if": {"properties": {"status": {"const": "superseded"}}, "required": ["status"]},
                "then": {"required": ["superseded_by"]}
            }
        ]
    })
}

fn valid_brief() -> Value {
    json!({
        "kind": "brief",
        "title": "Freelancers record their working time",
        "status": "draft",
        "problem": "Recording time breaks the flow.",
        "owner": "@example-owner",
        "goals": ["Record a time entry with one command"],
        "schema_version": 1
    })
}

fn with(mut document: Value, key: &str, value: Value) -> Value {
    document[key] = value;
    document
}

fn without(mut document: Value, key: &str) -> Value {
    document.as_object_mut().unwrap().remove(key);
    document
}

// Valid documents ---------------------------------------------------------------------------

#[test]
fn a_valid_document_gives_no_error() {
    assert_eq!(check(&brief(), &valid_brief()), []);
}

// One reason per keyword ----------------------------------------------------------------------

#[test]
fn required_maps_to_missing() {
    let errors = check(&brief(), &without(valid_brief(), "problem"));
    let error = only(&errors);
    assert_eq!(error.code(), CODE);
    assert_eq!(error.pointer(), &at("/problem"));
    assert_eq!(error.reason(), Reason::Missing);
    assert_eq!(error.expected(), None);
    assert_eq!(error.fix(), None);
}

#[test]
fn a_required_inside_then_maps_to_requires_with_the_condition_in_pointers() {
    let superseded = with(valid_brief(), "status", json!("superseded"));
    let errors = check(&brief(), &superseded);
    let error = only(&errors);
    assert_eq!(error.pointer(), &at("/superseded_by"));
    assert_eq!(error.reason(), Reason::Requires);
    assert_eq!(error.pointers(), Some(&[at("/status")][..]));
    // The condition does not hold: nothing is required.
    assert_eq!(check(&brief(), &valid_brief()), []);
    let given = with(superseded, "superseded_by", json!(["BRIEF-0002"]));
    assert_eq!(check(&brief(), &given), []);
}

#[test]
fn a_field_missing_under_required_and_then_is_reported_once_as_missing() {
    let schema = json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["a", "b"],
        "properties": {"a": {"type": "string"}, "b": {"type": "string"}},
        "if": {"properties": {"a": {"const": "x"}}},
        "then": {"required": ["b"]}
    });
    let errors = check(&schema, &json!({"a": "x"}));
    assert_eq!(found(&errors), [("/b".to_owned(), Reason::Missing)]);
}

#[test]
fn the_pointers_of_requires_name_the_members_the_condition_tested() {
    let schema = json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {"a": {}, "b": {}, "c": {}},
        "if": {"required": ["c", "a"], "properties": {"a": {"const": "x"}}},
        "then": {"required": ["b"]}
    });
    // `a` is tested by `properties` and by `required`: listed once, in byte order.
    let errors = check(&schema, &json!({"c": 1, "a": "x"}));
    let error = only(&errors);
    assert_eq!(error.pointer(), &at("/b"));
    assert_eq!(error.pointers(), Some(&[at("/a"), at("/c")][..]));
    let by_required = json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {"a": {}, "b": {}},
        "if": {"required": ["a"]},
        "then": {"required": ["b"]}
    });
    let errors = check(&by_required, &json!({"a": "x"}));
    assert_eq!(only(&errors).pointers(), Some(&[at("/a")][..]));
}

#[test]
fn a_condition_true_by_an_absent_member_gives_no_pointers() {
    let schema = json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {"a": {}, "b": {}},
        "if": {"properties": {"a": {"const": "x"}}},
        "then": {"required": ["b"]}
    });
    let errors = check(&schema, &json!({}));
    let error = only(&errors);
    assert_eq!(error.pointer(), &at("/b"));
    assert_eq!(error.reason(), Reason::Requires);
    // C2: an empty list would say that no member is involved; the field is left out.
    assert_eq!(error.pointers(), None);
    assert!(
        serde_json::to_value(error)
            .unwrap()
            .get("pointers")
            .is_none()
    );
}

#[test]
fn contains_in_a_condition_selects_a_requirement() {
    // AC35 of SPEC-0001: a rubric is required when the verification contains judge.
    let schema = json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["verification"],
        "properties": {
            "verification": {
                "type": "array",
                "minItems": 1,
                "items": {"enum": ["tdd", "property", "judge"]}
            },
            "rubric": {"type": "string"}
        },
        "allOf": [{
            "if": {"properties": {"verification": {"contains": {"const": "judge"}}}},
            "then": {"required": ["rubric"]}
        }]
    });
    let errors = check(&schema, &json!({"verification": ["tdd", "judge"]}));
    let error = only(&errors);
    assert_eq!(error.pointer(), &at("/rubric"));
    assert_eq!(error.reason(), Reason::Requires);
    assert_eq!(error.pointers(), Some(&[at("/verification")][..]));
    assert_eq!(check(&schema, &json!({"verification": ["tdd"]})), []);
}

#[test]
fn additional_properties_false_maps_to_unknown_field_with_the_closest_key() {
    // SPEC-0001 AC9 and brief.invalid.yaml: the unknown key first, then the missing one.
    let document = with(without(valid_brief(), "problem"), "problema", json!("..."));
    let errors = check(&brief(), &document);
    assert_eq!(
        found(&errors),
        [
            ("/problema".to_owned(), Reason::UnknownField),
            ("/problem".to_owned(), Reason::Missing),
        ]
    );
    assert_eq!(errors[0].fix(), Some(&json!("problem")));
    let titolo = with(
        without(valid_brief(), "title"),
        "titolo",
        json!("Un titolo"),
    );
    let errors = check(&brief(), &titolo);
    assert_eq!(errors[0].pointer(), &at("/titolo"));
    assert_eq!(errors[0].fix(), Some(&json!("title")));
}

#[test]
fn an_unknown_key_far_from_every_known_key_has_no_fix() {
    let errors = check(&brief(), &with(valid_brief(), "zzzz", json!(1)));
    let error = only(&errors);
    assert_eq!(error.reason(), Reason::UnknownField);
    assert_eq!(error.fix(), None);
}

#[test]
fn the_fix_never_names_a_key_already_present() {
    // `titles` is one edit from `title`, which the document already has.
    let errors = check(&brief(), &with(valid_brief(), "titles", json!("x")));
    assert_eq!(only(&errors).fix(), None);
}

#[test]
fn type_maps_to_type_and_stops_at_the_value() {
    let errors = check(&brief(), &with(valid_brief(), "title", json!(42)));
    let error = only(&errors);
    assert_eq!(error.pointer(), &at("/title"));
    assert_eq!(error.reason(), Reason::Type);
    // A wrong type is one error: the keywords of the value are not checked further.
    let errors = check(&brief(), &json!(["not", "an", "object"]));
    assert_eq!(found(&errors), [(String::new(), Reason::Type)]);
}

#[test]
fn an_optional_member_set_to_null_is_absent() {
    // ADR 0009 C2, SPEC-0001: absent and null mean not given.
    assert_eq!(
        check(&brief(), &with(valid_brief(), "owner", Value::Null)),
        []
    );
    assert_eq!(
        check(&brief(), &with(valid_brief(), "superseded_by", Value::Null)),
        []
    );
    // An unknown member set to null is not given either.
    assert_eq!(
        check(&brief(), &with(valid_brief(), "titolo", Value::Null)),
        []
    );
}

#[test]
fn a_required_member_set_to_null_is_missing() {
    let errors = check(&brief(), &with(valid_brief(), "problem", Value::Null));
    let error = only(&errors);
    assert_eq!(error.pointer(), &at("/problem"));
    assert_eq!(error.reason(), Reason::Missing);
    // A condition reads null as absent: a null status selects nothing.
    assert_eq!(
        found(&check(
            &brief(),
            &with(valid_brief(), "status", Value::Null)
        )),
        [("/status".to_owned(), Reason::Missing)]
    );
    // A member required by a `then` and set to null is reported as absent.
    let superseded = with(
        with(valid_brief(), "status", json!("superseded")),
        "superseded_by",
        Value::Null,
    );
    assert_eq!(
        found(&check(&brief(), &superseded)),
        [("/superseded_by".to_owned(), Reason::Requires)]
    );
}

#[test]
fn a_null_element_of_an_array_is_a_value() {
    let errors = check(
        &brief(),
        &with(valid_brief(), "goals", json!(["One", null])),
    );
    assert_eq!(found(&errors), [("/goals/1".to_owned(), Reason::Type)]);
    assert_eq!(only(&errors).message(), "Expected string, found null.");
}

#[test]
fn the_fix_may_name_a_member_set_to_null() {
    let document = with(
        with(valid_brief(), "problem", Value::Null),
        "problema",
        json!("..."),
    );
    let errors = check(&brief(), &document);
    assert_eq!(
        found(&errors),
        [
            ("/problema".to_owned(), Reason::UnknownField),
            ("/problem".to_owned(), Reason::Missing)
        ]
    );
    assert_eq!(errors[0].fix(), Some(&json!("problem")));
}

#[test]
fn an_integer_is_any_number_without_a_fraction() {
    let schema = json!({"type": "integer", "minimum": 1, "maximum": 3});
    for valid in [json!(1), json!(3), json!(2.0), json!(1e0)] {
        assert_eq!(check(&schema, &valid), [], "{valid}");
    }
    let errors = check(&schema, &json!(1.5));
    assert_eq!(found(&errors), [(String::new(), Reason::Type)]);
    for wrong in [json!("1"), json!(true), Value::Null, json!([1])] {
        assert_eq!(
            found(&check(&schema, &wrong)),
            [(String::new(), Reason::Type)]
        );
    }
    let huge = json!({"type": "integer", "maximum": 3});
    assert_eq!(
        found(&check(&huge, &json!(1e300))),
        [(String::new(), Reason::Range)]
    );
    assert_eq!(
        found(&check(&huge, &json!(u64::MAX))),
        [(String::new(), Reason::Range)]
    );
    let low = json!({"type": "integer", "minimum": -3});
    assert_eq!(
        found(&check(&low, &json!(-1e300))),
        [(String::new(), Reason::Range)]
    );
    assert_eq!(
        found(&check(&low, &json!(i64::MAX))),
        [(String::new(), Reason::Range)]
    );
}

/// 2^53 - 1: the range in which an IEEE 754 double holds every integer exactly (RFC 7493,
/// I-JSON).
const SAFE: i64 = 9_007_199_254_740_991;

#[test]
fn a_number_no_schema_describes_is_not_checked() {
    // Called directly: the oracle with the bound added would refuse it. The member of an open
    // object is described by no schema, so its integer is not bounded (canonical JSON keeps a
    // canonical form for it, RFC 8785).
    let schema = Schema::load(&json!({"type": "object"})).unwrap();
    let document = json!({"any": 9_007_199_254_740_993_u64});
    assert_eq!(schema.validate(&document, CODE), []);
}

#[test]
fn digits_beyond_the_precision_of_a_double_are_not_seen() {
    // Numbers are read as IEEE 754 doubles, as RFC 8785 reads them: this one is 1.0.
    let document: Value = serde_json::from_str("1.0000000000000001").unwrap();
    assert_eq!(check(&json!({"type": "integer"}), &document), []);
}

#[test]
fn integers_are_bounded_to_the_safe_range_whatever_the_schema_says() {
    // Decision of the maintainer, 2026-11-20: an integer beyond plus or minus (2^53 - 1) is
    // refused with the reason range, by one error, before any other keyword of the value.
    let integer = json!({"type": "integer"});
    for valid in [json!(SAFE), json!(-SAFE), json!(4364.0), json!(0)] {
        assert_eq!(check(&integer, &valid), [], "{valid}");
    }
    for beyond in [
        json!(SAFE + 1),
        json!(-SAFE - 1),
        json!(9_007_199_254_740_992.0),
        json!(u64::MAX),
        json!(1e300),
    ] {
        let errors = check(&integer, &beyond);
        assert_eq!(found(&errors), [(String::new(), Reason::Range)], "{beyond}");
    }
    let bounded = json!({"type": "integer", "minimum": 0, "maximum": 3});
    assert_eq!(
        found(&check(&bounded, &json!(SAFE + 1))),
        [(String::new(), Reason::Range)]
    );
    let listed = json!({"enum": [1, 2]});
    assert_eq!(
        found(&check(&listed, &json!(-SAFE - 1))),
        [(String::new(), Reason::Range)]
    );
    let untyped = json!({"type": "array", "items": {}});
    assert_eq!(
        found(&check(&untyped, &json!([1, SAFE + 1]))),
        [("/1".to_owned(), Reason::Range)]
    );
}

#[test]
fn numbers_compare_by_value() {
    let schema = json!({"type": "integer", "minimum": 4364, "maximum": 4364});
    assert_eq!(check(&schema, &json!(4364.0)), []);
    assert_eq!(
        found(&check(&schema, &json!(4363.0))),
        [(String::new(), Reason::Range)]
    );
    assert_eq!(
        found(&check(&schema, &json!(4365))),
        [(String::new(), Reason::Range)]
    );
    assert_eq!(check(&json!({"const": 4364}), &json!(4364.0)), []);
}

#[test]
fn integers_of_a_schema_are_within_the_safe_range() {
    let beyond = SAFE + 1;
    let cases = [
        (json!({"type": "integer", "maximum": beyond}), "/maximum"),
        (json!({"type": "integer", "minimum": -beyond}), "/minimum"),
        (json!({"enum": [1, beyond]}), "/enum/1"),
        (json!({"const": -beyond}), "/const"),
    ];
    for (schema, pointer) in cases {
        assert_eq!(invalid(&schema), at(pointer), "{schema}");
    }
    assert!(Schema::load(&json!({"type": "integer", "minimum": -SAFE, "maximum": SAFE})).is_ok());
}

#[test]
fn every_type_of_the_subset_is_checked() {
    let cases = [
        ("boolean", json!(true), json!("true")),
        ("integer", json!(7), json!(7.5)),
        ("string", json!("s"), json!(1)),
        ("array", json!([]), json!({})),
        ("object", json!({}), json!([])),
    ];
    for (name, valid, wrong) in cases {
        let schema = json!({"type": name});
        assert_eq!(check(&schema, &valid), [], "{name}");
        let errors = check(&schema, &wrong);
        assert_eq!(found(&errors), [(String::new(), Reason::Type)], "{name}");
    }
}

#[test]
fn enum_maps_to_enum_with_the_accepted_values_in_expected() {
    let errors = check(
        &brief(),
        &with(valid_brief(), "status", json!("implementing")),
    );
    let error = only(&errors);
    assert_eq!(error.pointer(), &at("/status"));
    assert_eq!(error.reason(), Reason::Enum);
    assert_eq!(
        error.expected(),
        Some(&[json!("draft"), json!("approved"), json!("superseded")][..])
    );
}

#[test]
fn const_maps_to_enum_with_its_value_in_expected() {
    let errors = check(&brief(), &with(valid_brief(), "kind", json!("story")));
    let error = only(&errors);
    assert_eq!(error.reason(), Reason::Enum);
    assert_eq!(error.expected(), Some(&[json!("brief")][..]));
}

#[test]
fn enum_compares_numbers_by_value() {
    let schema = json!({"enum": [1, "one", true, null]});
    for valid in [json!(1), json!(1.0), json!("one"), json!(true), Value::Null] {
        assert_eq!(check(&schema, &valid), [], "{valid}");
    }
    for wrong in [
        json!(2),
        json!(1.5),
        json!("1"),
        json!(false),
        json!([1]),
        json!({}),
    ] {
        assert_eq!(
            found(&check(&schema, &wrong)),
            [(String::new(), Reason::Enum)]
        );
    }
}

#[test]
fn length_keywords_map_to_length() {
    let errors = check(&brief(), &with(valid_brief(), "title", json!("")));
    assert_eq!(found(&errors), [("/title".to_owned(), Reason::Length)]);
    let long = "x".repeat(121);
    let errors = check(&brief(), &with(valid_brief(), "title", json!(long)));
    assert_eq!(found(&errors), [("/title".to_owned(), Reason::Length)]);
    let errors = check(&brief(), &with(valid_brief(), "goals", json!([])));
    assert_eq!(found(&errors), [("/goals".to_owned(), Reason::Length)]);
    let at_most = json!({"type": "array", "maxItems": 1});
    assert_eq!(
        found(&check(&at_most, &json!([1, 2]))),
        [(String::new(), Reason::Length)]
    );
}

#[test]
fn a_length_counts_characters_not_bytes() {
    let schema = json!({"type": "string", "maxLength": 3});
    assert_eq!(check(&schema, &json!("\u{e8}\u{e8}\u{e8}")), []);
    assert_eq!(check(&schema, &json!("\u{1f600}ab")), []);
    assert_eq!(
        found(&check(&schema, &json!("\u{e8}\u{e8}\u{e8}\u{e8}"))),
        [(String::new(), Reason::Length)]
    );
}

#[test]
fn numeric_bounds_map_to_range() {
    let errors = check(&brief(), &with(valid_brief(), "schema_version", json!(0)));
    assert_eq!(
        found(&errors),
        [("/schema_version".to_owned(), Reason::Range)]
    );
}

#[test]
fn format_maps_to_syntax_with_a_fix_when_one_can_be_derived() {
    let errors = check(
        &brief(),
        &with(valid_brief(), "owner", json!("example-owner")),
    );
    let error = only(&errors);
    assert_eq!(error.pointer(), &at("/owner"));
    assert_eq!(error.reason(), Reason::Syntax);
    assert_eq!(error.expected(), None);
    assert_eq!(error.fix(), Some(&json!("@example-owner")));
    let document = with(
        with(valid_brief(), "status", json!("superseded")),
        "superseded_by",
        json!(["BRIEF-0002", "brief-7"]),
    );
    let errors = check(&brief(), &document);
    let error = only(&errors);
    assert_eq!(error.pointer(), &at("/superseded_by/1"));
    assert_eq!(error.fix(), Some(&json!("BRIEF-0007")));
    let errors = check(&brief(), &with(valid_brief(), "owner", json!("@ no way")));
    assert_eq!(only(&errors).fix(), None);
}

#[test]
fn a_format_applies_to_strings_only() {
    let schema = json!({"format": "date"});
    assert_eq!(check(&schema, &json!(20_261_120)), []);
    assert_eq!(check(&schema, &json!("2026-11-20")), []);
    assert_eq!(
        found(&check(&schema, &json!("2026-02-30"))),
        [(String::new(), Reason::Syntax)]
    );
}

#[test]
fn every_named_format_is_known_to_the_interpreter_and_to_the_oracle() {
    for name in FORMATS {
        let schema = json!({"type": "string", "format": name});
        assert!(Schema::load(&schema).is_ok(), "{name}");
        check(&schema, &json!("not one of them"));
    }
}

/// Accepted and refused examples of each named format.
const FORMAT_EXAMPLES: [(&str, &[&str], &[&str]); 18] = [
    (
        "date",
        &[
            "2026-11-20",
            "2024-02-29",
            "2000-02-29",
            "2026-01-31",
            "2026-12-31",
            "2025-02-28",
        ],
        &[
            "2026-04-31",
            "2026-11-20T00:00:00Z",
            "2025-02-29",
            "1900-02-29",
            "2026-13-01",
            "2026-1-01",
            "26-11-20",
            "2026-11-00",
        ],
    ),
    (
        "utc-date-time",
        &["2026-11-20T08:30:00Z", "2026-11-20T23:59:59.123456789Z"],
        &[
            "2026-11-20T08:30:00+01:00",
            "2026-11-20t08:30:00z",
            "2026-11-20T24:00:00Z",
            "2026-11-20T08:60:00Z",
            "2026-11-20T08:30:60Z",
            "2026-11-20T08:30Z",
            "2026-11-20T08:30:00.Z",
            "2026-11-20T08:30:00.1234567890Z",
            "2026-02-30T08:30:00Z",
        ],
    ),
    (
        "language-tag",
        &[
            "en",
            "it",
            "IT",
            "en-GB",
            "zh-Hant-TW",
            "es-419",
            "de-CH-1901",
            "sl-rozaj-biske",
            "ast",
        ],
        &[
            "",
            "e",
            "english",
            "en_GB",
            "en-",
            "-en",
            "en--GB",
            "en-GB-x",
            "en-abcdefghi",
            "i-klingon",
            "en-1a",
            "en-abc",
            "en-ab.de",
        ],
    ),
    (
        "tracker-handle",
        &["@a", "@example-owner", "@A1-b2-c3"],
        &["a", "@", "@-a", "@a-", "@a--b", "@a_b", "@a.b", "example"],
    ),
    (
        "brief-id",
        &["BRIEF-0001", "BRIEF-9999", "BRIEF-10000", "BRIEF-0000"],
        &[
            "BRIEF-1",
            "BRIEF-00001",
            "BRIEF-01000a",
            "brief-0001",
            "TASK-0001",
            "BRIEF0001",
            "BRIEF-",
        ],
    ),
    ("council-id", &["COUNCIL-0001"], &["COUNCIL-001"]),
    ("gate-id", &["GATE-0001"], &["GATE-1"]),
    ("epic-id", &["EPIC-0003"], &["EPIC-03"]),
    ("story-id", &["STORY-0042"], &["STORY-42"]),
    (
        "spec-id",
        &["SPEC-0003", "SPEC-12345"],
        &["SPEC-012345", "SPEC-3"],
    ),
    ("change-id", &["CHANGE-0012"], &["CHANGE-12"]),
    (
        "task-id",
        &["TASK-0107", "TASK-10000"],
        &["TASK-107", "TASK-00107", "TASK-0107 "],
    ),
    (
        "milestone-id",
        &["M0", "M2", "M12"],
        &["M", "M02", "m2", "Milestone 2", "M-2"],
    ),
    (
        "handover-id",
        &["HANDOVER-12-3", "HANDOVER-1-1"],
        &[
            "HANDOVER-0-1",
            "HANDOVER-12-0",
            "HANDOVER-012-3",
            "HANDOVER-12",
            "handover-12-3",
        ],
    ),
    (
        "requirement-id",
        &["R1", "R12"],
        &["R0", "R01", "r1", "R", "AC1", "R1a"],
    ),
    (
        "criterion-id",
        &["AC1", "AC30"],
        &["AC0", "A1", "R1", "ac1"],
    ),
    ("decision-id", &["D1", "D3"], &["D0", "d3", "D"]),
    ("question-id", &["Q1", "Q20"], &["Q0", "QQ1"]),
];

#[test]
fn examples_of_each_format() {
    for (name, valid, invalid) in FORMAT_EXAMPLES {
        let schema = json!({"type": "string", "format": name});
        for text in valid {
            assert_eq!(check(&schema, &json!(text)), [], "{name}: {text}");
        }
        for text in invalid {
            let errors = check(&schema, &json!(text));
            assert_eq!(
                found(&errors),
                [(String::new(), Reason::Syntax)],
                "{name}: {text}"
            );
        }
    }
}

#[test]
fn fixes_of_formats() {
    let cases = [
        ("task-id", "TASK-107", Some("TASK-0107")),
        ("task-id", "TASK-00107", Some("TASK-0107")),
        ("task-id", "task 107", Some("TASK-0107")),
        ("task-id", "TASK_0107", Some("TASK-0107")),
        ("task-id", "TASK-010000", Some("TASK-10000")),
        ("task-id", "STORY-0107", None),
        ("task-id", "TASK-", None),
        ("milestone-id", "Milestone 2", Some("M2")),
        ("milestone-id", "m2", Some("M2")),
        ("milestone-id", "M02", Some("M2")),
        ("milestone-id", "M00", Some("M0")),
        ("milestone-id", "Mx", None),
        ("requirement-id", "r2", Some("R2")),
        ("requirement-id", "R02", Some("R2")),
        ("requirement-id", "R 2", Some("R2")),
        ("requirement-id", "R0", None),
        ("criterion-id", "ac3", Some("AC3")),
        ("handover-id", "handover-012-3", Some("HANDOVER-12-3")),
        ("handover-id", "HANDOVER-12", None),
        ("tracker-handle", "example", Some("@example")),
        ("tracker-handle", "@@example", None),
        ("language-tag", "en_GB", Some("en-GB")),
        ("language-tag", "english", None),
        ("date", "2026-11-20T08:00:00Z", None),
        ("utc-date-time", "2026-11-20 08:00:00Z", None),
    ];
    for (name, text, fix) in cases {
        let schema = json!({"type": "string", "format": name});
        let errors = check(&schema, &json!(text));
        let error = only(&errors);
        assert_eq!(error.reason(), Reason::Syntax, "{name}: {text}");
        assert_eq!(
            error.fix(),
            fix.map(|f| json!(f)).as_ref(),
            "{name}: {text}"
        );
    }
}

// One pass, document order -----------------------------------------------------------------

#[test]
fn three_independent_faults_give_three_errors_in_document_order() {
    let document = json!({
        "kind": "brief",
        "title": "",
        "status": "open",
        "problem": "Recording time breaks the flow.",
        "goals": ["One", 2],
    });
    let errors = check(&brief(), &document);
    assert_eq!(
        found(&errors),
        [
            ("/goals/1".to_owned(), Reason::Type),
            ("/status".to_owned(), Reason::Enum),
            ("/title".to_owned(), Reason::Length),
        ]
    );
}

#[test]
fn errors_of_a_value_come_before_the_errors_inside_it() {
    let schema = json!({
        "type": "array",
        "maxItems": 1,
        "items": {"type": "string"}
    });
    let errors = check(&schema, &json!([1, "a", true]));
    assert_eq!(
        found(&errors),
        [
            (String::new(), Reason::Length),
            ("/0".to_owned(), Reason::Type),
            ("/2".to_owned(), Reason::Type),
        ]
    );
}

#[test]
fn absent_fields_follow_the_present_ones_of_their_object() {
    let schema = json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["z", "a"],
        "properties": {
            "a": {"type": "string"},
            "m": {
                "type": "object",
                "additionalProperties": false,
                "required": ["inner"],
                "properties": {"inner": {"type": "string"}}
            },
            "z": {"type": "string"}
        }
    });
    let errors = check(&schema, &json!({"m": {}, "b": 1}));
    assert_eq!(
        found(&errors),
        [
            ("/b".to_owned(), Reason::UnknownField),
            ("/m/inner".to_owned(), Reason::Missing),
            ("/z".to_owned(), Reason::Missing),
            ("/a".to_owned(), Reason::Missing),
        ]
    );
}

#[test]
fn pointers_escape_keys_as_rfc_6901_says() {
    let schema = json!({"type": "object", "additionalProperties": false, "properties": {}});
    let errors = check(&schema, &json!({"a/b~c": 1}));
    assert_eq!(only(&errors).pointer(), &at("/a~1b~0c"));
}

#[test]
fn a_ref_to_a_def_of_the_same_file_is_followed() {
    let schema = json!({
        "$defs": {
            "task": {"type": "string", "format": "task-id"},
            "tasks": {"type": "array", "items": {"$ref": "#/$defs/task", "description": "A task"}}
        },
        "type": "object",
        "additionalProperties": false,
        "properties": {"depends_on": {"$ref": "#/$defs/tasks"}}
    });
    assert_eq!(check(&schema, &json!({"depends_on": ["TASK-0001"]})), []);
    let errors = check(&schema, &json!({"depends_on": ["TASK-0001", "TASK-1"]}));
    assert_eq!(
        found(&errors),
        [("/depends_on/1".to_owned(), Reason::Syntax)]
    );
}

#[test]
fn an_open_object_checks_its_declared_members_and_accepts_the_others() {
    // Output snapshots are open (ADR 0009 C5, amended on 2026-11-20; ADR 0030 G2).
    let open = json!({
        "type": "object",
        "required": ["a"],
        "properties": {"a": {"type": "string"}, "b": {"type": "integer"}}
    });
    assert_eq!(check(&open, &json!({"a": "x", "unknown": [1]})), []);
    assert_eq!(
        found(&check(&open, &json!({"b": "x", "unknown": 1}))),
        [
            ("/b".to_owned(), Reason::Type),
            ("/a".to_owned(), Reason::Missing)
        ]
    );
}

#[test]
fn an_object_without_properties_is_open_and_a_closed_empty_one_takes_no_key() {
    assert_eq!(check(&json!({"type": "object"}), &json!({"any": 1})), []);
    let closed = json!({"type": "object", "additionalProperties": false});
    let errors = check(&closed, &json!({"any": 1}));
    assert_eq!(found(&errors), [("/any".to_owned(), Reason::UnknownField)]);
}

// Schemas outside the subset are refused when loaded ----------------------------------------

fn refused(schema: &Value) -> LoadError {
    match Schema::load(schema) {
        Ok(_) => panic!("accepted: {schema}"),
        Err(error) => error,
    }
}

#[test]
fn a_keyword_outside_the_subset_is_refused_with_its_place() {
    for keyword in [
        "pattern",
        "anyOf",
        "oneOf",
        "not",
        "else",
        "patternProperties",
        "exclusiveMinimum",
        "uniqueItems",
        "default",
        "unevaluatedProperties",
        "dependentRequired",
        "$anchor",
    ] {
        let schema = json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {"a": {"type": "string", keyword: "x"}}
        });
        assert_eq!(
            refused(&schema),
            LoadError::Unsupported {
                pointer: at("/properties/a"),
                keyword: keyword.to_owned()
            },
            "{keyword}"
        );
    }
}

#[test]
fn keywords_of_a_condition_are_limited() {
    let schema = json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {"a": {"type": "string"}, "b": {"type": "string"}},
        "if": {"properties": {"a": {"minLength": 2}}},
        "then": {"required": ["b"]}
    });
    assert_eq!(
        refused(&schema),
        LoadError::Unsupported {
            pointer: at("/if/properties/a"),
            keyword: "minLength".to_owned()
        }
    );
    let then_with_properties = json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {"a": {"type": "string"}},
        "if": {"required": ["a"]},
        "then": {"properties": {"a": {"const": "x"}}}
    });
    assert_eq!(
        refused(&then_with_properties),
        LoadError::Unsupported {
            pointer: at("/then"),
            keyword: "properties".to_owned()
        }
    );
    let all_of_with_type = json!({"allOf": [{"type": "string"}]});
    assert_eq!(
        refused(&all_of_with_type),
        LoadError::Unsupported {
            pointer: at("/allOf/0"),
            keyword: "type".to_owned()
        }
    );
    let contains_outside = json!({"type": "array", "contains": {"const": 1}});
    assert_eq!(
        refused(&contains_outside),
        LoadError::Unsupported {
            pointer: Pointer::root(),
            keyword: "contains".to_owned()
        }
    );
}

fn invalid(schema: &Value) -> Pointer {
    match refused(schema) {
        LoadError::Invalid { pointer, .. } => pointer,
        other => panic!("{other:?} for {schema}"),
    }
}

#[test]
fn malformed_values_of_keywords_are_refused_with_their_place() {
    let cases = [
        (json!({"type": "number"}), "/type"),
        (json!({"type": "null"}), "/type"),
        (json!({"type": ["string", "null"]}), "/type"),
        (
            json!({"$schema": "http://json-schema.org/draft-07/schema#"}),
            "/$schema",
        ),
        (
            json!({"$id": "x", "type": "object", "properties": {"a": {"$id": "y"}}, "additionalProperties": false}),
            "/properties/a/$id",
        ),
        (
            json!({"type": "object", "additionalProperties": true}),
            "/additionalProperties",
        ),
        (
            json!({"type": "object", "additionalProperties": {}}),
            "/additionalProperties",
        ),
        (
            json!({"type": "object", "additionalProperties": false, "properties": {}, "required": ["a"]}),
            "/required/0",
        ),
        (
            json!({"type": "object", "additionalProperties": false, "properties": {"a": {}}, "required": ["a", "a"]}),
            "/required/1",
        ),
        (
            json!({"type": "object", "additionalProperties": false, "properties": {"a": {}}, "required": "a"}),
            "/required",
        ),
        (json!({"enum": []}), "/enum"),
        (json!({"enum": [{"a": 1}]}), "/enum/0"),
        (json!({"enum": [1.5]}), "/enum/0"),
        (json!({"const": [1]}), "/const"),
        (json!({"type": "string", "minLength": -1}), "/minLength"),
        (json!({"type": "string", "maxLength": 1.5}), "/maxLength"),
        (
            json!({"type": "string", "minLength": 3, "maxLength": 2}),
            "/maxLength",
        ),
        (
            json!({"type": "array", "minItems": 2, "maxItems": 1}),
            "/maxItems",
        ),
        (json!({"minimum": 1}), "/minimum"),
        (json!({"type": "integer", "minimum": 0.5}), "/minimum"),
        (
            json!({"type": "integer", "minimum": 2, "maximum": 1}),
            "/maximum",
        ),
        (json!({"type": "string", "format": "email"}), "/format"),
        (json!({"type": "array", "items": [{}]}), "/items"),
        (json!({"if": {"required": []}}), ""),
        (json!({"then": {"required": []}}), ""),
        (
            json!({"type": "object", "additionalProperties": false, "properties": {"a": {}}, "if": {"required": ["a"]}, "then": {"required": ["b"]}}),
            "/then/required/0",
        ),
        (
            json!({"type": "object", "additionalProperties": false, "properties": {"a": {}}, "if": {"properties": {"b": {"const": 1}}}, "then": {"required": ["a"]}}),
            "/if/properties/b",
        ),
        (json!({"allOf": []}), "/allOf"),
        (json!({"$ref": "#/$defs/none"}), "/$ref"),
        (
            json!({"$ref": "other.json#/$defs/a", "$defs": {"a": {}}}),
            "/$ref",
        ),
        // Escaped and empty names are refused, never matched as written: `a~1b` means `a/b`.
        (
            json!({"$ref": "#/$defs/a~1b", "$defs": {"a~1b": {}}}),
            "/$ref",
        ),
        (json!({"$ref": "#/$defs/", "$defs": {"": {}}}), "/$ref"),
        // A percent-encoded name is refused too, never decoded or matched as written.
        (
            json!({"$ref": "#/$defs/a%20b", "$defs": {"a%20b": {}}}),
            "/$ref",
        ),
        (
            json!({"$ref": "#/$defs/a", "type": "string", "$defs": {"a": {}}}),
            "",
        ),
        (
            json!({"$ref": "#/$defs/a", "$defs": {"a": {"$ref": "#/$defs/b"}, "b": {}}}),
            "/$defs/a",
        ),
        (json!({"$defs": {"a": 1}}), "/$defs/a"),
        (
            json!({"type": "object", "additionalProperties": false, "properties": {"a": {"$defs": {}}}}),
            "/properties/a/$defs",
        ),
        (json!("not a schema"), ""),
        (json!(true), ""),
    ];
    for (schema, pointer) in cases {
        assert_eq!(invalid(&schema), at(pointer), "{schema}");
    }
}

#[test]
fn a_recursive_reference_is_refused() {
    let schema = json!({
        "$defs": {
            "node": {
                "type": "object",
                "additionalProperties": false,
                "properties": {"children": {"type": "array", "items": {"$ref": "#/$defs/node"}}}
            }
        },
        "$ref": "#/$defs/node"
    });
    assert!(matches!(refused(&schema), LoadError::Recursive { .. }));
}

/// A schema of `levels` nested arrays, each counting one level of depth.
fn nested(levels: usize) -> Value {
    let mut schema = json!({"type": "string"});
    for _ in 0..levels {
        schema = json!({"type": "array", "items": schema});
    }
    schema
}

#[test]
fn a_schema_deeper_than_the_bound_is_refused() {
    assert!(Schema::load(&nested(MAX_DEPTH)).is_ok());
    assert!(matches!(
        refused(&nested(MAX_DEPTH + 1)),
        LoadError::TooDeep { .. }
    ));
    // Depth counts through references, so that the bound holds for the expanded schema.
    let mut defs = serde_json::Map::new();
    defs.insert("d0".to_owned(), json!({"type": "string"}));
    for i in 1..=MAX_DEPTH {
        defs.insert(
            format!("d{i}"),
            json!({"type": "array", "items": {"$ref": format!("#/$defs/d{}", i - 1)}}),
        );
    }
    let schema = json!({"$defs": defs, "$ref": format!("#/$defs/d{MAX_DEPTH}")});
    assert!(matches!(refused(&schema), LoadError::TooDeep { .. }));
}

#[test]
fn a_document_deeper_than_the_schema_is_not_walked() {
    // The interpreter descends only where the schema does, so a deep document costs nothing.
    let mut deep = json!("leaf");
    for _ in 0..1_000 {
        deep = json!([deep]);
    }
    assert_eq!(check(&json!({"type": "array"}), &deep), []);
}

#[test]
fn load_errors_name_their_place_for_a_person() {
    let error = refused(&json!({"pattern": "x"}));
    assert_eq!(
        error.to_string(),
        "the keyword `pattern` at the root of the schema is outside the subset of the interpreter"
    );
    let error = refused(&json!({"type": "object", "additionalProperties": true}));
    assert!(
        error.to_string().starts_with("at /additionalProperties: "),
        "{error}"
    );
}

// Property tests -----------------------------------------------------------------------------

/// A schema with one slot for each independent fault, and a document that fills them all.
fn faults_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["kind", "name"],
        "properties": {
            "count": {"type": "integer", "minimum": 0, "maximum": 9},
            "kind": {"const": "brief"},
            "mode": {"enum": ["plain", "special"]},
            "name": {"type": "string", "minLength": 1, "maxLength": 10},
            "nested": {
                "type": "object",
                "additionalProperties": false,
                "properties": {"flag": {"type": "boolean"}}
            },
            "owner": {"type": "string", "format": "tracker-handle"},
            "reason": {"type": "string"},
            "status": {"enum": ["draft", "done"]},
            "tags": {"type": "array", "minItems": 1, "items": {"type": "string"}}
        },
        "allOf": [{
            "if": {"properties": {"mode": {"const": "special"}}, "required": ["mode"]},
            "then": {"required": ["reason"]}
        }]
    })
}

fn faults_document() -> Value {
    json!({
        "count": 3,
        "kind": "brief",
        "mode": "plain",
        "name": "ok",
        "nested": {"flag": true},
        "owner": "@someone",
        "status": "draft",
        "tags": ["a"]
    })
}

proptest! {
    /// ADR 0009 C3: n independent faults give n errors, every one with its pointer, in
    /// document order, in one pass; the oracle agrees on the verdict.
    #[test]
    fn n_independent_faults_give_n_errors_in_document_order(
        faults in proptest::collection::vec(any::<bool>(), 9),
        count in 10_i64..1_000_000,
        name_length in 11_usize..40,
        extra in "zz_[a-z]{1,8}",
    ) {
        let mut document = faults_document();
        // The expected errors, in document order: present keys in key order, then the absent.
        let mut expected: Vec<(String, Reason)> = Vec::new();
        if faults[0] { document["count"] = json!(count); expected.push(("/count".into(), Reason::Range)); }
        if faults[1] { document["name"] = json!("n".repeat(name_length)); expected.push(("/name".into(), Reason::Length)); }
        if faults[2] { document["nested"]["flag"] = json!("yes"); expected.push(("/nested/flag".into(), Reason::Type)); }
        if faults[3] { document["owner"] = json!("someone else"); expected.push(("/owner".into(), Reason::Syntax)); }
        if faults[4] { document["status"] = json!("open"); expected.push(("/status".into(), Reason::Enum)); }
        if faults[5] { document["tags"] = json!([]); expected.push(("/tags".into(), Reason::Length)); }
        if faults[6] { document[extra.as_str()] = json!(1); expected.push((format!("/{extra}"), Reason::UnknownField)); }
        if faults[7] { document.as_object_mut().unwrap().remove("kind"); expected.push(("/kind".into(), Reason::Missing)); }
        if faults[8] { document["mode"] = json!("special"); expected.push(("/reason".into(), Reason::Requires)); }
        let errors = check(&faults_schema(), &document);
        prop_assert_eq!(errors.len(), faults.iter().filter(|f| **f).count());
        prop_assert_eq!(found(&errors), expected);
        prop_assert!(errors.iter().all(|e| e.code() == CODE));
    }

    /// ADR 0009 C2: setting optional members to null never adds an error, wherever the
    /// faults are.
    #[test]
    fn nulling_optional_members_adds_no_error(
        nulled in proptest::collection::vec(any::<bool>(), 7),
        faulty in any::<bool>(),
    ) {
        let mut document = faults_document();
        if faulty {
            document["status"] = json!("open");
        }
        let before = found(&check(&faults_schema(), &document));
        let optional = ["count", "mode", "nested", "owner", "reason", "status", "tags"];
        for (name, null) in optional.iter().zip(&nulled) {
            if *null {
                document[*name] = Value::Null;
            }
        }
        let after = found(&check(&faults_schema(), &document));
        prop_assert!(after.iter().all(|e| before.contains(e)), "{:?} then {:?}", before, after);
    }

    /// Every named format agrees with the check of the oracle, and a fix, when given, is a
    /// value the format accepts.
    #[test]
    fn formats_agree_with_the_oracle_and_fixes_are_accepted(
        format in proptest::sample::select(FORMATS.to_vec()),
        text in prop_oneof![
            "[A-Za-z]{0,9}[ _-]?[0-9]{0,6}(-[0-9]{1,3})?",
            "@?[A-Za-z0-9-]{0,42}",
            "[0-9]{4}-[0-9]{2}-[0-9]{2}(T[0-9]{2}:[0-9]{2}:[0-9]{2}(\\.[0-9]{1,10})?[Zz]?)?",
            "[a-zA-Z]{1,4}([-_][a-zA-Z0-9]{1,9}){0,3}",
            "(BRIEF|COUNCIL|GATE|EPIC|STORY|SPEC|CHANGE|TASK|HANDOVER|M|R|AC|D|Q)[-_ ]?[0-9]{0,6}(-[0-9]{0,3})?",
        ],
    ) {
        let schema = json!({"type": "string", "format": format});
        let errors = check(&schema, &json!(text));
        if let [error] = errors.as_slice() {
            prop_assert_eq!(error.reason(), Reason::Syntax);
            if let Some(fix) = error.fix() {
                prop_assert_ne!(fix, &json!(text));
                prop_assert_eq!(check(&schema, fix), Vec::<Error>::new());
            }
        } else {
            prop_assert!(errors.is_empty());
        }
    }
}

// Messages, boundaries and the places of load errors ------------------------------------------

#[test]
fn messages_name_the_types_and_the_values_for_a_person() {
    let cases = [
        (
            "string",
            json!(1.5),
            "Expected string, found a number with a fraction.",
        ),
        ("integer", json!("x"), "Expected integer, found a string."),
        ("string", json!(3), "Expected string, found an integer."),
        ("boolean", json!([]), "Expected boolean, found an array."),
        ("array", json!({}), "Expected array, found an object."),
        ("object", json!(true), "Expected object, found a boolean."),
        ("string", Value::Null, "Expected string, found null."),
    ];
    for (name, value, message) in cases {
        let errors = check(&json!({"type": name}), &value);
        assert_eq!(only(&errors).message(), message);
    }
    let errors = check(&json!({"enum": ["a", "b"]}), &json!("c"));
    assert_eq!(
        only(&errors).message(),
        "The value must be one of \"a\", \"b\"."
    );
    let errors = check(&json!({"const": "a"}), &json!("c"));
    assert_eq!(only(&errors).message(), "The value must be \"a\".");
}

#[test]
fn bounds_are_inclusive() {
    let text = json!({"type": "string", "minLength": 2, "maxLength": 2});
    assert_eq!(check(&text, &json!("ab")), []);
    let list = json!({"type": "array", "minItems": 2, "maxItems": 2});
    assert_eq!(check(&list, &json!([1, 2])), []);
    assert_eq!(
        found(&check(&list, &json!([1]))),
        [(String::new(), Reason::Length)]
    );
}

#[test]
fn load_errors_point_to_the_place_of_the_fault() {
    let mut beyond = String::new();
    for _ in 0..=MAX_DEPTH {
        beyond.push_str("/items");
    }
    assert_eq!(
        refused(&nested(MAX_DEPTH + 1)),
        LoadError::TooDeep {
            pointer: at(&beyond)
        }
    );
    let recursive = json!({
        "$defs": {"node": {"type": "array", "items": {"$ref": "#/$defs/node"}}},
        "$ref": "#/$defs/node"
    });
    assert_eq!(
        refused(&recursive),
        LoadError::Recursive {
            pointer: at("/$defs/node")
        }
    );
}

/// A condition `levels` levels of `properties` deep, testing the member `a` at each.
fn nested_condition(levels: usize) -> Value {
    let mut condition = json!({"const": 1});
    for _ in 0..levels {
        condition = json!({"properties": {"a": condition}});
    }
    condition
}

/// An object with the member `a`, required when the condition of `levels` levels holds.
fn conditional(levels: usize) -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {"a": {}},
        "if": {"properties": {"a": nested_condition(levels)}},
        "then": {"required": ["a"]}
    })
}

#[test]
fn the_depth_of_a_condition_counts_toward_the_bound() {
    // `if` is at depth 1, its member `a` at 2, the innermost condition at 2 + levels.
    let deepest = MAX_DEPTH - 2;
    assert!(Schema::load(&conditional(deepest)).is_ok());
    assert!(matches!(
        refused(&conditional(deepest + 1)),
        LoadError::TooDeep { .. }
    ));
    // Through a reference, one more level: the definition at 1, `if` at 2, `a` at 3.
    let by_ref = |levels: usize| json!({"$defs": {"c": conditional(levels)}, "$ref": "#/$defs/c"});
    assert!(Schema::load(&by_ref(deepest - 1)).is_ok());
    assert!(matches!(
        refused(&by_ref(deepest)),
        LoadError::TooDeep { .. }
    ));
}

/// Definitions `d0` (a string) to `dn`, each an array of the one before.
fn chain(n: usize) -> serde_json::Map<String, Value> {
    let mut defs = serde_json::Map::new();
    defs.insert("d0".to_owned(), json!({"type": "string"}));
    for i in 1..=n {
        defs.insert(
            format!("d{i}"),
            json!({"type": "array", "items": {"$ref": format!("#/$defs/d{}", i - 1)}}),
        );
    }
    defs
}

#[test]
fn the_bound_holds_exactly_through_references() {
    // An array of `d15`: `d15` at 2, `d0` at 2 + 2 * 15 = 32.
    let at_bound = json!({"$defs": chain(15), "type": "array", "items": {"$ref": "#/$defs/d15"}});
    assert!(Schema::load(&at_bound).is_ok());
    // `d16` at 1, `d0` at 1 + 2 * 16 = 33.
    let beyond = json!({"$defs": chain(16), "$ref": "#/$defs/d16"});
    assert!(matches!(refused(&beyond), LoadError::TooDeep { .. }));
}

#[test]
fn a_definition_measured_once_is_bounded_where_it_is_used_again() {
    // `a` uses `d5` near the root; `z` uses it again under `levels` arrays: the reference at
    // 1 + levels, `d5` at 2 + levels, `d0` at 12 + levels.
    let schema = |levels: usize| {
        let mut deep = json!({"$ref": "#/$defs/d5"});
        for _ in 0..levels {
            deep = json!({"type": "array", "items": deep});
        }
        json!({
            "$defs": chain(5),
            "type": "object",
            "additionalProperties": false,
            "properties": {"a": {"$ref": "#/$defs/d5"}, "z": deep}
        })
    };
    assert!(Schema::load(&schema(MAX_DEPTH - 12)).is_ok());
    assert!(matches!(
        refused(&schema(MAX_DEPTH - 11)),
        LoadError::TooDeep { .. }
    ));
}
