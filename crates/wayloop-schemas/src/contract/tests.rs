//! Tests of the generator: the transform, the checks and the helpers of the rules, on the
//! contract types and on test types that break one rule each.
// The test types exist for their schema only; they are never built.
#![allow(dead_code)]

use std::collections::BTreeMap;

use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde_json::{Value, json};

use super::rules::{
    artifact, formatted, require_if_absent, require_if_contains, require_if_equals, require_if_in,
    require_if_present,
};
use super::transform::{ANY_VALUE, only_reachable_definitions, unwrapped};
use super::{
    ContractError, Direction, MAX_OBJECT_LEVELS, PUBLISHED_AT, Problem, TYPES, schema_for, snapshot,
};
use crate::fixtures::{Counted, at};
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
            // A generic type is registered with its parameter, `Envelope<ValidateData>`; its
            // schema keeps the name of the type, `Envelope`.
            let title = registered.name.split('<').next().unwrap_or_default();
            assert_eq!(schema["title"], title);
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
        // An output is open; a pure map keeps the schema of its values (#307).
        for (pointer, schema) in &found {
            let pure_map = schema
                .get("additionalProperties")
                .is_none_or(Value::is_object)
                && schema.get("properties").is_none();
            assert!(
                schema.get("additionalProperties").is_none() || pure_map,
                "{pointer}"
            );
        }
        let mut found = Vec::new();
        objects(&input, &Pointer::root(), &mut found);
        // An object that declares members is closed; one that declares none is a free object
        // (a document of another schema), open in an input too (ADR 0030 G2, amended with #256).
        for (pointer, schema) in found {
            if schema.get("type") == Some(&json!("object")) && schema.get("properties").is_some() {
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
            "code", "document", "errors", "expected", "fix", "message", "pointer", "pointers",
            "reason", "source"
        ]
    );
    assert_eq!(
        schema["required"],
        json!(["code", "message", "pointer", "reason", "errors"])
    );
    assert_eq!(schema["properties"]["errors"]["minItems"], 1);
}

