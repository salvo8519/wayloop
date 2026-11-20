//! The schemas derived from the contract types stay inside the subset of the interpreter
//! (ADR 0030 G3): every one is loaded by `Schema::load` and is a valid draft 2020-12 schema for
//! the oracle; the serialised value of every contract type is accepted by its schema, by the
//! interpreter and by the oracle.
// A test stops at the line that failed: the restriction lints of the shipped code do not apply.
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

use core::num::{NonZeroU32, NonZeroU64};

use serde_json::{Value, json};
use wayloop_core::schema::Schema;
use wayloop_core::{
    Check, Code, DefaultApplied, DefaultReason, Diagnostics, Error, ErrorReport, InputHash, Meta,
    NotDone, NotDoneReason, Pointer, Reason, Source,
};
use wayloop_schemas::contract::{ContractError, Direction, TYPES, schema_for};

const DIRECTIONS: [Direction; 2] = [Direction::Input, Direction::Output];

fn at(pointer: &str) -> Pointer {
    Pointer::parse(pointer).unwrap()
}

fn number(n: u64) -> NonZeroU64 {
    NonZeroU64::new(n).unwrap()
}

#[test]
fn every_contract_type_has_a_schema_the_core_loads_and_the_oracle_accepts() {
    for registered in TYPES {
        for direction in DIRECTIONS {
            let schema = (registered.schema)(direction).unwrap();
            if let Err(error) = Schema::load(&schema) {
                panic!("{} {direction:?}: {error}", registered.name);
            }
            assert!(
                jsonschema::meta::is_valid(&schema),
                "{} {direction:?}",
                registered.name
            );
        }
    }
}

type Generate = fn(Direction) -> Result<Value, ContractError>;

/// Checks that `document`, a serialised value of a contract type, is accepted by the schema of
/// that type in both directions, by the interpreter of the core and by the oracle.
fn accepted(document: &Value, schema: Generate) {
    for direction in DIRECTIONS {
        let schema = schema(direction).unwrap();
        let errors = Schema::load(&schema)
            .unwrap()
            .validate(document, Code::new("contract"));
        assert_eq!(errors, [], "{direction:?}: {document}");
        let oracle = jsonschema::draft202012::new(&schema).unwrap();
        assert!(oracle.is_valid(document), "{direction:?}: {document}");
    }
}

/// `accepted` for the serialised `value` of the contract type `T`.
macro_rules! accept {
    ($t:ty, $value:expr) => {
        accepted(&serde_json::to_value($value).unwrap(), schema_for::<$t>)
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
fn the_serialised_value_of_every_contract_type_is_accepted_by_its_schema() {
    for reason in Reason::ALL {
        accept!(Reason, reason);
    }
    for reason in NotDoneReason::ALL {
        accept!(NotDoneReason, reason);
    }
    accept!(Code, Code::new("invalid_input"));
    accept!(Check, Check::new("schema"));
    accept!(DefaultReason, DefaultReason::new("project_language"));
    accept!(Pointer, at("/a~1b/0"));
    accept!(InputHash, InputHash::from_digest(&[0xab; 32]));
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
        accept!(Source, source);
    }
    accept!(Error, minimal_error());
    accept!(Error, full_error());
    accept!(Error, minimal_error().with_expected(Vec::<Value>::new()));
    accept!(
        ErrorReport,
        ErrorReport::from_errors([full_error()]).unwrap()
    );
    accept!(
        ErrorReport,
        ErrorReport::from_errors([minimal_error(), full_error()]).unwrap()
    );
    let not_done = NotDone::new(Check::new("links"), NotDoneReason::InputAbsent);
    accept!(NotDone, not_done);
    accept!(Diagnostics, Diagnostics::default());
    accept!(
        Diagnostics,
        Diagnostics::new(
            vec![full_error()],
            vec![not_done],
            vec![Check::new("schema")]
        )
    );
    let applied = DefaultApplied::new(at("/language"), "en", DefaultReason::new("project"))
        .with_message("The language of the project applies.");
    accept!(DefaultApplied, &applied);
    accept!(
        DefaultApplied,
        DefaultApplied::new(Pointer::root(), json!({}), DefaultReason::new("x"))
    );
    accept!(
        Meta,
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
    let check = |schema: &Value, document: Value| {
        let found = Schema::load(schema)
            .unwrap()
            .validate(&document, Code::new("contract"));
        found
            .iter()
            .map(|e| (e.pointer().to_string(), e.reason()))
            .collect::<Vec<_>>()
    };
    let source = schema_for::<Source>(Direction::Output).unwrap();
    assert_eq!(
        check(&source, json!({"file": "a.md"})),
        [("/line".to_owned(), Reason::Requires)]
    );
    assert_eq!(
        check(&source, json!({"issue": 0})),
        [("/issue".to_owned(), Reason::Range)]
    );
    let reason = schema_for::<Reason>(Direction::Output).unwrap();
    assert_eq!(
        check(&reason, json!("other")),
        [(String::new(), Reason::Enum)]
    );
    let hash = schema_for::<InputHash>(Direction::Output).unwrap();
    assert_eq!(check(&hash, json!("ab")), [(String::new(), Reason::Length)]);
    let report = schema_for::<ErrorReport>(Direction::Output).unwrap();
    let mut document = serde_json::to_value(ErrorReport::from_errors([minimal_error()])).unwrap();
    document["errors"] = json!([]);
    assert_eq!(
        check(&report, document.clone()),
        [("/errors".to_owned(), Reason::Length)]
    );
    // An output is open, an input closed.
    let mut open = serde_json::to_value(minimal_error()).unwrap();
    open["later"] = json!(true);
    let error_out = schema_for::<Error>(Direction::Output).unwrap();
    let error_in = schema_for::<Error>(Direction::Input).unwrap();
    assert_eq!(check(&error_out, open.clone()), []);
    assert_eq!(
        check(&error_in, open),
        [("/later".to_owned(), Reason::UnknownField)]
    );
}
