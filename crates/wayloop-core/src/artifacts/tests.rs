//! Tests of the artifact model and of `validate` (SPEC-0001 T5: R1, R5, R6, R12, R14, R16;
//! AC1, AC8, AC9, AC25). The contract cases of the command are in `tests/contract/validate/v1/`
//! and run in the tests of `wayloop-cli`; these tests cover the parts the cases cannot reach.

// The helpers take JSON values by value so that a test reads as the document it builds.
#![allow(clippy::needless_pass_by_value)]

use core::num::NonZeroU64;

use serde_json::{Map, Value, json};
use wayloop_schemas::artifact::ArtifactKind;
use wayloop_schemas::contract::{Area, CONTRACTS, EMBEDDED, Embedded, Part, VALIDATE};
use wayloop_schemas::validate::InputKind;
use wayloop_schemas::{NotDone, NotDoneReason, Source};

use super::input::{self, Input};
use super::kinds::{self, Checked, Kind, Migration};
use super::{checks, codes, run_with, validate_with};
use crate::command::{COMMAND_INPUT, INTERNAL_ERROR};
use crate::envelope::without_nulls;
use crate::schema::tests::oracle;
use crate::{Code, Command, Context, Error, Pointer, Reason, run, run_json};

// Helpers -------------------------------------------------------------------------------------

/// The types whose schema is built.
fn built() -> Vec<Kind> {
    kinds::built(&INTERNAL_ERROR).unwrap()
}

/// Validates `input` with the types whose schema is built.
fn validate(input: &Input) -> super::Validation {
    super::validate(input, &built())
}

/// The embedded snapshot at `path` under `schemas/`.
fn embedded(path: &str) -> Embedded {
    *EMBEDDED.iter().find(|e| e.path == path).unwrap()
}

/// The input schema of `validate`, loaded.
fn input_schema() -> crate::schema::Schema {
    let path = VALIDATE.path(Part::Schema, VALIDATE.version);
    kinds::load(embedded(&path), INTERNAL_ERROR).unwrap()
}

fn brief() -> Value {
    json!({
        "kind": "brief",
        "schema_version": 1,
        "id": "BRIEF-0001",
        "title": "Freelancers record their working time from the terminal",
        "status": "approved",
        "origin": "light_requirements",
        "owner": "@example-owner",
        "problem": "Freelancers lose billable hours because recording time breaks their flow.",
        "goals": ["Record a time entry with one command"],
        "users": ["Freelance developers"],
        "scope_in": ["Time entries and monthly export"]
    })
}

fn with(mut document: Value, key: &str, value: Value) -> Value {
    document[key] = value;
    document
}

fn entry(document: Value) -> Value {
    json!({"format": "document", "document": document})
}

/// The input of the set `documents`, read after it passed the input schema and its rules.
fn read(documents: Vec<Value>) -> Input {
    let input = json!({"documents": documents});
    assert_eq!(input_schema().validate(&input, COMMAND_INPUT), []);
    assert_eq!(input::rules(&input), []);
    input::read(&input).unwrap()
}

fn found(errors: &[Error]) -> Vec<(Code, String, Reason)> {
    errors
        .iter()
        .map(|e| (e.code().clone(), e.pointer().to_string(), e.reason()))
        .collect()
}

fn at(pointer: &str) -> Pointer {
    Pointer::parse(pointer).unwrap()
}

fn line(n: u64) -> NonZeroU64 {
    NonZeroU64::new(n).unwrap()
}

// The registry against the files ------------------------------------------------------------

#[test]
fn the_built_kinds_are_the_artifacts_of_the_registry_with_every_version() {
    // ADR 0030 G5: the kinds of the core are built from the registry.
    let built = built();
    let artifacts: Vec<&str> = CONTRACTS
        .iter()
        .filter(|c| c.area == Area::Artifacts)
        .map(|c| c.name)
        .collect();
    assert_eq!(built.iter().map(|k| k.id).collect::<Vec<_>>(), artifacts);
    for (kind, contract) in built.iter().zip(CONTRACTS) {
        assert!(ArtifactKind::from_id(kind.id).is_some(), "{}", kind.id);
        let paths: Vec<&str> = kind.versions.iter().map(|e| e.path).collect();
        let expected: Vec<String> = (1..=contract.version.get())
            .map(|v| contract.path(Part::Schema, core::num::NonZeroU32::new(v).unwrap()))
            .collect();
        assert_eq!(paths, expected);
        assert_eq!(kind.migrations.len() + 1, kind.versions.len());
    }
}

#[test]
fn every_built_schema_loads_and_the_command_schemas_too() {
    let built = built();
    let loaded = kinds::load_all(&built, &INTERNAL_ERROR).unwrap();
    assert_eq!(loaded.len(), built.len());
    for part in [Part::Schema, Part::Output] {
        let path = VALIDATE.path(part, VALIDATE.version);
        kinds::load(embedded(&path), INTERNAL_ERROR).unwrap();
    }
}

