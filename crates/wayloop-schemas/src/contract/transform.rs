//! The transform of the generator: what the derive writes outside the subset, rewritten into
//! it once, for every schema (ADR 0030 G2). It runs inside `schemars`, on the root and on every
//! subschema; no schema is edited after generation.

use std::collections::BTreeSet;

use schemars::Schema;
use schemars::transform::{Transform, transform_subschemas};
use serde_json::{Map, Value};

use super::Direction;

/// Description of a schema that accepts any JSON value, where its member gives none, such as
/// the elements of a list of values.
pub(super) const ANY_VALUE: &str = "Any JSON value.";

/// The transform of the generator for one direction.
#[derive(Debug, Clone, Copy)]
pub(super) struct Subset {
    pub(super) direction: Direction,
}

impl Transform for Subset {
    fn transform(&mut self, schema: &mut Schema) {
        if schema.as_bool() == Some(true) {
            let mut any = Map::new();
            any.insert("description".to_owned(), ANY_VALUE.into());
            *schema = Schema::from(any);
        }
        if let Some(object) = schema.as_object_mut() {
            if let Some(Value::String(description)) = object.get_mut("description") {
                *description = unwrapped(description);
            }
            optional_members_without_null(object);
            documented_constants_as_enum(object);
            if object.get("type").and_then(Value::as_str) == Some("integer")
                && object
                    .get("format")
                    .and_then(Value::as_str)
                    .is_some_and(|f| f.starts_with("int") || f.starts_with("uint"))
            {
                object.remove("format");
            }
            if self.direction == Direction::Input
                && object.get("type").and_then(Value::as_str) == Some("object")
                && object.contains_key("properties")
            {
                // An object that declares its members is closed. One that declares none is a
                // free object, written by hand for a document of another schema, and stays open
                // (ADR 0030 G2, amended with #256). A map keeps its schema of values, which the
                // loader of the core refuses: an input is never a map.
                object
                    .entry("additionalProperties")
                    .or_insert(Value::Bool(false));
            }
        }
        transform_subschemas(self, schema);
    }
}

/// Removes the `null` that the derive adds to an `Option`, from the members of `object` that
/// are not in its `required`: such a member is left out when not given (ADR 0009 C2). A
/// required member keeps its `null`, which the loader of the core then refuses.
fn optional_members_without_null(object: &mut Map<String, Value>) {
    let required: BTreeSet<String> = object
        .get("required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    let Some(Value::Object(members)) = object.get_mut("properties") else {
        return;
    };
    for (name, member) in members {
        if let (false, Value::Object(member)) = (required.contains(name), member) {
            without_null(member);
        }
    }
}

/// `type: [T, "null"]` becomes `type: T`; `anyOf: [S, {type: null}]` becomes `S`, its keywords
/// merged into `member` without replacing those already there, such as the description.
fn without_null(member: &mut Map<String, Value>) {
    if let Some(Value::Array(types)) = member.get_mut("type") {
        types.retain(|t| t != "null");
        if let [only] = types.as_slice() {
            let only = only.clone();
            member.insert("type".to_owned(), only);
        }
    }
    let is_null = |s: &Value| s.get("type").and_then(Value::as_str) == Some("null");
    let other = match member.get("anyOf") {
        Some(Value::Array(choices)) => match choices.as_slice() {
            [a, b] if is_null(b) && !is_null(a) => a.as_object().cloned(),
            [a, b] if is_null(a) && !is_null(b) => b.as_object().cloned(),
            _ => None,
        },
        _ => None,
    };
    if let Some(other) = other {
        member.remove("anyOf");
        for (keyword, value) in other {
            member.entry(keyword).or_insert(value);
        }
    }
}

/// A `oneOf` whose every entry is a constant with a description that is not blank, as the
/// derive writes a
/// documented enum, becomes an `enum`; the meaning of each value is appended to the
/// description, one line per value: ``- `value`: meaning`` (ADR 0024). A `oneOf` with an
/// entry of another form is left as it is, for the checks to refuse.
fn documented_constants_as_enum(object: &mut Map<String, Value>) {
    let Some(Value::Array(entries)) = object.get("oneOf") else {
        return;
    };
    let mut values = Vec::with_capacity(entries.len());
    let mut lines = Vec::with_capacity(entries.len());
    let mut kinds = BTreeSet::new();
    for entry in entries {
        let Some(entry) = entry.as_object() else {
            return;
        };
        let (Some(value), Some(meaning)) = (
            entry.get("const"),
            entry
                .get("description")
                .and_then(Value::as_str)
                .filter(|meaning| !meaning.trim().is_empty()),
        ) else {
            return;
        };
        if entry
            .keys()
            .any(|k| !matches!(k.as_str(), "const" | "type" | "description"))
        {
            return;
        }
        if let Some(kind) = entry.get("type").and_then(Value::as_str) {
            kinds.insert(kind.to_owned());
        }
        lines.push(meaning_line(value, &unwrapped(meaning)));
        values.push(value.clone());
    }
    let kind = match (kinds.pop_first(), kinds.is_empty()) {
        (Some(kind), true) => Some(kind),
        (None, true) => None,
        // Constants of several types: left as they are, for the checks to refuse.
        (_, false) => return,
    };
    object.remove("oneOf");
    if let Some(kind) = kind {
        object.insert("type".to_owned(), kind.into());
    }
    object.insert("enum".to_owned(), values.into());
    if let Some(Value::String(description)) = object.get_mut("description") {
        description.push_str("\n\n");
        description.push_str(&lines.join("\n"));
    }
}

/// `text` with the line breaks of its doc comment removed: the lines of a paragraph are joined
/// by a space, paragraphs stay apart by an empty line. `schemars` keeps the line breaks of the
/// source, which wrap at the width of the code, not of the reader.
pub(super) fn unwrapped(text: &str) -> String {
    text.split("\n\n")
        .map(|paragraph| {
            paragraph
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .filter(|paragraph| !paragraph.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// The text of `value` in a description: a string as written, any other value as JSON.
pub(super) fn value_text(value: &Value) -> String {
    value
        .as_str()
        .map_or_else(|| value.to_string(), str::to_owned)
}

/// The line of a description that gives the meaning of `value`.
pub(super) fn meaning_line(value: &Value, meaning: &str) -> String {
    format!("- `{}`: {meaning}", value_text(value))
}
