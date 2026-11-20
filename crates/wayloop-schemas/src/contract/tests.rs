//! Tests of the generator: the transform, the checks and the helpers of the rules, on the
//! contract types and on test types that break one rule each.
// The test types exist for their schema only; they are never built.
#![allow(dead_code)]

use std::collections::BTreeMap;

use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde_json::{Value, json};

use super::rules::{formatted, require_if_absent, require_if_equals, require_if_present};
use super::transform::{ANY_VALUE, unwrapped};
use super::{
    ContractError, Direction, MAX_OBJECT_LEVELS, PUBLISHED_AT, Problem, TYPES, schema_for, snapshot,
};
use crate::fixtures::at;
use crate::{Format, Pointer};

fn refused<T: JsonSchema>(direction: Direction) -> ContractError {
    schema_for::<T>(direction).unwrap_err()
}

/// Every subschema of `value` that is an object, with its pointer.
fn objects(value: &Value, pointer: &Pointer, out: &mut Vec<(Pointer, Value)>) {
    match value {
        Value::Object(map) => {
            out.push((pointer.clone(), value.clone()));
            for (key, child) in map {
                objects(child, &pointer.join(key), out);
            }
        }
        Value::Array(items) => {
            for (i, child) in items.iter().enumerate() {
                objects(child, &pointer.join_index(i), out);
            }
        }
        _ => {}
    }
}

#[test]
fn every_contract_type_generates_its_schema_in_both_directions() {
    let names: Vec<&str> = TYPES.iter().map(|t| t.name).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(names, sorted, "listed once each, in byte order");
    for registered in TYPES {
        for direction in [Direction::Input, Direction::Output] {
            let schema = (registered.schema)(direction)
                .unwrap_or_else(|e| panic!("{} {direction:?}: {e}", registered.name));
            assert_eq!(schema["title"], registered.name);
            assert_eq!(
                schema["$schema"],
                "https://json-schema.org/draft/2020-12/schema"
            );
        }
    }
}

#[test]
fn objects_are_open_in_an_output_and_closed_in_an_input() {
    for registered in TYPES {
        let output = (registered.schema)(Direction::Output).unwrap();
        let input = (registered.schema)(Direction::Input).unwrap();
        let mut found = Vec::new();
        objects(&output, &Pointer::root(), &mut found);
        for (pointer, schema) in &found {
            assert!(schema.get("additionalProperties").is_none(), "{pointer}");
        }
        let mut found = Vec::new();
        objects(&input, &Pointer::root(), &mut found);
        for (pointer, schema) in found {
            if schema.get("type") == Some(&json!("object")) {
                assert_eq!(
                    schema["additionalProperties"],
                    json!(false),
                    "{} {pointer}",
                    registered.name
                );
            }
        }
    }
}

#[test]
fn a_documented_enum_is_an_enum_with_the_meaning_of_each_value() {
    let schema = schema_for::<crate::Reason>(Direction::Output).unwrap();
    assert!(schema.get("oneOf").is_none());
    assert_eq!(schema["type"], "string");
    let ids: Vec<&str> = crate::Reason::ALL.iter().map(|r| r.id()).collect();
    assert_eq!(schema["enum"], json!(ids));
    let description = schema["description"].as_str().unwrap();
    assert!(
        description.starts_with("Why the engine refused an input, or `internal` when it failed"),
        "{description}"
    );
    assert!(description.contains("\n\n- `syntax`: The input is not well-formed.\n"));
    // The meaning of a value written over two lines of its doc comment is one line.
    assert!(description.ends_with(
        "\n- `internal`: The engine could not compute the result: the input may be valid \
         (ADR 0009 C3, amended on 2026-11-20). The command exits with code 5, not 1 (C7)."
    ));
}

