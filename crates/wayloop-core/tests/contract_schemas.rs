//! The schemas derived from the contract types stay inside the subset of the interpreter
//! (ADR 0030 G3), and the types and their schemas agree both ways (G1):
//!
//! - every registered type passes `contract_schema` in both directions, and the oracle accepts
//!   its schema as a valid draft 2020-12 schema;
//! - the serialised value of every contract type is accepted by its schema, by the interpreter
//!   and by the oracle, and reads back into its type, giving the same JSON; the envelope in
//!   each of its outcomes, with diagnostics and defaults;
//! - what the subset refuses (`null`, `pattern`, `default`, `uniqueItems`, an unknown format, a
//!   boolean schema, a map) is refused by the loader, at its place.
// A test stops at the line that failed: the restriction lints of the shipped code do not apply.
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

use core::num::{NonZeroU32, NonZeroU64};
use std::collections::{BTreeMap, BTreeSet};

use schemars::{JsonSchema, Schema as Derived, SchemaGenerator};
use serde_json::{Value, json};
use wayloop_core::schema::{ContractFault, LoadError, Schema, contract_schema};
use wayloop_core::{
    Check, Code, DefaultApplied, DefaultReason, Diagnostics, Envelope, Error, ErrorReport,
    InputHash, Meta, NotDone, NotDoneReason, Outcome, Pointer, Reason, Source,
};
use wayloop_schemas::contract::{
    ContractError, Direction, EMBEDDED, PUBLISHED_AT, Registered, TYPES, reread, schema_for,
};
use wayloop_schemas::value::{IssueNumber, Line, MilestoneNumber, ProjectPath};

const DIRECTIONS: [Direction; 2] = [Direction::Input, Direction::Output];

fn at(pointer: &str) -> Pointer {
    Pointer::parse(pointer).unwrap()
}

fn number(n: u64) -> NonZeroU64 {
    NonZeroU64::new(n).unwrap()
}

/// The registration of the test type `T`.
fn registered<T: JsonSchema>() -> Registered {
    Registered {
        name: "Test",
        schema: schema_for::<T> as fn(Direction) -> Result<Value, ContractError>,
        // A test type is never read: its documents are read as JSON values.
        reread: reread::<Value>,
    }
}

#[test]
fn every_contract_type_passes_contract_schema_and_the_oracle() {
    for registered in TYPES {
        for direction in DIRECTIONS {
            let schema = contract_schema(registered, direction)
                .unwrap_or_else(|e| panic!("{} {direction:?}: {e}", registered.name));
            assert!(
                jsonschema::meta::is_valid(&schema),
                "{} {direction:?}",
                registered.name
            );
        }
    }
}

/// The registration of the contract type called `name`.
fn named(name: &str) -> Registered {
    TYPES
        .into_iter()
        .find(|r| r.name == name)
        .unwrap_or_else(|| panic!("{name} is not registered"))
}

/// Checks that `document`, a serialised value of the contract type `name`, is accepted by its
/// schema in both directions, by the interpreter and by the oracle, and that it reads back
/// into the type giving the same JSON.
fn round_trip(name: &str, document: &Value) {
    let registered = named(name);
    for direction in DIRECTIONS {
        let schema = contract_schema(registered, direction).unwrap();
        let errors = Schema::load(&schema)
            .unwrap()
            .validate(document, Code::new("contract"));
        assert_eq!(errors, [], "{name} {direction:?}: {document}");
        let oracle = jsonschema::draft202012::new(&schema).unwrap();
        assert!(
            oracle.is_valid(document),
            "{name} {direction:?}: {document}"
        );
    }
    let again = (registered.reread)(document)
        .unwrap_or_else(|e| panic!("{name}: {document} does not read back: {e}"));
    assert_eq!(&again, document, "{name}");
}

/// `round_trip` for the serialised `value` of the contract type `name`.
macro_rules! accept {
    ($name:literal, $value:expr) => {
        round_trip($name, &serde_json::to_value($value).unwrap())
    };
}

fn full_error() -> Error {
    Error::new(
        Code::new("delta_conflict"),
        "Two operations change R2.",
        at("/operations/1"),
        Reason::Conflict,
    )
    .with_pointers([at("/operations/0"), at("/operations/1")])
    .with_expected([json!(1), json!("two"), json!({"a": [null]})])
    .with_fix(json!({"op": "removed"}))
    .with_location(Source::File {
        file: ProjectPath::new("specs/0001-artifact-model/spec.md"),
        line: Some(Line::new(number(42))),
    })
}

