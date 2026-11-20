//! The checks of the generator, after the transform (ADR 0030 G1, G3): the rules of the
//! contracts that a schema of the subset could still break.

use serde_json::{Map, Value};

use super::transform::{meaning_line, value_text};
use super::{ContractError, Direction, MAX_OBJECT_LEVELS, Problem};
use crate::{Format, Pointer};

/// Keywords the contracts never use: the core holds no regular expression engine, defaults are
/// applied by the core and reported (ADR 0009 C2), and lists are `Vec`, not sets.
const REFUSED: [&str; 3] = ["pattern", "default", "uniqueItems"];

/// Keywords whose value is one subschema that needs no description of its own.
const SUBSCHEMA: [&str; 2] = ["items", "additionalProperties"];

/// Keywords whose value is a condition, or what a condition requires: their members are names
/// of the members of the object, described where they are declared.
const CONDITION: [&str; 4] = ["if", "then", "else", "contains"];

/// Keywords whose value is a list of subschemas.
const SUBSCHEMAS: [&str; 3] = ["allOf", "anyOf", "oneOf"];

/// Most references followed on one path, a bound for a schema that reaches itself.
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

/// Checks the subschema `value` at `pointer` and every subschema below it.
fn walk(
    value: &Value,
    pointer: &Pointer,
    need: Need,
    direction: Direction,
) -> Result<(), ContractError> {
    let schema = match value {
        Value::Object(schema) => schema,
        Value::Bool(_) => return Err(refuse(pointer, Problem::Boolean)),
        // Not a schema: the loader of the core refuses it.
        _ => return Ok(()),
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
        if let Some(sub) = schema.get(keyword).filter(|s| s.is_object()) {
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
    let null = |t: &Value| t == "null";
    match schema.get("type") {
        Some(Value::Array(types)) if types.iter().any(null) => {
            return Err(refuse(&pointer.join("type"), Problem::Null));
        }
        Some(t) if null(t) => return Err(refuse(&pointer.join("type"), Problem::Null)),
        _ => {}
    }
    if let Some(keyword) = REFUSED.into_iter().find(|k| schema.contains_key(*k)) {
        return Err(refuse(&pointer.join(keyword), Problem::Refused(keyword)));
    }
    if direction == Direction::Output && schema.contains_key("additionalProperties") {
        let keyword = "additionalProperties";
        return Err(refuse(&pointer.join(keyword), Problem::Refused(keyword)));
    }
    if let Some(format) = schema.get("format") {
        if format.as_str().and_then(Format::named).is_none() {
            let name = value_text(format);
            return Err(refuse(
                &pointer.join("format"),
                Problem::UnknownFormat(name),
            ));
        }
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
/// `level` objects, following references into the definitions of `root`.
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
        if hops >= MAX_HOPS {
            return Err(refuse(pointer, Problem::TooDeep));
        }
        if let Some(definition) = definition {
            let at = Pointer::root().join("$defs").join(name);
            return levels(root, definition, &at, level, hops.saturating_add(1));
        }
        return Ok(());
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