#[test]
fn an_optional_member_has_no_null_and_is_not_required() {
    let schema = schema_for::<crate::Error>(Direction::Output).unwrap();
    let error = &schema;
    assert_eq!(
        error["required"],
        json!(["code", "message", "pointer", "reason"])
    );
    assert_eq!(error["properties"]["pointers"]["type"], "array");
    assert_eq!(
        error["properties"]["source"],
        json!({
            "$ref": "#/$defs/Source",
            "description": "Where the faulty value is stored; absent for a document sent directly."
        })
    );
    // Any JSON value: an object schema with a description, never `true`.
    assert_eq!(
        error["properties"]["expected"]["items"],
        json!({"description": ANY_VALUE})
    );
    assert_eq!(
        error["properties"]["fix"],
        json!({"description": "The closest member for a misspelt key, or a corrected value; \
                               absent when none can be derived."})
    );
}

#[test]
fn integer_formats_are_removed_and_the_bounds_remain() {
    let schema = schema_for::<crate::Meta>(Direction::Output).unwrap();
    assert_eq!(
        schema["properties"]["command_version"],
        json!({
            "description": "Version of the input and output schemas of the command, from 1.",
            "minimum": 1,
            "type": "integer"
        })
    );
    let source = schema_for::<crate::Source>(Direction::Output).unwrap();
    assert!(source["properties"]["line"].get("format").is_none());
    assert_eq!(source["properties"]["line"]["minimum"], 1);
}

#[test]
fn a_flattened_struct_merges_its_members_into_one_object() {
    let schema = schema_for::<crate::ErrorReport>(Direction::Output).unwrap();
    assert!(schema.get("allOf").is_none());
    assert!(schema.get("$ref").is_none());
    let members: Vec<&String> = schema["properties"].as_object().unwrap().keys().collect();
    assert_eq!(
        members,
        [
            "code", "errors", "expected", "fix", "message", "pointer", "pointers", "reason",
            "source"
        ]
    );
    assert_eq!(
        schema["required"],
        json!(["code", "message", "pointer", "reason", "errors"])
    );
    assert_eq!(schema["properties"]["errors"]["minItems"], 1);
}

#[test]
fn the_source_requires_file_and_line_together() {
    let schema = schema_for::<crate::Source>(Direction::Output).unwrap();
    assert_eq!(
        schema["allOf"],
        json!([
            {"if": {"required": ["file"]}, "then": {"required": ["line"]}},
            {"if": {"required": ["line"]}, "then": {"required": ["file"]}}
        ])
    );
    assert!(schema.get("required").is_none());
}

#[test]
fn descriptions_are_unwrapped_into_paragraphs() {
    assert_eq!(unwrapped("one\ntwo\n three"), "one two three");
    assert_eq!(unwrapped("one\ntwo\n\nthree\n"), "one two\n\nthree");
    assert_eq!(unwrapped("\n\none\n\n\n\ntwo"), "one\n\ntwo");
    assert_eq!(unwrapped(""), "");
}

// Test types, one broken rule each.

/// Described.
#[derive(JsonSchema)]
struct Described {
    /// A member.
    a: u32,
}

#[derive(JsonSchema)]
struct NoTypeDescription {
    /// A member.
    a: u32,
}

/// Described.
#[derive(JsonSchema)]
struct NoMemberDescription {
    /// A member.
    a: u32,
    b: u32,
}

/// Described.
#[derive(JsonSchema)]
struct NoDefinitionDescription {
    /// A member.
    a: NoTypeDescription,
}

/// Values.
#[derive(JsonSchema)]
enum Undocumented {
    /// The first.
    First,
    Second,
}

/// Values.
#[derive(JsonSchema)]
enum Unexplained {
    First,
    Second,
}

/// Described.
#[derive(JsonSchema)]
struct HoldsUndocumented {
    /// A member.
    a: Undocumented,
}

/// Described.
#[derive(JsonSchema)]
#[serde(deny_unknown_fields)]
struct Closed {
    /// A member.
    a: u32,
}

/// Level 4.
#[derive(JsonSchema)]
struct Level4 {
    /// A member.
    a: u32,
}

