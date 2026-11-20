//! Rules the derive does not write, for the contract types (ADR 0030 G2).
//!
//! A conditional requirement is written by a function of the type, named in
//! `#[schemars(transform = ...)]`, that calls one helper of this module; a value form with a
//! named format implements `JsonSchema` by hand, once, with [`formatted`]. The output stays
//! inside the subset of the core: an entry of `allOf` is a pair of `if` and `then`, the
//! condition tests declared members only, and `then` holds `required` only.

use core::num::NonZeroU32;

use schemars::Schema;
use serde_json::{Map, Value, json};

use crate::Format;
use crate::ident::string_schema;

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
    let condition = json!({
        "properties": {when: {"const": value.into()}},
        "required": [when],
    });
    push(schema, condition, then);
}

/// The rules of an artifact the derive does not write (SPEC-0001, Common rules): `kind` is the
/// constant `kind` and `schema_version` the constant `version`, both from the registration of
/// the contract (ADR 0030 G5); `superseded_by` is required when `status` equals `superseded`,
/// the spelling of the terminal status of replacement of the type.
pub fn artifact(schema: &mut Schema, kind: &str, version: NonZeroU32, superseded: &str) {
    constant(schema, "kind", kind.into(), None);
    constant(
        schema,
        "schema_version",
        version.get().into(),
        Some("integer"),
    );
    require_if_equals(schema, "status", superseded, &["superseded_by"]);
}

/// The member `member` of `schema` becomes the constant `value`, of the JSON type `kind` when
/// given, keeping its description: the subset allows only annotations beside a `$ref`, so the
/// derived schema of the member is replaced, not extended. A type keeps a value of another type
/// refused with `type`, before `enum`, as the interpreter reports it.
fn constant(schema: &mut Schema, member: &str, value: Value, kind: Option<&str>) {
    let Some(Value::Object(properties)) = schema.ensure_object().get_mut("properties") else {
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