#[test]
fn every_kind_and_input_has_a_schema_check_named_after_it() {
    let names = ArtifactKind::ALL
        .iter()
        .map(|k| k.id())
        .chain(InputKind::ALL.iter().map(|k| k.id()));
    for kind in names {
        assert_eq!(kinds::schema_check(kind).id(), format!("schema_{kind}"));
    }
    assert!(kinds::is_input_kind("commit"));
    assert!(!kinds::is_input_kind("brief"));
    assert_eq!(kinds::kinds().len(), ArtifactKind::ALL.len());
}

#[test]
fn the_kinds_and_their_checks_are_snake_case_and_unique() {
    let mut ids: Vec<String> = ArtifactKind::ALL
        .iter()
        .map(|k| k.id().to_owned())
        .chain(InputKind::ALL.iter().map(|k| k.id().to_owned()))
        .collect();
    ids.extend(checks::NOT_BUILT.iter().map(|c| c.id().to_owned()));
    ids.extend(
        [
            checks::DOCUMENT_KIND,
            checks::SCHEMA_VERSION,
            checks::IDENTIFIER_FORMAT,
            checks::MARKDOWN,
        ]
        .iter()
        .map(|c| c.id().to_owned()),
    );
    let count = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), count);
    for id in ids {
        assert!(
            id.bytes()
                .all(|b| b.is_ascii_lowercase() || b == b'_' || b.is_ascii_digit()),
            "{id}"
        );
    }
}

// The brief schema against the oracle -----------------------------------------------------

#[test]
fn the_oracle_agrees_with_the_interpreter_on_briefs() {
    let brief_v1 = embedded("artifacts/brief/v1.json");
    let schema: Value = serde_json::from_str(brief_v1.text).unwrap();
    let loaded = kinds::load(brief_v1, INTERNAL_ERROR).unwrap();
    let oracle = oracle(&schema);
    let documents = [
        brief(),
        with(brief(), "language", json!("it")),
        with(brief(), "language", Value::Null),
        with(brief(), "status", json!("implementing")),
        with(brief(), "status", json!("superseded")),
        with(
            with(brief(), "status", json!("superseded")),
            "superseded_by",
            json!(["BRIEF-0002"]),
        ),
        with(brief(), "superseded_by", json!([])),
        with(brief(), "problema", json!("x")),
        with(brief(), "id", json!("BRIEF-1")),
        with(brief(), "id", json!(1)),
        with(brief(), "owner", json!("example-owner")),
        with(brief(), "goals", json!([])),
        with(brief(), "goals", json!([""])),
        with(brief(), "title", json!("")),
        with(brief(), "title", json!("x".repeat(121))),
        with(brief(), "schema_version", json!(1.0)),
        with(brief(), "schema_version", json!(2)),
        with(brief(), "origin", json!("idea")),
        with(brief(), "scope_out", json!([])),
        with(brief(), "kind", json!("task")),
    ];
    for document in documents {
        let errors = loaded.validate(&document, codes::ARTIFACT_SCHEMA);
        assert_eq!(
            oracle.is_valid(&without_nulls(&document)),
            errors.is_empty(),
            "{document}: {errors:?}"
        );
    }
}

// Versions and migrations (R16, AC25) --------------------------------------------------------

const SAMPLE_V1: Embedded = embedded_test(
    "test/sample/v1.json",
    r#"{
        "type": "object", "additionalProperties": false,
        "required": ["kind", "schema_version", "name"],
        "properties": {
            "kind": {"const": "sample"},
            "schema_version": {"type": "integer"},
            "id": {"type": "string", "format": "task-id"},
            "name": {"type": "string", "minLength": 1}
        }
    }"#,
);

const SAMPLE_V2: Embedded = embedded_test(
    "test/sample/v2.json",
    r#"{
        "type": "object", "additionalProperties": false,
        "required": ["kind", "schema_version", "title"],
        "properties": {
            "kind": {"const": "sample"},
            "schema_version": {"type": "integer", "const": 2},
            "id": {"type": "string", "format": "task-id"},
            "title": {"type": "string", "minLength": 1}
        }
    }"#,
);

/// Version 1 to 2 of the sample: `name` is renamed `title`.
fn rename(mut document: Map<String, Value>) -> Map<String, Value> {
    if let Some(name) = document.remove("name") {
        document.insert("title".into(), name);
    }
    document.insert("schema_version".into(), json!(2));
    document
}

/// A migration that forgets to set the version: the current schema refuses its result.
fn broken(mut document: Map<String, Value>) -> Map<String, Value> {
    document.remove("name");
    document
}

const RENAME: [Migration; 1] = [rename];
const BROKEN: [Migration; 1] = [broken];