fn minimal_error() -> Error {
    Error::new(
        Code::new("invalid_input"),
        "No problem.",
        Pointer::root(),
        Reason::Missing,
    )
}

#[test]
fn every_serialised_value_is_accepted_and_reads_back_into_its_type() {
    for reason in Reason::ALL {
        accept!("Reason", reason);
    }
    for reason in NotDoneReason::ALL {
        accept!("NotDoneReason", reason);
    }
    accept!("Code", Code::new("invalid_input"));
    accept!("Check", Check::new("schema"));
    accept!("DefaultReason", DefaultReason::new("project_language"));
    accept!("Pointer", at("/a~1b/0"));
    accept!("InputHash", InputHash::from_digest(&[0xab; 32]));
    for source in [
        Source::File {
            file: ProjectPath::new("plan/items/TASK-0001.yaml"),
            line: Some(Line::new(number(7))),
        },
        Source::Issue {
            issue: IssueNumber::new(number(252)),
        },
        Source::Milestone {
            milestone: MilestoneNumber::new(number(3)),
        },
    ] {
        accept!("Source", source);
    }
    accept!("Error", minimal_error());
    accept!("Error", full_error());
    accept!("Error", minimal_error().with_expected(Vec::<Value>::new()));
    accept!(
        "ErrorReport",
        ErrorReport::from_errors([full_error()]).unwrap()
    );
    accept!(
        "ErrorReport",
        ErrorReport::from_errors([minimal_error(), full_error()]).unwrap()
    );
    let not_done = NotDone::new(Check::new("links"), NotDoneReason::InputAbsent);
    accept!("NotDone", &not_done);
    accept!("Diagnostics", Diagnostics::default());
    accept!(
        "Diagnostics",
        Diagnostics::new(
            vec![full_error()],
            vec![not_done],
            vec![Check::new("schema")]
        )
    );
    let applied = DefaultApplied::new(at("/language"), "en", DefaultReason::new("project"))
        .unwrap()
        .with_message("The language of the project applies.");
    accept!("DefaultApplied", &applied);
    accept!(
        "DefaultApplied",
        DefaultApplied::new(Pointer::root(), json!({}), DefaultReason::new("x")).unwrap()
    );
    accept!(
        "Meta",
        Meta::new(
            "0.0.0",
            NonZeroU32::MIN,
            InputHash::from_digest(&[0; 32]),
            true,
            vec![applied],
        )
    );
}

#[test]
fn every_outcome_of_the_envelope_is_accepted_and_reads_back() {
    // SPEC-0004 R18: a success has data and no error; a refusal or a failure has error and no
    // data. The hand-written serialiser writes what the generated schema accepts.
    let applied = DefaultApplied::new(at("/language"), "en", DefaultReason::new("project"))
        .unwrap()
        .with_message("The language of the project applies.");
    let diagnostics = Diagnostics::new(
        vec![full_error()],
        vec![NotDone::new(Check::new("links"), NotDoneReason::NotBuilt)],
        vec![Check::new("schema")],
    );
    let meta = Meta::new(
        "0.0.0",
        NonZeroU32::MIN,
        InputHash::from_digest(&[7; 32]),
        false,
        vec![applied],
    );
    let failure = Error::new(
        Code::new("internal_error"),
        "The engine could not compute the result.",
        Pointer::root(),
        Reason::Internal,
    );
    for outcome in [
        Outcome::success(json!({"documents": [{"kind": "brief"}]})),
        Outcome::success(json!([])),
        Outcome::refused(ErrorReport::from_errors([minimal_error(), full_error()]).unwrap()),
        Outcome::failed(ErrorReport::from_errors([failure]).unwrap()),
    ] {
        accept!(
            "Envelope",
            Envelope::new(
                "validate",
                outcome.unwrap(),
                diagnostics.clone(),
                meta.clone()
            )
        );
    }
    accept!(
        "Envelope",
        Envelope::new(
            "validate",
            Outcome::success(json!(1)).unwrap(),
            Diagnostics::default(),
            meta
        )
    );
}

/// The serialised envelope of a success with empty data, diagnostics and defaults.
fn empty_success() -> Value {
    serde_json::to_value(Envelope::new(
        "validate",
        Outcome::success(json!({})).unwrap(),
        Diagnostics::default(),
        Meta::new(
            "0.0.0",
            NonZeroU32::MIN,
            InputHash::from_digest(&[0; 32]),
            false,
            vec![],
        ),
    ))
    .unwrap()
}