/// Level 3.
#[derive(JsonSchema)]
struct Level3 {
    /// A list of objects: the list adds no level.
    a: Vec<Level4>,
}

/// Level 2.
#[derive(JsonSchema)]
struct Level2 {
    /// A member.
    a: Level3,
}

/// Level 1.
#[derive(JsonSchema)]
struct Level1 {
    /// A member.
    a: Level2,
}

/// Level 0.
#[derive(JsonSchema)]
struct Level0 {
    /// A member.
    a: Level1,
}

/// A task identifier, a value form with a named format.
struct TaskId;

impl JsonSchema for TaskId {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "TaskId".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        formatted(Format::TaskId, "Identifier of a task.")
    }
}

/// Status of an artifact.
#[derive(JsonSchema)]
#[serde(rename_all = "snake_case")]
enum Status {
    /// In force.
    Accepted,
    /// Replaced by the artifact in `superseded_by`.
    Superseded,
}

/// An artifact with a conditional requirement.
#[derive(JsonSchema)]
#[schemars(transform = superseded_requires_replacement)]
struct Artifact {
    /// Identifier of the artifact.
    id: TaskId,
    /// Status of the artifact.
    status: Status,
    /// The artifact that replaces this one.
    #[serde(skip_serializing_if = "Option::is_none")]
    superseded_by: Option<TaskId>,
}

fn superseded_requires_replacement(schema: &mut Schema) {
    require_if_equals(schema, "status", "superseded", &["superseded_by"]);
    require_if_present(schema, "superseded_by", &["status"]);
}

#[test]
fn a_type_a_member_or_a_definition_without_description_is_refused() {
    assert!(schema_for::<Described>(Direction::Output).is_ok());
    for direction in [Direction::Input, Direction::Output] {
        assert_eq!(
            refused::<NoTypeDescription>(direction),
            ContractError {
                pointer: Pointer::root(),
                problem: Problem::MissingDescription
            }
        );
        assert_eq!(
            refused::<NoMemberDescription>(direction),
            ContractError {
                pointer: at("/properties/b"),
                problem: Problem::MissingDescription
            }
        );
        assert_eq!(
            refused::<NoDefinitionDescription>(direction),
            ContractError {
                pointer: at("/$defs/NoTypeDescription"),
                problem: Problem::MissingDescription
            }
        );
    }
}

#[test]
fn an_enum_value_without_its_meaning_is_refused() {
    assert_eq!(
        refused::<Undocumented>(Direction::Output),
        ContractError {
            // The derive groups the values without a description in one entry, first.
            pointer: at("/oneOf/0/enum/0"),
            problem: Problem::UndocumentedValue("Second".into())
        }
    );
    assert_eq!(
        refused::<Unexplained>(Direction::Output),
        ContractError {
            pointer: at("/enum/0"),
            problem: Problem::UndocumentedValue("First".into())
        }
    );
    assert_eq!(
        refused::<HoldsUndocumented>(Direction::Input).pointer,
        at("/$defs/Undocumented/oneOf/0/enum/0")
    );
}

#[test]
fn an_output_closed_by_the_derive_is_refused() {
    assert_eq!(
        refused::<Closed>(Direction::Output),
        ContractError {
            pointer: at("/additionalProperties"),
            problem: Problem::ClosedOutput
        }
    );
    assert!(schema_for::<Closed>(Direction::Input).is_ok());
}

#[test]
fn objects_nest_at_most_four_levels() {
    assert_eq!(MAX_OBJECT_LEVELS, 4);
    assert!(schema_for::<Level1>(Direction::Output).is_ok());
    for direction in [Direction::Input, Direction::Output] {
        assert_eq!(
            refused::<Level0>(direction),
            ContractError {
                pointer: at("/$defs/Level4"),
                problem: Problem::TooDeep
            }
        );
    }
}