/// A test snapshot at `path` with `text`.
const fn embedded_test(path: &'static str, text: &'static str) -> Embedded {
    Embedded { path, text }
}

/// A test-only type with two versions and the migration between them.
fn sample_kind() -> Kind {
    Kind {
        id: "sample",
        versions: vec![SAMPLE_V1, SAMPLE_V2],
        migrations: &RENAME,
    }
}

fn sample() -> kinds::Loaded {
    kinds::load_all(&[sample_kind()], &INTERNAL_ERROR)
        .unwrap()
        .remove(0)
}

fn object(value: Value) -> Map<String, Value> {
    value.as_object().unwrap().clone()
}

fn check_sample(document: Value) -> Checked {
    sample().check(&object(document), &codes::ARTIFACT_SCHEMA, &INTERNAL_ERROR)
}

#[test]
fn a_document_of_an_earlier_version_is_validated_then_migrated() {
    // AC25, second half, on a type with two versions.
    assert_eq!(
        check_sample(json!({"kind": "sample", "schema_version": 1, "name": "Old"})),
        Checked::Valid(object(
            json!({"kind": "sample", "schema_version": 2, "title": "Old"})
        ))
    );
    let current = json!({"kind": "sample", "schema_version": 2, "title": "New"});
    assert_eq!(
        check_sample(current.clone()),
        Checked::Valid(object(current))
    );
}

#[test]
fn a_document_of_an_earlier_version_is_checked_against_its_own_schema() {
    let Checked::Refused(errors) =
        check_sample(json!({"kind": "sample", "schema_version": 1, "title": "x"}))
    else {
        panic!("refused expected");
    };
    assert_eq!(
        found(&errors),
        [
            (
                codes::ARTIFACT_SCHEMA,
                "/title".into(),
                Reason::UnknownField
            ),
            (codes::ARTIFACT_SCHEMA, "/name".into(), Reason::Missing),
        ]
    );
}

#[test]
fn an_unknown_version_is_refused_with_enum_and_the_known_versions() {
    // AC25, first half: R16 for every value that is not a known version.
    for version in [json!(99), json!(0), json!(-1), json!(3), json!(3.0)] {
        let document = json!({"kind": "sample", "schema_version": version, "name": "x"});
        let Checked::Refused(errors) = check_sample(document) else {
            panic!("refused expected for {version}");
        };
        assert_eq!(errors.len(), 1, "{version}");
        let error = &errors[0];
        assert_eq!(*error.code(), codes::SCHEMA_VERSION_UNKNOWN);
        assert_eq!(error.pointer(), &at("/schema_version"));
        assert_eq!(error.reason(), Reason::Enum);
        assert_eq!(error.expected(), Some(&[json!(1), json!(2)][..]));
    }
    // An integer written with a fraction of zero is that integer (plan of SPEC-0001, Numbers).
    assert!(matches!(
        check_sample(json!({"kind": "sample", "schema_version": 1.0, "name": "x"})),
        Checked::Valid(_)
    ));
}

#[test]
fn a_missing_or_mistyped_version_is_checked_against_the_current_schema_in_one_pass() {
    // Audit S3: every error of the document, in one pass (ADR 0009 C3), against the current
    // version, here v2, which knows `title` and not `name`.
    let unknown = (
        codes::ARTIFACT_SCHEMA,
        "/name".to_owned(),
        Reason::UnknownField,
    );
    let title = (codes::ARTIFACT_SCHEMA, "/title".to_owned(), Reason::Missing);
    for (version, reason) in [
        (None, Reason::Missing),
        (Some(Value::Null), Reason::Missing),
        (Some(json!("1")), Reason::Type),
        (Some(json!(1.5)), Reason::Type),
    ] {
        let mut document = json!({"kind": "sample", "name": "x"});
        if let Some(version) = version {
            document["schema_version"] = version;
        }
        let Checked::Refused(errors) = check_sample(document) else {
            panic!("refused expected");
        };
        let version = (codes::ARTIFACT_SCHEMA, "/schema_version".to_owned(), reason);
        assert_eq!(found(&errors), [unknown.clone(), version, title.clone()]);
    }
}

#[test]
fn every_version_of_every_built_schema_requires_an_integer_schema_version() {
    // The rule above relies on it: the current schema reports a missing or mistyped version.
    for kind in built() {
        for snapshot in &kind.versions {
            let schema = kinds::load(*snapshot, INTERNAL_ERROR).unwrap();
            let missing = schema.validate(&json!({}), codes::ARTIFACT_SCHEMA);
            assert!(
                found(&missing).contains(&(
                    codes::ARTIFACT_SCHEMA,
                    "/schema_version".into(),
                    Reason::Missing
                )),
                "{}",
                snapshot.path
            );
            let mistyped = schema.validate(&json!({"schema_version": "1"}), codes::ARTIFACT_SCHEMA);
            assert!(
                found(&mistyped).contains(&(
                    codes::ARTIFACT_SCHEMA,
                    "/schema_version".into(),
                    Reason::Type
                )),
                "{}",
                snapshot.path
            );
        }
    }
}

