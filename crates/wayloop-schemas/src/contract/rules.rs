//! Rules the derive does not write, for the contract types (ADR 0030 G2).
//!
//! A conditional requirement is written by a function of the type, named in
//! `#[schemars(transform = ...)]`, that calls one helper of this module; a value form with a
//! named format implements `JsonSchema` by hand, once, with [`formatted`]. The output stays
//! inside the subset of the core: an entry of `allOf` is a pair of `if` and `then`, the
//! condition tests declared members only, and `then` holds `required` only.

use schemars::Schema;
use serde_json::{Map, Value, json};

use crate::Format;

/// Requires the members `then` whenever the member `when` is present.
pub fn require_if_present(schema: &mut Schema, when: &str, then: &[&str]) {
    push(schema, json!({"required": [when]}), then);
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

/// The schema of a string in the named `format`, with its `description`.
#[must_use]
pub fn formatted(format: Format, description: &str) -> Schema {
    let mut schema = Map::new();
    schema.insert("description".to_owned(), description.into());
    schema.insert("type".to_owned(), "string".into());
    schema.insert("format".to_owned(), format.id().into());
    Schema::from(schema)
}

/// Appends the pair of `condition` and the requirement of `then` to the `allOf` of `schema`.
fn push(schema: &mut Schema, condition: Value, then: &[&str]) {
    let mut pair = Map::new();
    pair.insert("if".to_owned(), condition);
    pair.insert("then".to_owned(), json!({"required": then}));
    let pair = Value::Object(pair);
    let entries = schema
        .ensure_object()
        .entry("allOf")
        .or_insert_with(|| Value::Array(Vec::new()));
    if let Value::Array(entries) = entries {
        entries.push(pair);
    }
}
