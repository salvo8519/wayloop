//! The checks of the generator, after the transform (ADR 0030 G1, G3): the rules of the
//! contracts that are not rules of the subset.
//!
//! The subset has one definition, the loader of the core (`wayloop_core::schema::Schema::load`):
//! a keyword outside it (`pattern`, `default`, `uniqueItems`, a boolean schema, a `null`
//! type, a format that is not named, a reference out of the file or a reference chain) is
//! refused there, and `wayloop_core::schema::contract_schema` runs both. These checks hold
//! what a schema of the subset can still break: a description on the root, every definition and
//! every member; the meaning of every enum value; at most four levels of objects; and no
//! `additionalProperties` in an output.

use serde_json::{Map, Value};

use super::transform::{meaning_line, value_text};
use super::{ContractError, Direction, MAX_OBJECT_LEVELS, Problem};
use crate::Pointer;

/// Keywords whose value is one subschema that needs no description of its own.
const SUBSCHEMA: [&str; 2] = ["items", "additionalProperties"];

/// Keywords whose value is a condition, or what a condition requires: their members are names
/// of the members of the object, described where they are declared.
const CONDITION: [&str; 3] = ["if", "then", "contains"];

/// Keywords whose value is a list of subschemas.
const SUBSCHEMAS: [&str; 3] = ["allOf", "anyOf", "oneOf"];

/// Most references followed on one path. A reference that reaches itself is refused by the
/// loader of the core; this bound only stops the walk.
const MAX_HOPS: usize = 64;

/// The first place of `schema` that leaves the rules of the contracts.
pub(super) fn check(schema: &Value, direction: Direction) -> Result<(), ContractError> {
    walk(schema, &Pointer::root(), Need::Description, direction)?;
    levels(schema, schema, &Pointer::root(), 0, 0)
}

fn refuse(pointer: &Pointer, problem: Problem) -> ContractError {
    ContractError {
        pointer: pointer.clone(),
        problem,
    }
}

/// Whether a subschema needs a description of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Need {
    /// The root, a definition or a member.
    Description,
    /// Any other subschema outside a condition.
    Nothing,
    /// A subschema inside a condition, where members are only named.
    InCondition,
}

/// Checks the subschema `value` at `pointer` and every subschema below it. A value that is not
/// an object is not a schema of the subset; the loader of the core refuses it.
fn walk(
    value: &Value,
    pointer: &Pointer,
    need: Need,
    direction: Direction,
) -> Result<(), ContractError> {
    let Some(schema) = value.as_object() else {
        return Ok(());
    };
    let description = schema
        .get("description")
        .and_then(Value::as_str)
        .filter(|d| !d.trim().is_empty());
    if need == Need::Description && description.is_none() {
        return Err(refuse(pointer, Problem::MissingDescription));
    }
    own_keywords(schema, pointer, description.unwrap_or_default(), direction)?;
    for keyword in ["properties", "$defs"] {
        if let Some(Value::Object(members)) = schema.get(keyword) {
            let at = pointer.join(keyword);
            let member_need = match need {
                Need::InCondition => Need::InCondition,
                Need::Description | Need::Nothing => Need::Description,
            };
            for (name, member) in members {
                walk(member, &at.join(name), member_need, direction)?;
            }
        }
    }
    let inner = match need {
        Need::InCondition => Need::InCondition,
        Need::Description | Need::Nothing => Need::Nothing,
    };
    for keyword in SUBSCHEMA {
        if let Some(sub) = schema.get(keyword) {
            walk(sub, &pointer.join(keyword), inner, direction)?;
        }
    }
    for keyword in CONDITION {
        if let Some(sub) = schema.get(keyword) {
            walk(sub, &pointer.join(keyword), Need::InCondition, direction)?;
        }
    }
    for keyword in SUBSCHEMAS {
        if let Some(Value::Array(entries)) = schema.get(keyword) {
            let at = pointer.join(keyword);
            for (i, entry) in entries.iter().enumerate() {
                walk(entry, &at.join_index(i), inner, direction)?;
            }
        }
    }
    Ok(())
}

/// The rules on the keywords of one subschema.
fn own_keywords(
    schema: &Map<String, Value>,
    pointer: &Pointer,
    description: &str,
    direction: Direction,
) -> Result<(), ContractError> {
    if direction == Direction::Output && schema.contains_key("additionalProperties") {
        let at = pointer.join("additionalProperties");
        return Err(refuse(&at, Problem::ClosedOutput));
    }
    if let Some(Value::Array(values)) = schema.get("enum") {
        let at = pointer.join("enum");
        for (i, value) in values.iter().enumerate() {
            let line = meaning_line(value, "");
            if !description.lines().any(|l| l.starts_with(&line)) {
                return Err(refuse(
                    &at.join_index(i),
                    Problem::UndocumentedValue(value_text(value)),
                ));
            }
        }
    }
    if let Some(Value::Array(entries)) = schema.get("oneOf") {
        let at = pointer.join("oneOf");
        for (i, entry) in entries.iter().enumerate() {
            let undocumented = entry.get("description").is_none();
            if let (Some(value), true) = (entry.get("const"), undocumented) {
                return Err(refuse(
                    &at.join_index(i),
                    Problem::UndocumentedValue(value_text(value)),
                ));
            }
        }
    }
    Ok(())
}

/// Refuses objects nested deeper than [`MAX_OBJECT_LEVELS`] below `node`, which is inside
/// `level` objects, following references into the definitions of `root`. A reference out of
/// `#/$defs/`, to a missing definition or reaching itself is left to the loader of the core.
fn levels(
    root: &Value,
    node: &Value,
    pointer: &Pointer,
    level: usize,
    hops: usize,
) -> Result<(), ContractError> {
    let Some(schema) = node.as_object() else {
        return Ok(());
    };
    if let Some(target) = schema.get("$ref").and_then(Value::as_str) {
        let name = target.strip_prefix("#/$defs/").unwrap_or_default();
        let definition = root.get("$defs").and_then(|defs| defs.get(name));
        return match definition {
            Some(definition) if hops < MAX_HOPS => {
                let at = Pointer::root().join("$defs").join(name);
                levels(root, definition, &at, level, hops.saturating_add(1))
            }
            _ => Ok(()),
        };
    }
    let is_object = schema.get("type").and_then(Value::as_str) == Some("object");
    let level = if is_object {
        level.saturating_add(1)
    } else {
        level
    };
    if level > MAX_OBJECT_LEVELS {
        return Err(refuse(pointer, Problem::TooDeep));
    }
    if let Some(Value::Object(members)) = schema.get("properties") {
        let at = pointer.join("properties");
        for (name, member) in members {
            levels(root, member, &at.join(name), level, hops)?;
        }
    }
    if let Some(items) = schema.get("items") {
        levels(root, items, &pointer.join("items"), level, hops)?;
    }
    for keyword in SUBSCHEMAS {
        if let Some(Value::Array(entries)) = schema.get(keyword) {
            let at = pointer.join(keyword);
            for (i, entry) in entries.iter().enumerate() {
                levels(root, entry, &at.join_index(i), level, hops)?;
            }
        }
    }
    Ok(())
}
