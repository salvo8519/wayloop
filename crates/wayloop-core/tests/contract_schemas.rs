//! The schemas derived from the contract types stay inside the subset of the interpreter
//! (ADR 0030 G3), and the types and their schemas agree both ways (G1):
//!
//! - every registered type passes `contract_schema` in both directions, and the oracle accepts
//!   its schema as a valid draft 2020-12 schema;
//! - the serialised value of every contract type is accepted by its schema, by the interpreter
//!   and by the oracle, and reads back into its type, giving the same JSON;
//! - what the subset refuses (`null`, `pattern`, `default`, `uniqueItems`, an unknown format, a
//!   boolean schema, a map) is refused by the loader, at its place.
// A test stops at the line that failed: the restriction lints of the shipped code do not apply.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    dead_code
)]

use core::num::{NonZeroU32, NonZeroU64};
use std::collections::{BTreeMap, BTreeSet};

use schemars::{JsonSchema, Schema as Derived, SchemaGenerator};
use serde_json::{Value, json};
use wayloop_core::schema::{ContractFault, LoadError, Schema, contract_schema};
use wayloop_core::{
    Check, Code, DefaultApplied, DefaultReason, Diagnostics, Error, ErrorReport, InputHash, Meta,
    NotDone, NotDoneReason, Pointer, Reason, Source,
};
use wayloop_schemas::contract::{ContractError, Direction, Registered, TYPES, reread, schema_for};

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
        file: "specs/0001-artifact-model/spec.md".into(),
        line: number(42),
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
            file: "plan/items/TASK-0001.yaml".into(),
            line: number(7),
        },
        Source::Issue { issue: number(252) },
        Source::Milestone {
            milestone: number(3),
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
fn the_schemas_refuse_what_the_types_never_write() {
    let check = |name: &str, direction: Direction, document: Value| {
        let schema = contract_schema(named(name), direction).unwrap();
        let found = Schema::load(&schema)
            .unwrap()
            .validate(&document, Code::new("contract"));
        found
            .iter()
            .map(|e| (e.pointer().to_string(), e.reason()))
            .collect::<Vec<_>>()
    };
    let output = Direction::Output;
    assert_eq!(
        check("Source", output, json!({"file": "a.md"})),
        [("/line".to_owned(), Reason::Requires)]
    );
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
struct RequiredNull {
    /// A member that serialises as null when not given.
    a: Option<u32>,
}

/// Described.
#[derive(JsonSchema)]
struct WithPattern {
    /// A member.
    #[schemars(regex(pattern = "^a$"))]
    a: String,
}

/// Described.
#[derive(JsonSchema)]
struct WithDefault {
    /// A member.
    #[schemars(extend("default" = 1))]
    a: u32,
}

/// Described.
#[derive(JsonSchema)]
struct WithSet {
    /// A member.
    a: BTreeSet<String>,
}

/// Described.
#[derive(JsonSchema)]
struct WithUnknownFormat {
    /// A member.
    #[schemars(extend("format" = "email"))]
    a: String,
}

/// Described.
#[derive(JsonSchema)]
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
fn a_map_in_an_input_is_refused_by_the_loader_and_in_an_output_by_the_rules() {
    let (pointer, problem) = invalid(refused::<WithMap>(Direction::Input));
    assert_eq!(pointer, "/properties/a/additionalProperties");
    assert_eq!(problem, "`additionalProperties` is only `false`");
    let output = contract_schema(registered::<WithMap>(), Direction::Output).unwrap_err();
    assert!(matches!(output, ContractFault::Rule(_)), "{output}");
    assert!(output.to_string().starts_with("a rule of the contracts: "));
    let subset = ContractFault::Subset(refused::<WithMap>(Direction::Input));
    assert!(
        subset
            .to_string()
            .starts_with("the subset of the interpreter: ")
    );
}