#[test]
fn a_version_beyond_the_safe_integers_is_refused_with_range() {
    // Review R2: the bound of the plan of SPEC-0001 (Numbers) holds for the version too.
    for version in [
        json!(9_007_199_254_740_992_u64),
        json!(-9_007_199_254_740_992_i64),
        json!(1e300),
    ] {
        let Checked::Refused(errors) =
            check_sample(json!({"kind": "sample", "schema_version": version, "name": "x"}))
        else {
            panic!("refused expected");
        };
        assert_eq!(
            found(&errors),
            [(
                codes::ARTIFACT_SCHEMA,
                "/schema_version".into(),
                Reason::Range
            )],
            "{version}"
        );
    }
    assert!(matches!(
        check_sample(json!({"kind": "sample", "schema_version": 9_007_199_254_740_991_u64, "name": "x"})),
        Checked::Refused(errors) if *errors[0].code() == codes::SCHEMA_VERSION_UNKNOWN
    ));
}

#[test]
fn a_migration_that_gives_an_invalid_document_is_a_failure_of_the_engine() {
    let kind = Kind {
        migrations: &BROKEN,
        ..sample_kind()
    };
    let loaded = kinds::load_all(core::slice::from_ref(&kind), &INTERNAL_ERROR)
        .unwrap()
        .remove(0);
    let document = object(json!({"kind": "sample", "schema_version": 1, "name": "x"}));
    let Checked::Failed(failure) =
        loaded.check(&document, &codes::ARTIFACT_SCHEMA, &INTERNAL_ERROR)
    else {
        panic!("failure expected");
    };
    assert_eq!(*failure.code(), INTERNAL_ERROR);
    assert_eq!(failure.reason(), Reason::Internal);
    // Through validate and run: the result is a failure, exit code 5, whatever else is found.
    let input = json!({"documents": [
        entry(json!({"kind": "sample", "schema_version": 1, "name": "x"})),
        entry(json!({"kind": "ticket"})),
    ]});
    assert_eq!(input::rules(&input), []);
    let read = input::read(&input).unwrap();
    let loaded = kinds::load_all(core::slice::from_ref(&kind), &INTERNAL_ERROR).unwrap();
    let validation = validate_with(&read, &loaded);
    assert_eq!(validation.failures.len(), 1);
    assert_eq!(validation.errors.len(), 1);
    let (outcome, diagnostics, defaults) = run_with(&input, &[kind]);
    assert_eq!(outcome.exit_code(), crate::ExitCode::InternalError);
    assert_eq!(outcome.report().unwrap().first().reason(), Reason::Internal);
    assert_eq!(diagnostics, wayloop_schemas::Diagnostics::default());
    assert_eq!(defaults, []);
}