#[test]
fn a_named_format_and_a_conditional_requirement_stay_in_the_subset() {
    let schema = schema_for::<Artifact>(Direction::Input).unwrap();
    assert_eq!(
        schema["$defs"]["TaskId"],
        json!({"description": "Identifier of a task.", "type": "string", "format": "task-id"})
    );
    assert_eq!(
        schema["allOf"],
        json!([
            {
                "if": {"properties": {"status": {"const": "superseded"}}, "required": ["status"]},
                "then": {"required": ["superseded_by"]}
            },
            {"if": {"required": ["superseded_by"]}, "then": {"required": ["status"]}}
        ])
    );
    assert_eq!(schema["required"], json!(["id", "status"]));
    assert_eq!(schema["additionalProperties"], json!(false));
}

#[test]
fn each_problem_is_told_in_words_with_its_place() {
    let cases = [
        (
            Pointer::root(),
            Problem::MissingDescription,
            "at the root of the schema: no description",
        ),
        (
            at("/enum/0"),
            Problem::UndocumentedValue("x".into()),
            "at /enum/0: the description gives no meaning for the value x",
        ),
        (
            at("/additionalProperties"),
            Problem::ClosedOutput,
            "at /additionalProperties: `additionalProperties` in an output, which is open",
        ),
        (
            at("/$defs/A"),
            Problem::TooDeep,
            "at /$defs/A: objects nested deeper than 4 levels",
        ),
    ];
    for (pointer, problem, text) in cases {
        let error: Box<dyn std::error::Error> = Box::new(ContractError { pointer, problem });
        assert_eq!(error.to_string(), text);
    }
}

// The transform and the checks on hand-written schemas: forms the derive of the contract types
// does not produce today, which another type or another version of `schemars` could.

/// `schema` after the transform of the generator, in the output direction.
fn transformed(schema: Value) -> Value {
    use schemars::transform::Transform;
    let mut schema = Schema::try_from(schema).unwrap();
    super::transform::Subset {
        direction: Direction::Output,
    }
    .transform(&mut schema);
    schema.to_value()
}

/// An object with the optional member `a` of schema `member`.
fn with_optional(member: &Value) -> Value {
    json!({"description": "d", "type": "object", "properties": {"a": member}})
}

#[test]
fn an_optional_member_loses_its_null_in_either_order_and_only_its_null() {
    let reference = json!({"$ref": "#/$defs/A"});
    // Not named `null`: `json!` reads that name as the JSON null.
    let absent = json!({"type": "null"});
    for choices in [json!([reference, absent]), json!([absent, reference])] {
        assert_eq!(
            transformed(with_optional(
                &json!({"description": "m", "anyOf": choices})
            )),
            with_optional(&json!({"description": "m", "$ref": "#/$defs/A"}))
        );
    }
    // Two choices that are both null, or neither, are not an `Option`: left as they are.
    for choices in [
        json!([absent, absent]),
        json!([{"type": "string"}, {"type": "integer"}]),
    ] {
        let member = json!({"description": "m", "anyOf": choices});
        assert_eq!(transformed(with_optional(&member)), with_optional(&member));
    }
}

#[test]
fn only_the_integer_formats_of_integers_are_removed() {
    for (schema, kept) in [
        (json!({"type": "integer", "format": "int64"}), false),
        (json!({"type": "integer", "format": "uint8"}), false),
        (json!({"type": "integer", "format": "date"}), true),
        (json!({"type": "string", "format": "int-like"}), true),
    ] {
        let after = transformed(schema.clone());
        assert_eq!(after.get("format").is_some(), kept, "{schema}");
        assert_eq!(after["type"], schema["type"]);
    }
}

/// Described.
#[derive(JsonSchema)]
struct BlankDescription {
    #[schemars(description = " \n  ")]
    a: u32,
}

/// Described.
#[derive(JsonSchema)]
struct WithMap {
    /// A map of numbers.
    a: BTreeMap<String, u32>,
}

