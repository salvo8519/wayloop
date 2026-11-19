//! Checking a document against a loaded schema: one pass, every error, in document order.

use std::collections::BTreeSet;

use serde_json::{Map, Value};

use super::closest::closest;
use super::load::{Compiled, Node, Rules, Type, Values};
use crate::{Code, Error, Pointer, Reason};

/// The largest magnitude of an integer, 2^53 - 1: the largest integer that every JSON parser
/// holds exactly (decision of the maintainer, 2026-11-20).
pub(super) const SAFE_INTEGER: i64 = 9_007_199_254_740_991;

/// Whether `n` is within plus or minus [`SAFE_INTEGER`].
pub(super) fn is_safe(n: i128) -> bool {
    n.unsigned_abs() <= SAFE_INTEGER.unsigned_abs().into()
}

/// The integer value of `value`: a JSON number without a fraction. A number beyond the range
/// of `i128` saturates, which keeps every comparison with an `i64` bound exact.
pub(super) fn integer(value: &Value) -> Option<i128> {
    let number = value.as_number()?;
    if let Some(i) = number.as_i64() {
        return Some(i.into());
    }
    if let Some(u) = number.as_u64() {
        return Some(u.into());
    }
    let float = number.as_f64()?;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the value has no fraction, and a cast from a float saturates"
    )]
    let whole = float.trunc() as i128;
    (float.is_finite() && float.trunc().to_bits() == float.to_bits()).then_some(whole)
}

/// Whether `a` and `b` are the same JSON value; numbers compare by value.
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(_), Value::Number(_)) => {
            a == b || matches!((integer(a), integer(b)), (Some(x), Some(y)) if x == y)
        }
        _ => a == b,
    }
}

fn type_of(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) if integer(value).is_some() => "an integer",
        Value::Number(_) => "a number with a fraction",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

fn has_type(kind: Type, value: &Value) -> bool {
    match kind {
        Type::Null => value.is_null(),
        Type::Boolean => value.is_boolean(),
        Type::Integer => integer(value).is_some(),
        Type::String => value.is_string(),
        Type::Array => value.is_array(),
        Type::Object => value.is_object(),
    }
}