#[test]
fn a_type_whose_schemas_cannot_be_loaded_is_a_failure_of_the_engine() {
    const NOT_JSON: Embedded = embedded_test("test/broken/v1.json", "{");
    const OUTSIDE: Embedded = embedded_test("test/broken/v1.json", r#"{"pattern": "x"}"#);
    for kind in [
        Kind {
            id: "broken",
            versions: vec![NOT_JSON],
            migrations: &[],
        },
        Kind {
            id: "broken",
            versions: vec![OUTSIDE],
            migrations: &[],
        },
        Kind {
            id: "broken",
            versions: vec![SAMPLE_V1, SAMPLE_V2],
            migrations: &[],
        },
        Kind {
            id: "broken",
            versions: vec![],
            migrations: &[],
        },
    ] {
        let failure = kinds::load_all(core::slice::from_ref(&kind), &INTERNAL_ERROR).unwrap_err();
        assert_eq!(failure.reason(), Reason::Internal, "{failure}");
        let (outcome, _, _) = run_with(&json!({"documents": []}), &[kind]);
        assert_eq!(outcome.exit_code(), crate::ExitCode::InternalError);
    }
}

// The identifier stage (R14) ---------------------------------------------------------------

#[test]
fn a_format_error_of_the_id_has_its_own_code_and_keeps_its_fix() {
    let validation = validate(&read(vec![entry(with(brief(), "id", json!("brief_12")))]));
    let [error] = &validation.errors[..] else {
        panic!("{:?}", validation.errors);
    };
    assert_eq!(*error.code(), codes::IDENTIFIER_FORMAT);
    assert_eq!(error.reason(), Reason::Syntax);
    assert_eq!(error.fix(), Some(&json!("BRIEF-0012")));
    // Another fault of the id keeps the code of the schema.
    let validation = validate(&read(vec![entry(with(brief(), "id", json!(12)))]));
    assert_eq!(
        found(&validation.errors),
        [(codes::ARTIFACT_SCHEMA, "/id".into(), Reason::Type)]
    );
}

// The set ----------------------------------------------------------------------------------

#[test]
fn checks_not_built_are_listed_once_with_their_reason() {
    let validation = validate(&read(vec![
        entry(json!({"kind": "task", "id": "TASK-0001"})),
        entry(json!({"kind": "task"})),
        json!({"format": "markdown", "text": "# SPEC-0001: x"}),
        json!({"format": "document", "document": {"header": "x"}, "input_kind": "commit"}),
        entry(brief()),
    ]));
    assert_eq!(validation.errors, []);
    let not_done: Vec<&str> = validation.not_done.iter().map(|n| n.check().id()).collect();
    assert_eq!(
        not_done,
        [
            "identifier_duplicate",
            "links",
            "markdown",
            "naming",
            "parts",
            "schema_commit",
            "schema_task",
            "statuses",
        ]
    );
    assert!(
        validation
            .not_done
            .iter()
            .all(|n| n.reason() == NotDoneReason::NotBuilt)
    );
    // One check identifier per schema (S6): schema_brief ran, schema_task did not.
    assert_eq!(
        validation
            .checks
            .iter()
            .map(wayloop_schemas::Check::id)
            .collect::<Vec<_>>(),
        [
            "document_kind",
            "schema_version",
            "schema_brief",
            "identifier_format"
        ]
    );
    let summaries: Vec<(Option<&str>, Option<&str>, Option<&str>)> = validation
        .documents
        .iter()
        .map(|d| {
            (
                d.kind.map(wayloop_schemas::artifact::ArtifactKind::id),
                d.input_kind.map(wayloop_schemas::validate::InputKind::id),
                d.id.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        summaries,
        [
            (Some("task"), None, Some("TASK-0001")),
            (Some("task"), None, None),
            (None, None, None),
            (None, Some("commit"), None),
            (Some("brief"), None, Some("BRIEF-0001")),
        ]
    );
    assert_eq!(validation.warnings, []);
    assert_eq!(validation.defaults_applied, []);
    assert_eq!(
        NotDone::new(checks::LINKS, NotDoneReason::NotBuilt),
        validation.not_done[1]
    );
}

#[test]
fn errors_come_in_the_order_of_the_documents_then_of_their_values() {
    let validation = validate(&read(vec![
        entry(with(with(brief(), "status", json!("done")), "zz", json!(1))),
        entry(json!({"kind": 7})),
        entry(json!({"kind": "ticket"})),
        entry(with(brief(), "schema_version", json!(2))),
    ]));
    assert_eq!(
        found(&validation.errors),
        [
            (codes::ARTIFACT_SCHEMA, "/status".into(), Reason::Enum),
            (codes::ARTIFACT_SCHEMA, "/zz".into(), Reason::UnknownField),
            (codes::ARTIFACT_SCHEMA, "/kind".into(), Reason::Type),
            (codes::ARTIFACT_SCHEMA, "/kind".into(), Reason::Enum),
            (
                codes::SCHEMA_VERSION_UNKNOWN,
                "/schema_version".into(),
                Reason::Enum
            ),
        ]
    );
    assert_eq!(
        validation.errors[3].expected().map(<[Value]>::len),
        Some(ArtifactKind::ALL.len())
    );
    // Same input, same result.
    let again = validate(&read(vec![
        entry(with(with(brief(), "status", json!("done")), "zz", json!(1))),
        entry(json!({"kind": 7})),
        entry(json!({"kind": "ticket"})),
        entry(with(brief(), "schema_version", json!(2))),
    ]));
    assert_eq!(again, validation);
}

// Sources (R12) ----------------------------------------------------------------------------

fn sources(origin: Value, lines: Option<Value>, document: Value) -> Vec<Option<Source>> {
    let mut entry = json!({"format": "document", "document": document, "origin": origin});
    if let Some(lines) = lines {
        entry["lines"] = lines;
    }
    validate(&read(vec![entry]))
        .errors
        .iter()
        .map(|e| e.location().cloned())
        .collect()
}

#[test]
fn the_source_of_an_error_is_the_line_of_its_value_or_of_the_nearest_value_holding_it() {
    let file = |line: Option<u64>| {
        Some(Source::File {
            file: "plan/briefs/BRIEF-0001.yaml".into(),
            line: line.map(self::line),
        })
    };
    let document = with(
        with(brief(), "goals", json!(["", "Export"])),
        "problem",
        Value::Null,
    );
    let origin = json!({"file": "plan/briefs/BRIEF-0001.yaml"});
    // /goals/0 has no line of its own: the line of /goals; /problem is missing: the root.
    assert_eq!(
        sources(
            origin.clone(),
            Some(json!({"": 2, "/goals": 9, "/goals/1": 11})),
            document.clone()
        ),
        [file(Some(9)), file(Some(2))]
    );
    // No line given for the value nor for a value holding it: no line, never a guess.
    assert_eq!(
        sources(origin.clone(), None, document.clone()),
        [file(None), file(None)]
    );
    assert_eq!(
        sources(origin, Some(json!({"/goals/1": 11})), document.clone()),
        [file(None), file(None)]
    );
    let issue = Some(Source::Issue { issue: line(12) });
    assert_eq!(
        sources(json!({"issue": 12}), None, document.clone()),
        [issue.clone(), issue]
    );
    let milestone = Some(Source::Milestone { milestone: line(3) });
    assert_eq!(
        sources(json!({"milestone": 3.0}), None, document),
        [milestone.clone(), milestone]
    );
}

#[test]
fn an_unreadable_entry_is_a_syntax_error_at_the_line_of_the_parser() {
    let validation = validate(&read(vec![
        entry(brief()),
        json!({"format": "unreadable", "origin": {"file": "plan/briefs/BRIEF-0002.yaml"},
               "error": {"line": 4, "message": "found a tab"}}),
    ]));
    let [error] = &validation.errors[..] else {
        panic!("{:?}", validation.errors);
    };
    assert_eq!(*error.code(), codes::ARTIFACT_SCHEMA);
    assert_eq!(error.pointer(), &Pointer::root());
    assert_eq!(error.reason(), Reason::Syntax);
    assert_eq!(
        error.location(),
        Some(&Source::File {
            file: "plan/briefs/BRIEF-0002.yaml".into(),
            line: Some(line(4))
        })
    );
    assert_eq!(error.document(), Some(1));
    assert!(error.message().contains("found a tab"));
}

#[test]
fn an_unreadable_entry_needs_a_file_origin() {
    // Only a file can fail to parse (review R1).
    let input = json!({"documents": [
        {"format": "unreadable", "error": {"line": 1, "message": "empty"}},
        {"format": "unreadable", "origin": {"issue": 5}, "error": {"line": 4, "message": "m"}},
    ]});
    assert_eq!(
        refused_input(&input),
        [
            (
                "/documents/0/origin".into(),
                Reason::Requires,
                Some(vec!["/documents/0/format".into()])
            ),
            (
                "/documents/1/origin".into(),
                Reason::Conflict,
                Some(vec!["/documents/1/format".into()])
            ),
        ]
    );
}

#[test]
fn every_error_of_a_document_carries_its_index_and_errors_of_the_input_none() {
    // SPEC-0004 R2 as amended: the pointer is relative to the document named by `document`.
    let validation = validate(&read(vec![
        entry(brief()),
        entry(json!({"kind": "ticket"})),
        entry(with(brief(), "status", json!("done"))),
    ]));
    let documents: Vec<Option<u64>> = validation.errors.iter().map(Error::document).collect();
    assert_eq!(documents, [Some(1), Some(2)]);
    let envelope = run(
        Command::Validate,
        &json!({"documents": [], "adrs": [0]}),
        &Context::default(),
    );
    let Some(report) = envelope.outcome().report() else {
        panic!("refused expected");
    };
    assert_eq!(report.first().document(), None);
}

// Rules of the input beyond its schema ----------------------------------------------------

fn refused_input(input: &Value) -> Vec<(String, Reason, Option<Vec<String>>)> {
    let schema = input_schema();
    assert_eq!(schema.validate(input, COMMAND_INPUT), [], "{input}");
    let errors = input::rules(input);
    errors
        .iter()
        .inspect(|e| assert_eq!(*e.code(), COMMAND_INPUT))
        .map(|e| {
            (
                e.pointer().to_string(),
                e.reason(),
                e.pointers()
                    .map(|p| p.iter().map(ToString::to_string).collect()),
            )
        })
        .collect()
}

#[test]
fn a_member_of_another_format_is_a_conflict_with_the_format() {
    let input = json!({"documents": [
        {"format": "document", "document": {}, "text": "x", "error": {"line": 1, "message": "m"}},
        {"format": "markdown", "text": "x", "lines": {}, "input_kind": "commit"},
        {"format": "unreadable", "error": {"line": 1, "message": "m"}, "document": {}, "text": null},
    ]});
    let format = |i: usize| Some(vec![format!("/documents/{i}/format")]);
    assert_eq!(
        refused_input(&input),
        [
            ("/documents/0/error".into(), Reason::Conflict, format(0)),
            ("/documents/0/text".into(), Reason::Conflict, format(0)),
            (
                "/documents/1/input_kind".into(),
                Reason::Conflict,
                format(1)
            ),
            ("/documents/1/lines".into(), Reason::Conflict, format(1)),
            ("/documents/2/document".into(), Reason::Conflict, format(2)),
            // An unreadable entry needs a file origin (review R1).
            (
                "/documents/2/origin".into(),
                Reason::Requires,
                Some(vec!["/documents/2/format".into()])
            ),
        ]
    );
}

#[test]
fn an_origin_gives_exactly_one_place() {
    let input = json!({"documents": [
        {"format": "markdown", "text": "x", "origin": {}},
        {"format": "markdown", "text": "x", "origin": {"file": null}},
        {"format": "markdown", "text": "x", "origin": {"issue": 1, "file": "a", "milestone": 2}},
        {"format": "markdown", "text": "x", "origin": {"issue": 1, "file": null}},
    ]});
    assert_eq!(
        refused_input(&input),
        [
            ("/documents/0/origin".into(), Reason::Missing, None),
            ("/documents/1/origin".into(), Reason::Missing, None),
            (
                "/documents/2/origin".into(),
                Reason::Conflict,
                Some(vec![
                    "/documents/2/origin/file".into(),
                    "/documents/2/origin/issue".into(),
                    "/documents/2/origin/milestone".into(),
                ])
            ),
        ]
    );
}

#[test]
fn lines_need_a_file_origin_and_line_numbers_from_1() {
    let input = json!({"documents": [
        {"format": "document", "document": {}, "lines": {"": 1}},
        {"format": "document", "document": {}, "lines": {"": 1}, "origin": {"issue": 3}},
        {"format": "document", "document": {}, "lines": {"": 1}, "origin": {}},
        {"format": "document", "document": {}, "origin": {"file": "a.yaml"},
         "lines": {"x": 1, "/a": "2", "/b": 0, "/c": 2.5, "/d": 9_007_199_254_740_992_u64,
                   "/e": null, "/f": 3.0, "/~2": 1}},
    ]});
    assert_eq!(
        refused_input(&input),
        [
            (
                "/documents/0/origin".into(),
                Reason::Requires,
                Some(vec!["/documents/0/lines".into()])
            ),
            (
                "/documents/1/origin".into(),
                Reason::Conflict,
                Some(vec!["/documents/1/lines".into()])
            ),
            ("/documents/2/origin".into(), Reason::Missing, None),
            ("/documents/3/lines/~1a".into(), Reason::Type, None),
            ("/documents/3/lines/~1b".into(), Reason::Range, None),
            ("/documents/3/lines/~1c".into(), Reason::Type, None),
            ("/documents/3/lines/~1d".into(), Reason::Range, None),
            ("/documents/3/lines/~1~02".into(), Reason::Syntax, None),
            ("/documents/3/lines/x".into(), Reason::Syntax, None),
        ]
    );
}

// The entry point --------------------------------------------------------------------------

#[test]
fn the_entry_point_reports_the_dry_run_of_the_context_and_hashes_the_input() {
    let input = json!({"documents": [entry(brief())]});
    let envelope = run(
        Command::Validate,
        &input,
        &Context::default().with_dry_run(true),
    );
    assert!(envelope.meta().dry_run());
    assert_eq!(envelope.command(), "validate");
    assert_eq!(envelope.meta().command_version().get(), 1);
    assert_eq!(
        envelope.meta().input_hash(),
        &crate::envelope::input_hash(&input)
    );
    let text = serde_json::to_vec(&input).unwrap();
    assert_eq!(
        run_json(
            Command::Validate,
            &text,
            &Context::default().with_dry_run(true)
        ),
        envelope
    );
}

#[test]
fn a_refused_input_runs_no_check_of_the_documents() {
    let input = json!({"documents": [entry(json!({"kind": "ticket"}))], "adrs": [-1]});
    let envelope = run(Command::Validate, &input, &Context::default());
    let Some(report) = envelope.outcome().report() else {
        panic!("refused expected");
    };
    assert_eq!(
        found(&report.errors().cloned().collect::<Vec<_>>()),
        [(COMMAND_INPUT, "/adrs/0".into(), Reason::Range)]
    );
    assert_eq!(envelope.diagnostics().checks(), []);
    let input = json!({"documents": [{"format": "markdown", "text": "x", "origin": {}}]});
    let envelope = run(Command::Validate, &input, &Context::default());
    assert_eq!(envelope.exit_code().code(), 1);
    assert_eq!(envelope.diagnostics().not_done(), []);
}

#[test]
fn an_input_that_is_not_an_object_is_refused_with_type_at_the_root() {
    for input in [json!([]), json!(null), json!("documents")] {
        let envelope = run(Command::Validate, &input, &Context::default());
        let Some(report) = envelope.outcome().report() else {
            panic!("refused expected");
        };
        assert_eq!(report.first().pointer(), &Pointer::root());
        assert_eq!(report.first().reason(), Reason::Type);
    }
}

#[test]
fn identifier_errors_come_after_the_schema_errors_of_their_document() {
    // Review R4: by document, then by stage (schema, then identifier), then in the order of
    // the schema pass, members in the byte order of their keys (`/aa` before `/zz`).
    let document = with(
        with(with(brief(), "id", json!("brief-7")), "zz", json!(1)),
        "aa",
        json!(1),
    );
    let validation = validate(&read(vec![entry(document)]));
    assert_eq!(
        found(&validation.errors),
        [
            (codes::ARTIFACT_SCHEMA, "/aa".into(), Reason::UnknownField),
            (codes::ARTIFACT_SCHEMA, "/zz".into(), Reason::UnknownField),
            (codes::IDENTIFIER_FORMAT, "/id".into(), Reason::Syntax),
        ]
    );
}

#[test]
fn without_a_requested_version_the_current_one_runs_and_is_listed_as_a_default() {
    // ADR 0028 as amended on 2026-11-20.
    let envelope = run(
        Command::Validate,
        &json!({"documents": []}),
        &Context::default(),
    );
    assert_eq!(envelope.exit_code().code(), 0);
    let [default] = envelope.meta().defaults_applied() else {
        panic!("{:?}", envelope.meta().defaults_applied());
    };
    assert_eq!(default.reason().id(), "current_command_version");
    assert_eq!(default.value(), &json!(1));
    assert_eq!(default.pointer(), &Pointer::root());
    let one = core::num::NonZeroU32::MIN;
    let requested = run(
        Command::Validate,
        &json!({"documents": []}),
        &Context::default().with_command_version(one),
    );
    assert_eq!(requested.meta().defaults_applied(), []);
    assert_eq!(requested.meta().command_version(), one);
}

#[test]
fn an_unknown_requested_version_is_refused_with_the_known_versions() {
    let two = core::num::NonZeroU32::new(2).unwrap();
    let context = Context::default().with_command_version(two);
    for envelope in [
        run(Command::Validate, &json!({"documents": []}), &context),
        run_json(Command::Validate, b"{", &context),
    ] {
        let Some(report) = envelope.outcome().report() else {
            panic!("refused expected");
        };
        let error = report.first();
        assert_eq!(*error.code(), COMMAND_INPUT);
        assert_eq!(error.reason(), Reason::Enum);
        assert_eq!(error.pointer(), &Pointer::root());
        assert_eq!(error.expected(), Some(&[json!(1)][..]));
        assert_eq!(report.errors().count(), 1);
        assert_eq!(envelope.meta().command_version().get(), 1);
        assert_eq!(envelope.meta().defaults_applied(), []);
    }
    // Text that is not JSON with no version requested: the default is still listed.
    let envelope = run_json(Command::Validate, b"{", &Context::default());
    assert_eq!(envelope.meta().defaults_applied().len(), 1);
}

#[test]
fn usage_and_unreadable_input_envelopes_carry_the_error_and_their_exit_code() {
    let usage = crate::usage_error("nope", "Unknown command.");
    assert_eq!(usage.command(), "nope");
    assert_eq!(usage.exit_code().code(), 2);
    let Some(report) = usage.outcome().report() else {
        panic!("usage expected");
    };
    assert_eq!(*report.first().code(), crate::USAGE);
    assert_eq!(report.first().reason(), Reason::Syntax);
    assert_eq!(report.first().message(), "Unknown command.");
    assert_eq!(usage.meta().command_version().get(), 1);
    // SHA-256 of no bytes.
    assert_eq!(
        usage.meta().input_hash().as_str(),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(crate::usage_error("validate", "x").command(), "validate");
    for (not_found, reason) in [(true, Reason::Missing), (false, Reason::State)] {
        let envelope = crate::input_unreadable(Command::Validate, "No such file.", not_found);
        assert_eq!(envelope.command(), "validate");
        assert_eq!(envelope.exit_code().code(), 3);
        let Some(report) = envelope.outcome().report() else {
            panic!("environment expected");
        };
        assert_eq!(*report.first().code(), crate::INPUT_UNREADABLE);
        assert_eq!(report.first().reason(), reason);
    }
}

#[test]
fn a_file_origin_outside_the_project_is_refused() {
    // Audit B2: no absolute or escaping path.
    for file in [
        "/etc/passwd",
        "../outside.yaml",
        "plan\\briefs\\a.yaml",
        "C:/x.yaml",
    ] {
        let input = json!({"documents": [
            {"format": "markdown", "text": "x", "origin": {"file": file}}
        ]});
        let envelope = run(Command::Validate, &input, &Context::default());
        let Some(report) = envelope.outcome().report() else {
            panic!("refused expected for {file}");
        };
        assert_eq!(
            found(&report.errors().cloned().collect::<Vec<_>>()),
            [(
                COMMAND_INPUT,
                "/documents/0/origin/file".into(),
                Reason::Syntax
            )],
            "{file}"
        );
    }
}