#[test]
fn a_description_of_blanks_only_is_none() {
    assert_eq!(
        refused::<BlankDescription>(Direction::Output),
        ContractError {
            pointer: at("/properties/a"),
            problem: Problem::MissingDescription
        }
    );
}

#[test]
fn a_map_is_refused_in_an_output_and_kept_for_the_loader_in_an_input() {
    assert_eq!(
        refused::<WithMap>(Direction::Output),
        ContractError {
            pointer: at("/properties/a/additionalProperties"),
            problem: Problem::ClosedOutput
        }
    );
    // In an input the schema of the values stays, never overwritten by `false`: the loader of
    // the core refuses it by its place.
    let schema = schema_for::<WithMap>(Direction::Input).unwrap();
    assert_eq!(
        schema["properties"]["a"]["additionalProperties"],
        json!({"type": "integer", "minimum": 0})
    );
    assert_eq!(schema["additionalProperties"], json!(false));
}

#[test]
fn a_conditional_requirement_never_drops_an_all_of_that_is_not_a_list() {
    let mut schema = Schema::from(serde_json::Map::new());
    schema.insert("allOf".to_owned(), json!({"broken": true}));
    require_if_present(&mut schema, "a", &["b"]);
    assert_eq!(
        schema.get("allOf"),
        Some(&json!([
            {"broken": true},
            {"if": {"required": ["a"]}, "then": {"required": ["b"]}}
        ]))
    );
}

#[test]
fn a_requirement_on_an_absent_member_tests_it_against_null() {
    let mut schema = Schema::from(serde_json::Map::new());
    require_if_absent(&mut schema, "data", &["error"]);
    assert_eq!(
        schema.get("allOf"),
        Some(&json!([
            {"if": {"properties": {"data": {"const": null}}}, "then": {"required": ["error"]}}
        ]))
    );
}

#[test]
fn the_envelope_requires_its_error_when_its_data_is_absent() {
    let schema = schema_for::<crate::Envelope<Value>>(Direction::Output).unwrap();
    assert_eq!(schema["title"], "Envelope");
    assert_eq!(
        schema["required"],
        json!(["command", "diagnostics", "meta"])
    );
    assert_eq!(
        schema["allOf"],
        json!([{"if": {"properties": {"data": {"const": null}}}, "then": {"required": ["error"]}}])
    );
    assert_eq!(schema["properties"]["error"]["$ref"], "#/$defs/ErrorReport");
    // Any data: an object schema with the description of the member.
    assert_eq!(
        schema["properties"]["data"],
        json!({"description": "The result of the command; absent when the command did not succeed."})
    );
}

/// The data of a command in the tests.
#[derive(JsonSchema)]
struct Counted {
    /// Number of documents, from 0.
    documents: u32,
}

#[test]
fn the_envelope_of_a_command_differs_from_the_generic_one_only_by_its_data() {
    let generic = schema_for::<crate::Envelope<Value>>(Direction::Output).unwrap();
    let mut typed = schema_for::<crate::Envelope<Counted>>(Direction::Output).unwrap();
    assert_eq!(
        typed["properties"]["data"],
        json!({
            "$ref": "#/$defs/Counted",
            "description": "The result of the command; absent when the command did not succeed."
        })
    );
    assert_eq!(typed["$defs"]["Counted"]["type"], "object");
    typed["$defs"].as_object_mut().unwrap().remove("Counted");
    typed["properties"]["data"] = generic["properties"]["data"].clone();
    assert_eq!(typed, generic);
}