#[test]
fn every_embedded_snapshot_of_every_version_loads_in_the_core_and_the_oracle() {
    // ADR 0030 G3: the committed snapshots, embedded in wayloop-schemas, are schemas the
    // interpreter loads and valid draft 2020-12 schemas, each with its published `$id`.
    assert_ne!(EMBEDDED.len(), 0);
    for embedded in EMBEDDED {
        let read: Value = serde_json::from_str(embedded.text).unwrap();
        assert_eq!(
            read["$id"],
            format!("{PUBLISHED_AT}{}", embedded.path),
            "{}",
            embedded.path
        );
        Schema::load(&read).unwrap_or_else(|e| panic!("{}: {e}", embedded.path));
        assert!(jsonschema::meta::is_valid(&read), "{}", embedded.path);
    }
}

#[test]
fn the_envelope_snapshot_is_the_schema_of_the_envelope_and_accepts_an_envelope() {
    // The current version of the envelope; version 1 is frozen (ADR 0030 G5).
    let path = format!("envelope/result/v{}.json", wayloop_core::ENVELOPE_VERSION);
    let embedded = EMBEDDED.iter().find(|e| e.path == path).unwrap();
    let mut read: Value = serde_json::from_str(embedded.text).unwrap();
    read.as_object_mut().unwrap().remove("$id");
    assert_eq!(
        read,
        contract_schema(named("Envelope"), Direction::Output).unwrap()
    );
    let oracle = jsonschema::draft202012::new(&read).unwrap();
    assert!(oracle.is_valid(&empty_success()));
    let errors = Schema::load(&read)
        .unwrap()
        .validate(&empty_success(), Code::new("contract"));
    assert_eq!(errors, []);
}

/// The pointer and the reason of every error of `document` against the schema of the contract
/// type `name`, for `direction`, as the interpreter reports them.
fn errors_of(name: &str, direction: Direction, document: &Value) -> Vec<(String, Reason)> {
    let schema = contract_schema(named(name), direction).unwrap();
    let found = Schema::load(&schema)
        .unwrap()
        .validate(document, Code::new("contract"));
    let oracle = jsonschema::draft202012::new(&schema).unwrap();
    assert_eq!(
        oracle.is_valid(document),
        found.is_empty(),
        "{name}: {document}"
    );
    found
        .iter()
        .map(|e| (e.pointer().to_string(), e.reason()))
        .collect()
}

#[test]
fn the_envelope_schema_requires_the_error_when_the_data_is_absent() {
    let success = empty_success();
    let mut neither = success.clone();
    neither.as_object_mut().unwrap().remove("data");
    // Null means not given (ADR 0009 C2), for the interpreter and for the oracle alike.
    let mut null = success.clone();
    null["data"] = Value::Null;
    for document in [neither, null] {
        assert_eq!(
            errors_of("Envelope", Direction::Output, &document),
            [("/error".to_owned(), Reason::Requires)]
        );
        assert!((named("Envelope").reread)(&document).is_err(), "{document}");
    }
    // Both members: the subset has no negation, so only the type refuses it.
    let mut both = success;
    both["error"] = serde_json::to_value(ErrorReport::from_errors([minimal_error()])).unwrap();
    assert_eq!(errors_of("Envelope", Direction::Output, &both), []);
    assert!((named("Envelope").reread)(&both).is_err());
}

#[test]
fn the_schemas_refuse_what_the_types_never_write() {
    let check =
        |name: &str, direction: Direction, document: Value| errors_of(name, direction, &document);
    let output = Direction::Output;
    // The line of a file is written only when it is known (SPEC-0004 R2).
    assert_eq!(check("Source", output, json!({"file": "a.md"})), []);
    assert_eq!(
        check("Source", output, json!({"line": 2})),
        [("/file".to_owned(), Reason::Requires)]
    );
    assert_eq!(
        check("Source", output, json!({"issue": 0})),
        [("/issue".to_owned(), Reason::Range)]
    );
    assert_eq!(
        check("Reason", output, json!("other")),
        [(String::new(), Reason::Enum)]
    );
    assert_eq!(
        check("InputHash", output, json!("ab")),
        [(String::new(), Reason::Length)]
    );
    let mut report = serde_json::to_value(ErrorReport::from_errors([minimal_error()])).unwrap();
    report["errors"] = json!([]);
    assert_eq!(
        check("ErrorReport", output, report),
        [("/errors".to_owned(), Reason::Length)]
    );
    // An output is open, an input closed.
    let mut open = serde_json::to_value(minimal_error()).unwrap();
    open["later"] = json!(true);
    assert_eq!(check("Error", output, open.clone()), []);
    assert_eq!(
        check("Error", Direction::Input, open),
        [("/later".to_owned(), Reason::UnknownField)]
    );
}