/// A size of the document as a `u64`, saturating on a platform where `usize` is wider.
fn size(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

fn quoted(values: &[Value]) -> String {
    values
        .iter()
        .map(Value::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

/// One pass over a document; collects every error with `code`.
pub(super) struct Checker<'s> {
    pub(super) schema: &'s Compiled,
    pub(super) code: Code,
    pub(super) errors: Vec<Error>,
}

impl Checker<'_> {
    fn error(&self, message: String, pointer: &Pointer, reason: Reason) -> Error {
        Error::new(self.code, message, pointer.clone(), reason)
    }

    fn push(&mut self, message: String, pointer: &Pointer, reason: Reason) {
        let error = self.error(message, pointer, reason);
        self.errors.push(error);
    }

    /// Checks `value`, found at `pointer`, against the node `node`. The recursion follows the
    /// schema, whose expanded depth is bounded when it is loaded.
    pub(super) fn check(&mut self, node: usize, value: &Value, pointer: &Pointer) {
        match self.schema.nodes.get(node) {
            Some(Node::Ref(target)) => self.check(*target, value, pointer),
            Some(Node::Rules(rules)) => self.rules(rules, value, pointer),
            None => {}
        }
    }

    fn rules(&mut self, rules: &Rules, value: &Value, pointer: &Pointer) {
        if let Some(kind) = rules.kind {
            if !has_type(kind, value) {
                let message = format!("Expected {}, found {}.", kind.name(), type_of(value));
                self.push(message, pointer, Reason::Type);
                return;
            }
        }
        if integer(value).is_some_and(|n| !is_safe(n)) {
            let message = format!(
                "The integer {value} is beyond the accepted range, from -{SAFE_INTEGER} to {SAFE_INTEGER}."
            );
            self.push(message, pointer, Reason::Range);
            return;
        }
        if let Some(values) = &rules.values {
            self.values(values, value, pointer);
        }
        match value {
            Value::String(text) => self.string(rules, text, pointer),
            Value::Number(_) => self.number(rules, value, pointer),
            Value::Array(items) => self.array(rules, items, pointer),
            Value::Object(members) => self.object(rules, members, pointer),
            Value::Null | Value::Bool(_) => {}
        }
    }

    fn values(&mut self, values: &Values, value: &Value, pointer: &Pointer) {
        if values.list.iter().any(|accepted| same(accepted, value)) {
            return;
        }
        let message = if values.is_const {
            format!("The value must be {}.", quoted(&values.list))
        } else {
            format!("The value must be one of {}.", quoted(&values.list))
        };
        let error = self
            .error(message, pointer, Reason::Enum)
            .with_expected(values.list.iter().cloned());
        self.errors.push(error);
    }

    fn string(&mut self, rules: &Rules, text: &str, pointer: &Pointer) {
        let length = size(text.chars().count());
        if let Some(min) = rules.min_length.filter(|min| length < *min) {
            let message = format!("The text has {length} characters; at least {min} are required.");
            self.push(message, pointer, Reason::Length);
        }
        if let Some(max) = rules.max_length.filter(|max| length > *max) {
            let message = format!("The text has {length} characters; at most {max} are accepted.");
            self.push(message, pointer, Reason::Length);
        }
        if let Some(format) = rules.format {
            if !format.accepts(text) {
                let message = format!(
                    "The text is not in the form of the format `{}`.",
                    format.name()
                );
                let mut error = self.error(message, pointer, Reason::Syntax);
                if let Some(fix) = format.fix(text) {
                    error = error.with_fix(fix);
                }
                self.errors.push(error);
            }
        }
    }

    fn number(&mut self, rules: &Rules, value: &Value, pointer: &Pointer) {
        // Numeric bounds exist only beside `type: integer`, which has been checked.
        let Some(number) = integer(value) else {
            return;
        };
        if let Some(min) = rules.minimum.filter(|min| number < i128::from(*min)) {
            let message = format!("The number {value} is below the minimum {min}.");
            self.push(message, pointer, Reason::Range);
        }
        if let Some(max) = rules.maximum.filter(|max| number > i128::from(*max)) {
            let message = format!("The number {value} is above the maximum {max}.");
            self.push(message, pointer, Reason::Range);
        }
    }

    fn array(&mut self, rules: &Rules, items: &[Value], pointer: &Pointer) {
        let length = size(items.len());
        if let Some(min) = rules.min_items.filter(|min| length < *min) {
            let message = format!("The list has {length} items; at least {min} are required.");
            self.push(message, pointer, Reason::Length);
        }
        if let Some(max) = rules.max_items.filter(|max| length > *max) {
            let message = format!("The list has {length} items; at most {max} are accepted.");
            self.push(message, pointer, Reason::Length);
        }
        if let Some(schema) = rules.items {
            for (i, item) in items.iter().enumerate() {
                self.check(schema, item, &pointer.join_index(i));
            }
        }
    }

    fn object(&mut self, rules: &Rules, members: &Map<String, Value>, pointer: &Pointer) {
        for (key, member) in members {
            let at = pointer.join(key);
            if let Some(&schema) = rules.properties.get(key) {
                self.check(schema, member, &at);
            } else if rules.closed {
                let absent = rules
                    .properties
                    .keys()
                    .map(String::as_str)
                    .filter(|k| !members.contains_key(*k));
                let mut error =
                    self.error(format!("Unknown field `{key}`."), &at, Reason::UnknownField);
                if let Some(fix) = closest(key, absent) {
                    error = error.with_fix(fix);
                }
                self.errors.push(error);
            }
        }
        let mut reported: BTreeSet<&str> = BTreeSet::new();
        for name in rules.required.iter().filter(|n| !members.contains_key(*n)) {
            reported.insert(name);
            let message = format!("The required field `{name}` is missing.");
            self.push(message, &pointer.join(name), Reason::Missing);
        }
        for conditional in &rules.conditionals {
            if !self.matches_object(conditional.when, members) {
                continue;
            }
            let tested: Vec<&str> = self
                .schema
                .conditions
                .get(conditional.when)
                .map(|c| {
                    c.properties
                        .keys()
                        .map(String::as_str)
                        .filter(|k| members.contains_key(*k))
                        .collect()
                })
                .unwrap_or_default();
            for name in &conditional.then {
                if members.contains_key(name) || !reported.insert(name) {
                    continue;
                }
                let message = if tested.is_empty() {
                    format!("The field `{name}` is required by a condition of the schema.")
                } else {
                    let fields: Vec<String> = tested.iter().map(|t| format!("`{t}`")).collect();
                    format!(
                        "The field `{name}` is required by the value of {}.",
                        fields.join(", ")
                    )
                };
                let pointers = tested.iter().map(|t| pointer.join(t)).collect::<Vec<_>>();
                let error = self
                    .error(message, &pointer.join(name), Reason::Requires)
                    .with_pointers(pointers);
                self.errors.push(error);
            }
        }
    }

    /// Whether `value` matches the condition `index`: the semantics of JSON Schema for the
    /// keywords of a condition. Conditions are trees of bounded depth.
    fn matches(&self, index: usize, value: &Value) -> bool {
        if let Value::Object(members) = value {
            return self.matches_object(index, members);
        }
        let Some(condition) = self.schema.conditions.get(index) else {
            return true;
        };
        let accepted = condition
            .values
            .as_ref()
            .is_none_or(|values| values.list.iter().any(|v| same(v, value)));
        let contains = match (value, condition.contains) {
            (Value::Array(items), Some(child)) => {
                items.iter().any(|item| self.matches(child, item))
            }
            _ => true,
        };
        accepted && contains
    }

    /// [`Self::matches`] for an object, without building a value of it. `enum` and `const`
    /// hold scalars only, so an object never matches them.
    fn matches_object(&self, index: usize, members: &Map<String, Value>) -> bool {
        let Some(condition) = self.schema.conditions.get(index) else {
            return true;
        };
        condition.values.is_none()
            && condition.required.iter().all(|n| members.contains_key(n))
            && condition
                .properties
                .iter()
                .all(|(key, child)| members.get(key).is_none_or(|m| self.matches(*child, m)))
    }
}
