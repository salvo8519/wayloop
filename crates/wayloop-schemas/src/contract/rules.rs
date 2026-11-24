//! Rules the derive does not write, for the contract types (ADR 0030 G2).
//!
//! A conditional requirement is written by a function of the type, named in
//! `#[schemars(transform = ...)]`, that calls one helper of this module; a value form with a
//! named format implements `JsonSchema` by hand, once, with [`formatted`]; a typed map, whose
//! keys `schemars` does not describe, with [`map`]. The output stays
//! inside the subset of the core: an entry of `allOf` is a pair of `if` and `then`, the
//! condition tests declared members only, and `then` holds `required` only.

use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde_json::{Map, Value, json};

use super::Contract;
use crate::Format;
use crate::ident::string_schema;

/// The name of the member that holds the status of an artifact (SPEC-0001, Common rules), the
/// member the conditions on a status test.
pub const STATUS: &str = "status";

/// Requires the members `then` whenever the member `when` is present.
pub fn require_if_present(schema: &mut Schema, when: &str, then: &[&str]) {
    push(schema, json!({"required": [when]}), then);
}

/// Requires the members `then` whenever the member `when` is absent. A member set to null is
/// absent (ADR 0009 C2), and the condition `{"properties": {when: {"const": null}}}` holds for
/// exactly these two cases, in the core and in draft 2020-12: a member that is absent leaves
/// `properties` nothing to check, and no other value equals null. The subset has no `not`.
pub fn require_if_absent(schema: &mut Schema, when: &str, then: &[&str]) {
    push(schema, json!({"properties": {when: {"const": null}}}), then);
}

/// Requires the members `then` whenever the member `when` equals `value`, for example
/// `superseded_by` when `status` is `superseded`.
pub fn require_if_equals(schema: &mut Schema, when: &str, value: impl Into<Value>, then: &[&str]) {
    require_if_member(schema, when, &json!({"const": value.into()}), then);
}

/// Requires the members `then` whenever the member `when` equals one of `values`, for example
/// `issue` when `status` is `implementing` or `done`.
pub fn require_if_in(schema: &mut Schema, when: &str, values: &[&str], then: &[&str]) {
    require_if_member(schema, when, &json!({"enum": values}), then);
}

/// Requires the members `then` whenever the member `when` is a list that holds `value`, for
/// example `rubric` when `verification` holds `judge`. The condition states the type: in JSON
/// Schema `contains` holds for any value that is not a list, so without it a member of the
/// wrong type would also require `then`, beside its own error of type.
pub fn require_if_contains(
    schema: &mut Schema,
    when: &str,
    value: impl Into<Value>,
    then: &[&str],
) {
    let test = json!({"type": "array", "contains": {"const": value.into()}});
    require_if_member(schema, when, &test, then);
}

/// Requires the members `then` whenever the member `when` is present and passes `test`.
fn require_if_member(schema: &mut Schema, when: &str, test: &Value, then: &[&str]) {
    let condition = json!({"properties": {when: test}, "required": [when]});
    push(schema, condition, then);
}

/// The rules of an artifact the derive does not write (SPEC-0001, Common rules): its
/// [`constants`], and `superseded_by` required when `status` equals `superseded`, the spelling
/// of the terminal status of replacement of the type.
pub fn artifact(schema: &mut Schema, contract: &Contract, superseded: &str) {
    constants(schema, contract);
    require_if_equals(schema, STATUS, superseded, &["superseded_by"]);
}

/// The constants of a document of `contract`: `kind` is the constant name of the contract, and
/// `schema_version` its [`version`]. An artifact without a status, such as the hand-over
/// record, has these rules only.
pub fn constants(schema: &mut Schema, contract: &Contract) {
    constant(schema, "kind", contract.name.into(), None);
    version(schema, contract);
}

/// `schema_version` is the constant of the current version of `contract` (ADR 0030 G5): the
/// rule of every versioned document, also of an input without `kind`, such as the hand-over
/// input, whose file is a part of the same contract and version.
pub fn version(schema: &mut Schema, contract: &Contract) {
    constant(
        schema,
        "schema_version",
        contract.version.get().into(),
        Some("integer"),
    );
}

/// The member `member` of `schema` becomes the constant `value`, of the JSON type `kind` when
/// given, keeping its description: the subset allows only annotations beside a `$ref`, so the
/// derived schema of the member is replaced, not extended. A type keeps a value of another type
/// refused with `type`, before `enum`, as the interpreter reports it. A schema without
/// `properties` gets them, so that the rule is never lost; `properties` that are not an object
/// are kept, for the loader of the core to refuse by their place.
fn constant(schema: &mut Schema, member: &str, value: Value, kind: Option<&str>) {
    let properties = schema
        .ensure_object()
        .entry("properties")
        .or_insert_with(|| Value::Object(Map::new()));
    let Value::Object(properties) = properties else {
        return;
    };
    let description = properties
        .get(member)
        .and_then(|m| m.get("description"))
        .cloned();
    let mut replaced = Map::new();
    if let Some(description) = description {
        replaced.insert("description".to_owned(), description);
    }
    if let Some(kind) = kind {
        replaced.insert("type".to_owned(), kind.into());
    }
    replaced.insert("const".to_owned(), value);
    properties.insert(member.to_owned(), Value::Object(replaced));
}

/// The schema of a string in the named `format`, with its `description`.
#[must_use]
pub fn formatted(format: Format, description: &str) -> Schema {
    let mut schema = string_schema(description);
    schema.insert("format".to_owned(), format.id().into());
    schema
}

/// The schema of a map from `K` to `V`, with its `description`: `propertyNames` checks every
/// key by the schema of `K` and `additionalProperties` every value by the schema of `V` (#307).
/// The derive of `schemars` writes no `propertyNames` for a map, so a typed map uses this.
#[must_use]
pub fn map<K: JsonSchema, V: JsonSchema>(
    generator: &mut SchemaGenerator,
    description: &str,
) -> Schema {
    schemars::json_schema!({
        "description": description,
        "type": "object",
        "propertyNames": generator.subschema_for::<K>(),
        "additionalProperties": generator.subschema_for::<V>(),
    })
}

/// Appends the pair of `condition` and the requirement of `then` to the `allOf` of `schema`.
/// An `allOf` that is not a list is kept as the first entry of the new list, so that the
/// loader of the core refuses it by its place instead of losing a rule.
fn push(schema: &mut Schema, condition: Value, then: &[&str]) {
    let mut pair = Map::new();
    pair.insert("if".to_owned(), condition);
    pair.insert("then".to_owned(), json!({"required": then}));
    let pair = Value::Object(pair);
    let entries = schema
        .ensure_object()
        .entry("allOf")
        .or_insert_with(|| Value::Array(Vec::new()));
    match entries {
        Value::Array(entries) => entries.push(pair),
        other => *other = Value::Array(vec![other.take(), pair]),
    }
}