#[test]
fn a_snapshot_is_the_schema_with_its_published_id_in_one_stable_text() {
    assert_eq!(PUBLISHED_AT, "https://salvo8519.github.io/wayloop/schemas/");
    let schema = json!({"title": "T", "description": "\u{e8} vero.", "$schema": "s"});
    let text = snapshot(schema.clone(), "envelope/v1.json").unwrap();
    assert_eq!(
        text,
        concat!(
            "{\n",
            "  \"$id\": \"https://salvo8519.github.io/wayloop/schemas/envelope/v1.json\",\n",
            "  \"$schema\": \"s\",\n",
            "  \"description\": \"\u{e8} vero.\",\n",
            "  \"title\": \"T\"\n",
            "}\n"
        )
    );
    assert_eq!(snapshot(schema, "envelope/v1.json").unwrap(), text);
    assert_eq!(snapshot(json!(true), "a/v1.json").unwrap(), "true\n");
}

#[test]
fn reread_gives_back_a_document_of_the_type_and_refuses_another() {
    let document = json!({"check": "links", "reason": "input_absent", "later": 1});
    assert_eq!(
        super::reread::<crate::NotDone>(&document).unwrap(),
        json!({"check": "links", "reason": "input_absent"})
    );
    assert!(super::reread::<crate::NotDone>(&json!({"check": "links"})).is_err());
}

/// A schema whose root reaches, through `length` definitions that are lists, a definition
/// holding five levels of objects.
fn chain(length: usize) -> Value {
    let object =
        |inner: Value| json!({"description": "d", "type": "object", "properties": {"a": inner}});
    let deep = object(object(object(object(object(json!({"description": "d"}))))));
    let mut defs = serde_json::Map::new();
    for i in 0..length {
        let next = if i + 1 == length {
            "#/$defs/Deep".to_owned()
        } else {
            format!("#/$defs/D{}", i + 1)
        };
        defs.insert(
            format!("D{i}"),
            json!({"description": "d", "type": "array", "items": {"$ref": next}}),
        );
    }
    defs.insert("Deep".to_owned(), deep);
    json!({"description": "d", "$ref": "#/$defs/D0", "$defs": defs})
}

#[test]
fn the_walk_of_references_stops_after_its_bound_and_leaves_the_rest_to_the_loader() {
    let problem = |length| super::check::check(&chain(length), Direction::Output);
    let deep = ContractError {
        pointer: at("/$defs/Deep/properties/a/properties/a/properties/a/properties/a"),
        problem: Problem::TooDeep,
    };
    assert_eq!(problem(1), Err(deep.clone()));
    // 63 lists then Deep: reached within 64 references.
    assert_eq!(problem(63), Err(deep));
    // 64 lists then Deep: the walk stops at its bound. Such a schema is deeper than the 32
    // levels the loader of the core accepts, and is refused there.
    assert_eq!(problem(64), Ok(()));
}

crate::contract_enum! {
    /// A light.
    pub enum HalfDocumented {
        /// Stop.
        Red = "red",
        Green = "green",
    }
}

// A value whose doc comment is blanks only, which Clippy refuses as an empty doc comment.
#[allow(clippy::empty_docs)]
mod blank {
    crate::contract_enum! {
        /// A light.
        pub enum BlankValue {
            /// Stop.
            Red = "red",
            #[doc = "   "]
            Green = "green",
        }
    }
}
use blank::BlankValue;

#[test]
fn a_contract_enum_value_without_its_meaning_is_refused() {
    for error in [
        refused::<HalfDocumented>(Direction::Output),
        refused::<BlankValue>(Direction::Output),
    ] {
        assert_eq!(
            error,
            ContractError {
                pointer: at("/oneOf/1"),
                problem: Problem::UndocumentedValue("green".into())
            }
        );
    }
}

#[test]
fn an_enum_line_without_a_meaning_is_refused() {
    let schema =
        |description: &str| json!({"description": description, "type": "string", "enum": ["a"]});
    assert_eq!(
        super::check::check(&schema("d\n\n- `a`:   "), Direction::Output),
        Err(ContractError {
            pointer: at("/enum/0"),
            problem: Problem::UndocumentedValue("a".into())
        })
    );
    assert_eq!(
        super::check::check(&schema("d\n\n- `a`: First."), Direction::Output),
        Ok(())
    );
}