// What the subset refuses, refused by the loader at its place: one test type per rule.

/// Described.
#[derive(JsonSchema)]
#[expect(
    dead_code,
    reason = "a test type exists for its schema only and is never built"
)]
struct RequiredNull {
    /// A member that serialises as null when not given.
    a: Option<u32>,
}

/// Described.
#[derive(JsonSchema)]
#[expect(
    dead_code,
    reason = "a test type exists for its schema only and is never built"
)]
struct WithPattern {
    /// A member.
    #[schemars(regex(pattern = "^a$"))]
    a: String,
}

/// Described.
#[derive(JsonSchema)]
#[expect(
    dead_code,
    reason = "a test type exists for its schema only and is never built"
)]
struct WithDefault {
    /// A member.
    #[schemars(extend("default" = 1))]
    a: u32,
}

/// Described.
#[derive(JsonSchema)]
#[expect(
    dead_code,
    reason = "a test type exists for its schema only and is never built"
)]
struct WithSet {
    /// A member.
    a: BTreeSet<String>,
}

/// Described.
#[derive(JsonSchema)]
#[expect(
    dead_code,
    reason = "a test type exists for its schema only and is never built"
)]
struct WithUnknownFormat {
    /// A member.
    #[schemars(extend("format" = "email"))]
    a: String,
}

/// Described.
#[derive(JsonSchema)]
#[expect(
    dead_code,
    reason = "a test type exists for its schema only and is never built"
)]
struct WithMap {
    /// A map of numbers.
    a: BTreeMap<String, u32>,
}

/// Nothing: its schema is the boolean `false`.
struct Never;

impl JsonSchema for Never {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Never".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Derived {
        false.into()
    }
}

/// Described.
#[derive(JsonSchema)]
#[expect(
    dead_code,
    reason = "a test type exists for its schema only and is never built"
)]
struct HoldsNever {
    /// A list of nothing.
    a: Vec<Never>,
}

/// The refusal of the loader for the schema of `T`.
fn refused<T: JsonSchema>(direction: Direction) -> LoadError {
    match contract_schema(registered::<T>(), direction) {
        Err(ContractFault::Subset(error)) => error,
        other => panic!("expected a refusal of the loader, got {other:?}"),
    }
}

/// The pointer and the problem of an `Invalid` refusal.
fn invalid(error: LoadError) -> (String, &'static str) {
    match error {
        LoadError::Invalid { pointer, problem } => (pointer.to_string(), problem),
        other => panic!("expected an invalid keyword, got {other:?}"),
    }
}

#[test]
fn a_null_type_is_refused_by_the_loader() {
    // An `Option` without skip_serializing_if is required in an output and written as null.
    let (pointer, problem) = invalid(refused::<RequiredNull>(Direction::Output));
    assert_eq!(pointer, "/properties/a/type");
    assert!(
        problem.contains("boolean, integer, string, array, object"),
        "{problem}"
    );
}

#[test]
fn pattern_default_and_unique_items_are_refused_by_the_loader() {
    for (error, keyword) in [
        (refused::<WithPattern>(Direction::Input), "pattern"),
        (refused::<WithDefault>(Direction::Output), "default"),
        (refused::<WithSet>(Direction::Output), "uniqueItems"),
    ] {
        assert_eq!(
            error,
            LoadError::Unsupported {
                pointer: at("/properties/a"),
                keyword: keyword.to_owned()
            }
        );
    }
}

#[test]
fn a_format_that_is_not_named_is_refused_by_the_loader() {
    let (pointer, problem) = invalid(refused::<WithUnknownFormat>(Direction::Output));
    assert_eq!(pointer, "/properties/a/format");
    assert!(problem.contains("named formats"), "{problem}");
}

#[test]
fn a_boolean_schema_is_refused_by_the_loader() {
    let (pointer, problem) = invalid(refused::<HoldsNever>(Direction::Output));
    assert_eq!(pointer, "/$defs/Never");
    assert_eq!(problem, "a schema is an object");
}

#[test]
fn a_map_is_refused_by_the_rules_in_both_directions() {
    // An input map is an open object, refused before the loader sees it (contract audit S5).
    for direction in [Direction::Input, Direction::Output] {
        let fault = contract_schema(registered::<WithMap>(), direction).unwrap_err();
        assert!(matches!(fault, ContractFault::Rule(_)), "{fault}");
        assert!(fault.to_string().starts_with("a rule of the contracts: "));
    }
    let subset = ContractFault::Subset(refused::<RequiredNull>(Direction::Output));
    assert!(
        subset
            .to_string()
            .starts_with("the subset of the interpreter: ")
    );
}
