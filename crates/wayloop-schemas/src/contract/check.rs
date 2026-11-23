//! The checks of the generator, after the transform (ADR 0030 G1, G3): the rules of the
//! contracts that are not rules of the subset.
//!
//! The subset has one definition, the loader of the core (`wayloop_core::schema::Schema::load`):
//! a keyword outside it (`pattern`, `default`, `uniqueItems`, a boolean schema, a `null`
//! type, a format that is not named, a reference out of the file or a reference chain) is
//! refused there, and `wayloop_core::schema::contract_schema` runs both. These checks hold
//! what a schema of the subset can still break: a description on the root, every definition and
//! every member; the meaning of every enum value; at most four levels of objects; no
//! `additionalProperties: false` in an output; and, in an input, `additionalProperties` on every
//! object but the free objects of [`FREE_OBJECTS`]: `false`, or the schema of every member of
//! a map, such as the `lines` of `validate`.

use serde_json::{Map, Value};

use super::transform::{is_object, meaning_line, value_text};
use super::{ContractError, Direction, MAX_OBJECT_LEVELS, Problem};
use crate::Pointer;
use crate::validate::FreeDocument;

/// Keywords whose value is one subschema that needs no description of its own.
const SUBSCHEMA: [&str; 3] = ["items", "additionalProperties", "propertyNames"];

/// Keywords whose value is a condition, or what a condition requires: their members are names
/// of the members of the object, described where they are declared.
const CONDITION: [&str; 3] = ["if", "then", "contains"];

/// Keywords whose value is a list of subschemas.
const SUBSCHEMAS: [&str; 3] = ["allOf", "anyOf", "oneOf"];

/// The definitions that may stay open in an input: documents of another schema, written by
/// hand with `value::free_object_schema` and checked by the core (ADR 0030 G2, as amended for
/// #256). Every other object of an input is closed, or a map whose members one schema checks.
pub(super) const FREE_OBJECTS: [&str; 1] = [FreeDocument::NAME];

/// Whether an object of an input is closed: `additionalProperties: false`, or a map whose
/// schema of values, or the definition it refers to, states a type, so that no member goes
/// unchecked. A map of any value, such as `BTreeMap<String, Value>` or a map of a newtype over
/// it, stays open.
fn is_closed(root: &Value, schema: &Map<String, Value>) -> bool {
    match schema.get("additionalProperties") {
        Some(Value::Bool(false)) => true,
        Some(values @ Value::Object(_)) => {
            let target = values
                .get("$ref")
                .and_then(Value::as_str)
                .and_then(|r| r.strip_prefix("#/$defs/"))
                .and_then(|name| root.get("$defs")?.get(name));
            target.unwrap_or(values).get("type").is_some()
        }
        _ => false,
    }
}

/// Most references followed on one path. A reference that reaches itself is refused by the
/// loader of the core; this bound only stops the walk.
const MAX_HOPS: usize = 64;

/// The first place of `schema` that leaves the rules of the contracts.
pub(super) fn check(schema: &Value, direction: Direction) -> Result<(), ContractError> {
    walk(
        schema,
        schema,
        &Pointer::root(),
        Need::Description,
        direction,
    )?;
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
    root: &Value,
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
    if direction == Direction::Input
        && need != Need::InCondition
        && is_object(schema)
        && !is_closed(root, schema)
        && !FREE_OBJECTS
            .iter()
            .any(|name| *pointer == Pointer::root().join("$defs").join(name))
    {
        return Err(refuse(pointer, Problem::OpenInput));
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
                walk(root, member, &at.join(name), member_need, direction)?;
            }
        }
    }
    let inner = match need {
        Need::InCondition => Need::InCondition,
        Need::Description | Need::Nothing => Need::Nothing,
    };
    for keyword in SUBSCHEMA {
        if let Some(sub) = schema.get(keyword) {
            walk(root, sub, &pointer.join(keyword), inner, direction)?;
        }
    }
    for keyword in CONDITION {
        if let Some(sub) = schema.get(keyword) {
            walk(
                root,
                sub,
                &pointer.join(keyword),
                Need::InCondition,
                direction,
            )?;
        }
    }
    for keyword in SUBSCHEMAS {
        if let Some(Value::Array(entries)) = schema.get(keyword) {
            let at = pointer.join(keyword);
            for (i, entry) in entries.iter().enumerate() {
                walk(root, entry, &at.join_index(i), inner, direction)?;
            }
        }
    }
    Ok(())
}

/// Whether an object of an output refuses members it does not declare: any
/// `additionalProperties` but the schema of the values of a pure map, an object that declares
/// no member. Beside declared members (a map flattened into a type), the schema of the map would
/// refuse a future member of another type (ADR 0009 C5).
fn closes_output(schema: &Map<String, Value>) -> bool {
    match schema.get("additionalProperties") {
        None => false,
        Some(Value::Object(_)) => schema.contains_key("properties"),
        Some(_) => true,
    }
}

/// The rules on the keywords of one subschema.
fn own_keywords(
    schema: &Map<String, Value>,
    pointer: &Pointer,
    description: &str,
    direction: Direction,
) -> Result<(), ContractError> {
    if direction == Direction::Output && closes_output(schema) {
        let at = pointer.join("additionalProperties");
        return Err(refuse(&at, Problem::ClosedOutput));
    }
    if let Some(Value::Array(values)) = schema.get("enum") {
        let at = pointer.join("enum");
        for (i, value) in values.iter().enumerate() {
            let line = meaning_line(value, "");
            let explained = description.lines().any(|l| {
                l.strip_prefix(&line)
                    .is_some_and(|meaning| !meaning.trim().is_empty())
            });
            if !explained {
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
            let undocumented = entry
                .get("description")
                .and_then(Value::as_str)
                .is_none_or(|d| d.trim().is_empty());
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
    // The values of a map nest like a member: an object there is one level deeper (#307).
    for keyword in ["items", "additionalProperties"] {
        if let Some(sub) = schema.get(keyword) {
            levels(root, sub, &pointer.join(keyword), level, hops)?;
        }
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