#[test]
fn the_line_of_a_source_comes_only_with_its_file() {
    // The line is written only when it is known, never guessed (SPEC-0004 R2).
    let schema = schema_for::<crate::Source>(Direction::Output).unwrap();
    assert_eq!(
        schema["allOf"],
        json!([{"if": {"required": ["line"]}, "then": {"required": ["file"]}}])
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
#[schemars(rename_all = "snake_case")]
enum Undocumented {
    /// The first.
    First,
    Second,
}

/// Values.
#[derive(JsonSchema)]
#[schemars(rename_all = "snake_case")]
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
            problem: Problem::UndocumentedValue("second".into())
        }
    );
    assert_eq!(
        refused::<Unexplained>(Direction::Output),
        ContractError {
            pointer: at("/enum/0"),
            problem: Problem::UndocumentedValue("first".into())
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

/// Described.
#[derive(JsonSchema)]
struct MapOfLevel1 {
    /// A map whose values nest three more levels of objects.
    a: BTreeMap<String, Level1>,
}

#[test]
fn the_values_of_a_map_count_in_the_levels_of_objects() {
    // Review of #307: a map adds the level of its values, as a member does.
    for direction in [Direction::Input, Direction::Output] {
        assert_eq!(
            refused::<MapOfLevel1>(direction),
            ContractError {
                pointer: at("/$defs/Level3"),
                problem: Problem::TooDeep
            }
        );
    }
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
            at("/enum/0"),
            Problem::NotSnakeCase("a/b".into()),
            "at /enum/0: the enum value a/b is not snake_case",
        ),
        (
            at("/additionalProperties"),
            Problem::ClosedOutput,
            "at /additionalProperties: `additionalProperties` that closes an output, which is open",
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

/// Described.
#[derive(JsonSchema)]
struct WithFreeMap {
    /// A map of any values.
    a: BTreeMap<String, Value>,
}

/// Any value, behind a name.
#[derive(JsonSchema)]
struct AnyValue(Value);

/// Described.
#[derive(JsonSchema)]
struct WithNamedFreeMap {
    /// A map of any values, through a definition.
    a: BTreeMap<String, AnyValue>,
}

/// Described.
#[derive(JsonSchema)]
struct WithFlattenedMap {
    /// A member.
    a: u32,
    /// Other members, numbers.
    #[serde(flatten)]
    rest: BTreeMap<String, u32>,
}

#[test]
fn a_map_flattened_beside_members_closes_an_output() {
    // Contract audit of #307, S1: the schema of the map would refuse a future member of another
    // type, so the output would be closed (ADR 0009 C5).
    assert_eq!(
        refused::<WithFlattenedMap>(Direction::Output),
        ContractError {
            pointer: at("/additionalProperties"),
            problem: Problem::ClosedOutput
        }
    );
}

#[test]
fn a_typed_map_is_accepted_and_a_map_of_any_value_is_an_open_input() {
    // #307: a map keeps the schema of its values, which checks every member, in both
    // directions; a map whose values may be anything stays open, and an input refuses it.
    for direction in [Direction::Input, Direction::Output] {
        let schema = schema_for::<WithMap>(direction).unwrap();
        assert_eq!(
            schema["properties"]["a"]["additionalProperties"]["type"],
            "integer"
        );
    }
    assert!(schema_for::<WithFreeMap>(Direction::Output).is_ok());
    // A reference is followed: a definition of any value leaves the map open (review of #307).
    for open in [refused::<WithFreeMap>, refused::<WithNamedFreeMap>] {
        assert_eq!(
            open(Direction::Input),
            ContractError {
                pointer: at("/properties/a"),
                problem: Problem::OpenInput
            }
        );
    }
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

/// An artifact whose replacement is required in two statuses.
#[derive(JsonSchema)]
#[schemars(transform = replaced_in_two_statuses)]
struct Started {
    /// Status of the artifact.
    status: Status,
    /// The artifact that replaces this one.
    #[serde(skip_serializing_if = "Option::is_none")]
    superseded_by: Option<TaskId>,
}

fn replaced_in_two_statuses(schema: &mut Schema) {
    require_if_in(
        schema,
        "status",
        &["accepted", "superseded"],
        &["superseded_by"],
    );
}

/// The same artifact, whose condition names a value that is not `snake_case`.
#[derive(JsonSchema)]
#[schemars(transform = replaced_in_a_misspelt_status)]
struct Misspelt {
    /// Status of the artifact.
    status: Status,
    /// The artifact that replaces this one.
    #[serde(skip_serializing_if = "Option::is_none")]
    superseded_by: Option<TaskId>,
}

fn replaced_in_a_misspelt_status(schema: &mut Schema) {
    require_if_in(schema, "status", &["Superseded"], &["superseded_by"]);
}

#[test]
fn a_condition_names_values_documented_where_the_member_is_declared() {
    // The values of an `enum` in an `if` are values of a declared member, described there:
    // the condition needs no meaning of its own, but keeps the spelling of the contracts.
    for direction in [Direction::Input, Direction::Output] {
        let schema = schema_for::<Started>(direction).unwrap();
        assert_eq!(
            schema["allOf"][0]["if"]["properties"]["status"]["enum"],
            json!(["accepted", "superseded"])
        );
        assert_eq!(
            refused::<Misspelt>(direction),
            ContractError {
                pointer: at("/allOf/0/if/properties/status/enum/0"),
                problem: Problem::NotSnakeCase("Superseded".into())
            }
        );
    }
}

#[test]
fn a_requirement_on_one_of_several_values_or_on_an_element_of_a_list() {
    let mut schema = Schema::from(serde_json::Map::new());
    require_if_in(&mut schema, "status", &["implementing", "done"], &["issue"]);
    require_if_contains(&mut schema, "verification", "judge", &["rubric"]);
    assert_eq!(
        schema.get("allOf"),
        Some(&json!([
            {
                "if": {
                    "properties": {"status": {"enum": ["implementing", "done"]}},
                    "required": ["status"]
                },
                "then": {"required": ["issue"]}
            },
            {
                "if": {
                    "properties": {"verification": {"contains": {"const": "judge"}}},
                    "required": ["verification"]
                },
                "then": {"required": ["rubric"]}
            }
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

crate::contract_enum! {
    /// A folder written as a path, which C4 refuses (review of #320).
    pub enum PathValue {
        /// The folder of the briefs.
        Briefs = "plan/briefs",
    }
}

#[test]
fn an_enum_value_that_is_not_snake_case_is_refused() {
    for direction in [Direction::Input, Direction::Output] {
        let error = refused::<PathValue>(direction);
        assert_eq!(error.problem, Problem::NotSnakeCase("plan/briefs".into()));
    }
}

#[test]
fn snake_case_is_lower_case_words_joined_by_one_underscore() {
    for good in ["a", "plan_briefs", "schema_v1", "step_2", "x9"] {
        assert!(super::check::is_snake_case(good), "{good}");
    }
    for bad in [
        "",
        "_a",
        "a_",
        "a__b",
        "Plan",
        "plan/briefs",
        "plan-briefs",
        "1a",
        "a b",
    ] {
        assert!(!super::check::is_snake_case(bad), "{bad}");
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

/// A free object of an input: a document whose schema is another one.
struct Free;

impl JsonSchema for Free {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Free".into()
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> Schema {
        crate::value::free_object_schema("A document of another schema.")
    }
}

/// An input with a free member.
#[derive(JsonSchema)]
#[allow(dead_code, reason = "a schema only, never built")]
struct WithFree {
    /// A free document.
    document: Free,
}

#[test]
fn only_a_declared_free_object_stays_open_in_an_input() {
    // Contract audit S5: an open object of an input is refused unless it is listed in
    // FREE_OBJECTS, as the documents of `validate` are.
    assert_eq!(
        refused::<WithFree>(Direction::Input),
        ContractError {
            pointer: at("/$defs/Free"),
            problem: Problem::OpenInput
        }
    );
    assert_eq!(
        super::check::FREE_OBJECTS,
        [crate::validate::FreeDocument::NAME]
    );
    let schema = schema_for::<crate::validate::ValidateInput>(Direction::Input).unwrap();
    assert_eq!(schema["additionalProperties"], json!(false));
    for name in super::check::FREE_OBJECTS {
        let free = &schema["$defs"][name];
        assert_eq!(free["type"], "object");
        assert!(free.get("additionalProperties").is_none(), "{schema}");
    }
    // The same object in an output is open, as every output is.
    assert!(schema_for::<WithFree>(Direction::Output).is_ok());
    // A condition names members; it is not an object of the input.
    let mut conditional = json!({
        "additionalProperties": false,
        "description": "d",
        "properties": {"a": {"description": "a", "type": "string"}},
        "type": "object",
        "allOf": [{"if": {"type": "object", "required": ["a"]}, "then": {"required": ["a"]}}]
    });
    assert_eq!(super::check::check(&conditional, Direction::Input), Ok(()));
    conditional["additionalProperties"] = json!(true);
    assert_eq!(
        super::check::check(&conditional, Direction::Input),
        Err(ContractError {
            pointer: at(""),
            problem: Problem::OpenInput
        })
    );
}

#[test]
fn the_rules_of_an_artifact_hold_on_a_schema_without_members_and_keep_odd_members() {
    // Review N5: no member to replace is no reason to lose the rule, nor to panic.
    let one = core::num::NonZeroU32::MIN;
    let mut bare = Schema::from(serde_json::Map::new());
    artifact(&mut bare, "brief", one, "superseded");
    assert_eq!(
        bare.get("properties").unwrap()["kind"],
        json!({"const": "brief"})
    );
    assert_eq!(
        bare.get("properties").unwrap()["schema_version"],
        json!({"const": 1, "type": "integer"})
    );
    let mut odd = Schema::try_from(json!({"properties": true})).unwrap();
    artifact(&mut odd, "brief", one, "superseded");
    assert_eq!(odd.get("properties"), Some(&json!(true)));
}

#[test]
fn a_definition_no_reference_reaches_is_removed() {
    // Audit N1: the constant kind of a brief leaves ArtifactKind unused.
    let brief = schema_for::<crate::artifact::Brief>(Direction::Input).unwrap();
    assert!(brief["$defs"].get("ArtifactKind").is_none(), "{brief}");
    assert!(brief["$defs"].get("BriefStatus").is_some());
    let mut object = json!({
        "$ref": "#/$defs/A",
        "items": {"$ref": "#/$defs/B"},
        "$defs": {
            "A": {"$ref": "#/$defs/C"},
            "B": {},
            "C": {"items": [{"$ref": "#/$defs/A"}]},
            "Unused": {"$ref": "#/$defs/Also"},
            "Also": {}
        }
    });
    only_reachable_definitions(object.as_object_mut().unwrap());
    let kept: Vec<&String> = object["$defs"].as_object().unwrap().keys().collect();
    assert_eq!(kept, ["A", "B", "C"]);
    let mut none = json!({"type": "object", "$defs": {"Unused": {}}});
    only_reachable_definitions(none.as_object_mut().unwrap());
    assert_eq!(none, json!({"type": "object"}));
}

/// An object written by hand with members and no `type`.
struct Untyped;

impl JsonSchema for Untyped {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Untyped".into()
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> Schema {
        Schema::try_from(json!({
            "description": "Members without a type.",
            "properties": {"a": {"description": "A member.", "type": "string"}}
        }))
        .unwrap()
    }
}

/// An input holding an untyped object.
#[derive(JsonSchema)]
#[allow(dead_code, reason = "a schema only, never built")]
struct WithUntyped {
    /// The untyped object.
    inner: Untyped,
}

#[test]
fn a_schema_with_members_is_an_object_without_its_type() {
    // Second review: `properties` alone makes an object, closed by the transform in an input
    // and refused by the checks when open; a condition is never closed.
    let schema = schema_for::<WithUntyped>(Direction::Input).unwrap();
    assert_eq!(
        schema["$defs"]["Untyped"]["additionalProperties"],
        json!(false)
    );
    let output = schema_for::<WithUntyped>(Direction::Output).unwrap();
    assert!(
        output["$defs"]["Untyped"]
            .get("additionalProperties")
            .is_none()
    );
    let open = json!({
        "additionalProperties": false,
        "description": "d",
        "properties": {"a": {"description": "a", "properties": {}}},
        "type": "object"
    });
    assert_eq!(
        super::check::check(&open, Direction::Input),
        Err(ContractError {
            pointer: at("/properties/a"),
            problem: Problem::OpenInput
        })
    );
    let entry = &schema_for::<crate::validate::ValidateInput>(Direction::Input).unwrap()["$defs"]["DocumentEntry"]
        ["allOf"];
    for condition in entry.as_array().unwrap() {
        assert!(
            condition["if"].get("additionalProperties").is_none(),
            "{condition}"
        );
        assert!(
            condition["then"].get("additionalProperties").is_none(),
            "{condition}"
        );
    }
}
