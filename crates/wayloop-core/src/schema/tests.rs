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
    assert_eq!(
        oracle(schema).is_valid(document),
        errors.is_empty(),
        "the oracle disagrees on {document} against {schema}: {errors:?}"
    );
    assert_eq!(loaded.validate(document, CODE), errors, "not deterministic");
    errors
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
fn null_is_a_value_of_its_own_type() {
    let errors = check(&brief(), &with(valid_brief(), "owner", Value::Null));
    assert_eq!(found(&errors), [("/owner".to_owned(), Reason::Type)]);
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
    assert_eq!(check(&low, &json!(i64::MAX)), []);
}

#[test]
fn every_type_of_the_subset_is_checked() {
    let cases = [
        ("null", Value::Null, json!(0)),
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
        &["2026-11-20", "2024-02-29", "2000-02-29"],
        &[
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
        (json!({"type": ["string", "null"]}), "/type"),
        (
            json!({"$schema": "http://json-schema.org/draft-07/schema#"}),
            "/$schema",
        ),
        (
            json!({"$id": "x", "type": "object", "properties": {"a": {"$id": "y"}}, "additionalProperties": false}),
            "/properties/a/$id",
        ),
        (json!({"type": "object", "properties": {"a": {}}}), ""),
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
