//! Tests of the generator: the transform, the checks and the helpers of the rules, on the
//! contract types and on test types that break one rule each.
// The test types exist for their schema only; they are never built.
#![allow(dead_code)]

use std::collections::BTreeSet;

use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde_json::{Value, json};

use super::rules::{formatted, require_if_equals, require_if_present};
use super::transform::{ANY_VALUE, unwrapped};
use super::{ContractError, Direction, MAX_OBJECT_LEVELS, Problem, TYPES, schema_for};
use crate::{Format, Pointer};

fn at(pointer: &str) -> Pointer {
    Pointer::parse(pointer).unwrap()
}

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
fn the_source_requires_a_line_beside_a_file() {
    let schema = schema_for::<crate::Source>(Direction::Output).unwrap();
    assert_eq!(
        schema["allOf"],
        json!([{"if": {"required": ["file"]}, "then": {"required": ["line"]}}])
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
fn a_null_type_is_refused() {
    // In an output, an `Option` without skip_serializing_if is required and written as null.
    assert_eq!(
        refused::<RequiredNull>(Direction::Output),
        ContractError {
            pointer: at("/properties/a/type"),
            problem: Problem::Null
        }
    );
}

#[test]
fn pattern_default_and_unique_items_are_refused() {
    for (error, keyword) in [
        (refused::<WithPattern>(Direction::Input), "pattern"),
        (refused::<WithDefault>(Direction::Output), "default"),
        (refused::<WithSet>(Direction::Output), "uniqueItems"),
    ] {
        assert_eq!(
            error,
            ContractError {
                pointer: at(&format!("/properties/a/{keyword}")),
                problem: Problem::Refused(keyword)
            }
        );
    }
}

#[test]
fn a_format_that_is_not_named_is_refused() {
    assert_eq!(
        refused::<WithUnknownFormat>(Direction::Output),
        ContractError {
            pointer: at("/properties/a/format"),
            problem: Problem::UnknownFormat("email".into())
        }
    );
}

#[test]
fn an_output_closed_by_the_derive_is_refused() {
    assert_eq!(
        refused::<Closed>(Direction::Output),
        ContractError {
            pointer: at("/additionalProperties"),
            problem: Problem::Refused("additionalProperties")
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
fn a_boolean_schema_is_refused() {
    /// Described.
    struct Never;
    impl JsonSchema for Never {
        fn schema_name() -> std::borrow::Cow<'static, str> {
            "Never".into()
        }
        fn json_schema(_: &mut SchemaGenerator) -> Schema {
            false.into()
        }
    }
    /// Described.
    #[derive(JsonSchema)]
    struct HoldsNever {
        /// A list of nothing.
        a: Vec<Never>,
    }
    assert_eq!(
        refused::<HoldsNever>(Direction::Output),
        ContractError {
            pointer: at("/$defs/Never"),
            problem: Problem::Boolean
        }
    );
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
            at("/type"),
            Problem::Null,
            "at /type: a `null` type; absent means not given",
        ),
        (
            at("/pattern"),
            Problem::Refused("pattern"),
            "at /pattern: the keyword `pattern` is not used in contracts",
        ),
        (
            at("/items"),
            Problem::Boolean,
            "at /items: a boolean schema",
        ),
        (
            at("/format"),
            Problem::UnknownFormat("email".into()),
            "at /format: `email` is not a named format",
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

/// The verdict of the checks of the generator on `schema`, in the output direction.
fn checked(schema: &Value) -> Result<(), ContractError> {
    super::check::check(schema, Direction::Output)
}

#[test]
fn a_null_type_alone_or_in_a_list_is_refused_and_other_lists_are_left_to_the_core() {
    for kind in [json!("null"), json!(["string", "null"])] {
        assert_eq!(
            checked(&json!({"description": "d", "type": kind})),
            Err(ContractError {
                pointer: at("/type"),
                problem: Problem::Null
            })
        );
    }
    // A list of types without null is outside the subset; the loader of the core refuses it.
    assert_eq!(
        checked(&json!({"description": "d", "type": ["string", "integer"]})),
        Ok(())
    );
    assert_eq!(
        checked(&json!({"description": "d", "type": "string"})),
        Ok(())
    );
}
