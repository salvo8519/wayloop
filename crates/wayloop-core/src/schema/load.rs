//! Loading a schema: every keyword is checked against the subset and compiled into an arena
//! of nodes, references are resolved, and the depth of the expanded schema is bounded.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};

use super::check::is_safe;
use super::format::Format;
use super::{LoadError, MAX_DEPTH};
use crate::Pointer;

/// The only dialect accepted in `$schema`.
const DIALECT: &str = "https://json-schema.org/draft/2020-12/schema";

/// Annotations: accepted everywhere, never checked.
const ANNOTATIONS: [&str; 3] = ["$comment", "title", "description"];

/// Keywords of a schema that validates a value. `$schema`, `$id` and `$defs` are refused
/// below the root.
const RULE_KEYWORDS: [&str; 21] = [
    "$schema",
    "$id",
    "$defs",
    "$ref",
    "type",
    "enum",
    "const",
    "minLength",
    "maxLength",
    "minItems",
    "maxItems",
    "minimum",
    "maximum",
    "format",
    "items",
    "properties",
    "additionalProperties",
    "required",
    "if",
    "then",
    "allOf",
];

/// Keywords of the condition of an `if`.
const CONDITION_KEYWORDS: [&str; 5] = ["properties", "required", "enum", "const", "contains"];

/// Keywords of an entry of `allOf`.
const ALL_OF_KEYWORDS: [&str; 2] = ["if", "then"];

/// Keywords of a `then`.
const THEN_KEYWORDS: [&str; 1] = ["required"];

/// A type of the subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Type {
    Null,
    Boolean,
    Integer,
    String,
    Array,
    Object,
}

impl Type {
    fn named(name: &str) -> Option<Self> {
        Some(match name {
            "null" => Type::Null,
            "boolean" => Type::Boolean,
            "integer" => Type::Integer,
            "string" => Type::String,
            "array" => Type::Array,
            "object" => Type::Object,
            _ => return None,
        })
    }

    pub(super) fn name(self) -> &'static str {
        match self {
            Type::Null => "null",
            Type::Boolean => "boolean",
            Type::Integer => "integer",
            Type::String => "string",
            Type::Array => "array",
            Type::Object => "object",
        }
    }
}

/// Accepted values of `enum` or `const`.
#[derive(Debug, Clone)]
pub(super) struct Values {
    pub(super) list: Vec<Value>,
    pub(super) is_const: bool,
}

/// A conditional requirement: `if` and the `required` of its `then`.
#[derive(Debug, Clone)]
pub(super) struct Conditional {
    /// Index of the condition in the arena of conditions.
    pub(super) when: usize,
    pub(super) then: Vec<String>,
}

/// The keywords of a schema that validates a value.
#[derive(Debug, Clone, Default)]
pub(super) struct Rules {
    pub(super) kind: Option<Type>,
    pub(super) values: Option<Values>,
    pub(super) min_length: Option<u64>,
    pub(super) max_length: Option<u64>,
    pub(super) min_items: Option<u64>,
    pub(super) max_items: Option<u64>,
    pub(super) minimum: Option<i64>,
    pub(super) maximum: Option<i64>,
    pub(super) format: Option<Format>,
    pub(super) items: Option<usize>,
    pub(super) properties: BTreeMap<String, usize>,
    pub(super) closed: bool,
    pub(super) required: Vec<String>,
    pub(super) conditionals: Vec<Conditional>,
}

/// A node of the arena of schemas.
#[derive(Debug, Clone)]
pub(super) enum Node {
    /// A `$ref`, resolved to the index of the definition.
    Ref(usize),
    /// A schema with its keywords.
    Rules(Box<Rules>),
}

/// The condition of an `if`.
#[derive(Debug, Clone, Default)]
pub(super) struct Condition {
    pub(super) properties: BTreeMap<String, usize>,
    pub(super) required: Vec<String>,
    pub(super) values: Option<Values>,
    pub(super) contains: Option<usize>,
}

/// A loaded schema: the arenas and the index of the root.
#[derive(Debug, Clone)]
pub(super) struct Compiled {
    pub(super) nodes: Vec<Node>,
    pub(super) conditions: Vec<Condition>,
    pub(super) root: usize,
}

/// State of a node while the depth of the expanded schema is measured.
#[derive(Debug, Clone, Copy)]
enum Measure {
    New,
    Visiting,
    Done(usize),
}

struct Loader {
    nodes: Vec<Node>,
    /// Pointer into the schema of each node, for the errors of the expanded schema.
    pointers: Vec<Pointer>,
    conditions: Vec<Condition>,
    /// Height of each condition: the deepest level of subschemas below it.
    condition_heights: Vec<usize>,
    condition_pointers: Vec<Pointer>,
    /// References to resolve: the node, the name of the definition, the pointer of `$ref`.
    refs: Vec<(usize, String, Pointer)>,
}

pub(super) fn load(schema: &Value) -> Result<Compiled, LoadError> {
    let mut loader = Loader {
        nodes: Vec::new(),
        pointers: Vec::new(),
        conditions: Vec::new(),
        condition_heights: Vec::new(),
        condition_pointers: Vec::new(),
        refs: Vec::new(),
    };
    let root_pointer = Pointer::root();
    let root = loader.rules(schema, &root_pointer, 0, true)?;
    let mut defs = BTreeMap::new();
    if let Some(found) = schema.get("$defs") {
        let pointer = root_pointer.join("$defs");
        let Some(entries) = found.as_object() else {
            return Err(invalid(&pointer, "`$defs` is an object of schemas"));
        };
        for (name, definition) in entries {
            let at = pointer.join(name);
            if definition.get("$ref").is_some() {
                return Err(invalid(&at, "a definition is not itself a reference"));
            }
            let index = loader.rules(definition, &at, 0, false)?;
            defs.insert(name.clone(), index);
        }
    }
    for (node, name, pointer) in core::mem::take(&mut loader.refs) {
        let Some(&target) = defs.get(&name) else {
            return Err(invalid(&pointer, "no definition of this name in `$defs`"));
        };
        if let Some(slot) = loader.nodes.get_mut(node) {
            *slot = Node::Ref(target);
        }
    }
    let mut states = vec![Measure::New; loader.nodes.len()];
    loader.measure(root, 0, &mut states)?;
    for index in defs.into_values() {
        loader.measure(index, 0, &mut states)?;
    }
    Ok(Compiled {
        nodes: loader.nodes,
        conditions: loader.conditions,
        root,
    })
}

fn invalid(pointer: &Pointer, problem: &'static str) -> LoadError {
    LoadError::Invalid {
        pointer: pointer.clone(),
        problem,
    }
}

/// The object of a subschema, or the refusal of a value that is not one.
fn object<'a>(value: &'a Value, pointer: &Pointer) -> Result<&'a Map<String, Value>, LoadError> {
    value
        .as_object()
        .ok_or_else(|| invalid(pointer, "a schema is an object"))
}

/// Refuses the first keyword of `map`, in byte order, that is neither an annotation nor one of
/// `allowed`.
fn only(map: &Map<String, Value>, allowed: &[&str], pointer: &Pointer) -> Result<(), LoadError> {
    match map
        .keys()
        .find(|k| !ANNOTATIONS.contains(&k.as_str()) && !allowed.contains(&k.as_str()))
    {
        Some(keyword) => Err(LoadError::Unsupported {
            pointer: pointer.clone(),
            keyword: keyword.clone(),
        }),
        None => Ok(()),
    }
}

/// A non-negative integer bound.
fn count(
    map: &Map<String, Value>,
    keyword: &str,
    pointer: &Pointer,
) -> Result<Option<u64>, LoadError> {
    map.get(keyword)
        .map(|v| {
            v.as_u64()
                .ok_or_else(|| invalid(&pointer.join(keyword), "a bound is a non-negative integer"))
        })
        .transpose()
}

/// An integer bound of a number.
fn bound(
    map: &Map<String, Value>,
    keyword: &str,
    pointer: &Pointer,
) -> Result<Option<i64>, LoadError> {
    map.get(keyword)
        .map(|v| {
            v.as_i64().filter(|n| is_safe((*n).into())).ok_or_else(|| {
                invalid(
                    &pointer.join(keyword),
                    "a numeric bound is an integer within plus or minus 2^53 - 1",
                )
            })
        })
        .transpose()
}

/// Refuses `upper` below `lower`.
fn ordered<T: PartialOrd>(
    lower: Option<T>,
    upper: Option<T>,
    at: &Pointer,
) -> Result<(), LoadError> {
    match (lower, upper) {
        (Some(low), Some(high)) if high < low => {
            Err(invalid(at, "the upper bound is below the lower bound"))
        }
        _ => Ok(()),
    }
}

/// `enum` or `const`, each value a string, an integer within plus or minus 2^53 - 1, a
/// boolean or `null`.
fn values(map: &Map<String, Value>, pointer: &Pointer) -> Result<Option<Values>, LoadError> {
    let scalar = |v: &Value| {
        v.is_string()
            || v.is_boolean()
            || v.is_null()
            || v.as_i64().is_some_and(|n| is_safe(n.into()))
    };
    match (map.get("enum"), map.get("const")) {
        (Some(_), Some(_)) => Err(invalid(
            &pointer.join("const"),
            "`enum` and `const` are not used together",
        )),
        (Some(list), None) => {
            let at = pointer.join("enum");
            let Some(list) = list.as_array().filter(|l| !l.is_empty()) else {
                return Err(invalid(&at, "`enum` is a list of at least one value"));
            };
            if let Some(i) = list.iter().position(|v| !scalar(v)) {
                return Err(invalid(
                    &at.join_index(i),
                    "a value of `enum` is a string, a safe integer, a boolean or null",
                ));
            }
            Ok(Some(Values {
                list: list.clone(),
                is_const: false,
            }))
        }
        (None, Some(value)) => {
            if !scalar(value) {
                return Err(invalid(
                    &pointer.join("const"),
                    "`const` is a string, a safe integer, a boolean or null",
                ));
            }
            Ok(Some(Values {
                list: vec![value.clone()],
                is_const: true,
            }))
        }
        (None, None) => Ok(None),
    }
}

/// A list of distinct member names, each one of `declared` when it is given.
fn names(
    value: &Value,
    at: &Pointer,
    declared: Option<&BTreeMap<String, usize>>,
) -> Result<Vec<String>, LoadError> {
    let Some(list) = value.as_array() else {
        return Err(invalid(at, "`required` is a list of member names"));
    };
    let mut seen = BTreeSet::new();
    let mut out = Vec::with_capacity(list.len());
    for (i, item) in list.iter().enumerate() {
        let here = at.join_index(i);
        let Some(name) = item.as_str() else {
            return Err(invalid(&here, "a member name is a string"));
        };
        if declared.is_some_and(|d| !d.contains_key(name)) {
            return Err(invalid(&here, "the member is not declared in `properties`"));
        }
        if !seen.insert(name) {
            return Err(invalid(&here, "the member is listed twice"));
        }
        out.push(name.to_owned());
    }
    Ok(out)
}

impl Loader {
    /// Compiles a schema that validates a value, at `depth` levels below its tree's root.
    fn rules(
        &mut self,
        value: &Value,
        pointer: &Pointer,
        depth: usize,
        root: bool,
    ) -> Result<usize, LoadError> {
        if depth > MAX_DEPTH {
            return Err(LoadError::TooDeep {
                pointer: pointer.clone(),
            });
        }
        let map = object(value, pointer)?;
        only(map, &RULE_KEYWORDS, pointer)?;
        for keyword in ["$schema", "$id", "$defs"] {
            if !root && map.contains_key(keyword) {
                return Err(invalid(
                    &pointer.join(keyword),
                    "this keyword is accepted at the root only",
                ));
            }
        }
        if let Some(dialect) = map.get("$schema") {
            if dialect.as_str() != Some(DIALECT) {
                return Err(invalid(
                    &pointer.join("$schema"),
                    "the dialect is JSON Schema draft 2020-12",
                ));
            }
        }
        if map.get("$id").is_some_and(|id| !id.is_string()) {
            return Err(invalid(&pointer.join("$id"), "`$id` is a string"));
        }
        let index = self.nodes.len();
        self.pointers.push(pointer.clone());
        if let Some(target) = map.get("$ref") {
            let beside = map.keys().any(|k| {
                !ANNOTATIONS.contains(&k.as_str())
                    && !["$ref", "$schema", "$id", "$defs"].contains(&k.as_str())
            });
            if beside {
                return Err(invalid(pointer, "a reference takes no other keyword"));
            }
            let at = pointer.join("$ref");
            let name = target
                .as_str()
                .and_then(|t| t.strip_prefix("#/$defs/"))
                .filter(|n| !n.is_empty() && !n.contains(['/', '~']));
            let Some(name) = name else {
                return Err(invalid(&at, "a reference is `#/$defs/<name>`"));
            };
            self.refs.push((index, name.to_owned(), at));
            // Resolved when every definition is compiled.
            self.nodes.push(Node::Ref(index));
            return Ok(index);
        }
        self.nodes.push(Node::Rules(Box::default()));
        let rules = self.compile_rules(map, pointer, depth)?;
        if let Some(slot) = self.nodes.get_mut(index) {
            *slot = Node::Rules(Box::new(rules));
        }
        Ok(index)
    }

    fn compile_rules(
        &mut self,
        map: &Map<String, Value>,
        pointer: &Pointer,
        depth: usize,
    ) -> Result<Rules, LoadError> {
        let mut rules = Rules::default();
        if let Some(kind) = map.get("type") {
            rules.kind = Some(kind.as_str().and_then(Type::named).ok_or_else(|| {
                invalid(
                    &pointer.join("type"),
                    "the type is one of null, boolean, integer, string, array, object",
                )
            })?);
        }
        rules.values = values(map, pointer)?;
        rules.min_length = count(map, "minLength", pointer)?;
        rules.max_length = count(map, "maxLength", pointer)?;
        ordered(
            rules.min_length,
            rules.max_length,
            &pointer.join("maxLength"),
        )?;
        rules.min_items = count(map, "minItems", pointer)?;
        rules.max_items = count(map, "maxItems", pointer)?;
        ordered(rules.min_items, rules.max_items, &pointer.join("maxItems"))?;
        rules.minimum = bound(map, "minimum", pointer)?;
        rules.maximum = bound(map, "maximum", pointer)?;
        for keyword in ["minimum", "maximum"] {
            if map.contains_key(keyword) && rules.kind != Some(Type::Integer) {
                return Err(invalid(
                    &pointer.join(keyword),
                    "a numeric bound needs `type: integer`",
                ));
            }
        }
        ordered(rules.minimum, rules.maximum, &pointer.join("maximum"))?;
        if let Some(name) = map.get("format") {
            rules.format = Some(name.as_str().and_then(Format::named).ok_or_else(|| {
                invalid(
                    &pointer.join("format"),
                    "the format is not one of the named formats of the subset",
                )
            })?);
        }
        let next = depth.saturating_add(1);
        if let Some(items) = map.get("items") {
            rules.items = Some(self.rules(items, &pointer.join("items"), next, false)?);
        }
        match map.get("additionalProperties") {
            None => {}
            Some(Value::Bool(false)) => rules.closed = true,
            Some(_) => {
                return Err(invalid(
                    &pointer.join("additionalProperties"),
                    "`additionalProperties` is only `false`",
                ));
            }
        }
        if let Some(properties) = map.get("properties") {
            let at = pointer.join("properties");
            let Some(entries) = properties.as_object() else {
                return Err(invalid(&at, "`properties` is an object of schemas"));
            };
            if !rules.closed {
                return Err(invalid(
                    pointer,
                    "`properties` needs `additionalProperties: false`",
                ));
            }
            for (name, schema) in entries {
                let child = self.rules(schema, &at.join(name), next, false)?;
                rules.properties.insert(name.clone(), child);
            }
        }
        if let Some(required) = map.get("required") {
            rules.required = names(required, &pointer.join("required"), Some(&rules.properties))?;
        }
        if map.contains_key("if") || map.contains_key("then") {
            let pair = self.conditional(map, pointer, next, &rules.properties)?;
            rules.conditionals.push(pair);
        }
        if let Some(all_of) = map.get("allOf") {
            let at = pointer.join("allOf");
            let Some(entries) = all_of.as_array().filter(|e| !e.is_empty()) else {
                return Err(invalid(
                    &at,
                    "`allOf` is a list of at least one `if` and `then` pair",
                ));
            };
            for (i, entry) in entries.iter().enumerate() {
                let here = at.join_index(i);
                let entry = object(entry, &here)?;
                only(entry, &ALL_OF_KEYWORDS, &here)?;
                let pair = self.conditional(entry, &here, next, &rules.properties)?;
                rules.conditionals.push(pair);
            }
        }
        Ok(rules)
    }

    /// The `if` and `then` of `map`, whose members are among `declared`.
    fn conditional(
        &mut self,
        map: &Map<String, Value>,
        pointer: &Pointer,
        depth: usize,
        declared: &BTreeMap<String, usize>,
    ) -> Result<Conditional, LoadError> {
        let (Some(when), Some(then)) = (map.get("if"), map.get("then")) else {
            return Err(invalid(pointer, "`if` and `then` come together"));
        };
        let if_at = pointer.join("if");
        let condition = object(when, &if_at)?;
        if let Some(properties) = condition.get("properties").and_then(Value::as_object) {
            let at = if_at.join("properties");
            if let Some(name) = properties.keys().find(|k| !declared.contains_key(*k)) {
                return Err(invalid(
                    &at.join(name),
                    "the member is not declared in `properties`",
                ));
            }
        }
        if let Some(required) = condition.get("required") {
            names(required, &if_at.join("required"), Some(declared))?;
        }
        let when = self.condition(when, &if_at, depth)?;
        let then_at = pointer.join("then");
        let then = object(then, &then_at)?;
        only(then, &THEN_KEYWORDS, &then_at)?;
        let required = match then.get("required") {
            Some(list) => names(list, &then_at.join("required"), Some(declared))?,
            None => Vec::new(),
        };
        Ok(Conditional {
            when,
            then: required,
        })
    }

    /// Compiles a condition; returns its index in the arena of conditions.
    fn condition(
        &mut self,
        value: &Value,
        pointer: &Pointer,
        depth: usize,
    ) -> Result<usize, LoadError> {
        if depth > MAX_DEPTH {
            return Err(LoadError::TooDeep {
                pointer: pointer.clone(),
            });
        }
        let map = object(value, pointer)?;
        only(map, &CONDITION_KEYWORDS, pointer)?;
        let next = depth.saturating_add(1);
        let mut condition = Condition {
            values: values(map, pointer)?,
            ..Condition::default()
        };
        let mut height = 0;
        if let Some(properties) = map.get("properties") {
            let at = pointer.join("properties");
            let Some(entries) = properties.as_object() else {
                return Err(invalid(&at, "`properties` is an object of schemas"));
            };
            for (name, schema) in entries {
                let child = self.condition(schema, &at.join(name), next)?;
                height = height.max(self.condition_height(child).saturating_add(1));
                condition.properties.insert(name.clone(), child);
            }
        }
        if let Some(required) = map.get("required") {
            condition.required = names(required, &pointer.join("required"), None)?;
        }
        if let Some(contains) = map.get("contains") {
            let child = self.condition(contains, &pointer.join("contains"), next)?;
            height = height.max(self.condition_height(child).saturating_add(1));
            condition.contains = Some(child);
        }
        let index = self.conditions.len();
        self.conditions.push(condition);
        self.condition_heights.push(height);
        self.condition_pointers.push(pointer.clone());
        Ok(index)
    }

    fn condition_height(&self, index: usize) -> usize {
        self.condition_heights.get(index).copied().unwrap_or(0)
    }

    fn pointer(&self, node: usize) -> Pointer {
        self.pointers.get(node).cloned().unwrap_or_default()
    }

    /// The height of `node` (the deepest level of subschemas below it, through references),
    /// reached at `depth`; refuses a reference that reaches itself and a depth beyond the
    /// bound. The recursion stops at the bound, so it is at most [`MAX_DEPTH`] calls deep.
    fn measure(
        &self,
        node: usize,
        depth: usize,
        states: &mut [Measure],
    ) -> Result<usize, LoadError> {
        let too_deep = |pointer: Pointer| LoadError::TooDeep { pointer };
        if depth > MAX_DEPTH {
            return Err(too_deep(self.pointer(node)));
        }
        match states.get(node).copied() {
            Some(Measure::Done(height)) => {
                return if depth.saturating_add(height) > MAX_DEPTH {
                    Err(too_deep(self.pointer(node)))
                } else {
                    Ok(height)
                };
            }
            Some(Measure::Visiting) => {
                return Err(LoadError::Recursive {
                    pointer: self.pointer(node),
                });
            }
            Some(Measure::New) | None => {}
        }
        if let Some(state) = states.get_mut(node) {
            *state = Measure::Visiting;
        }
        let next = depth.saturating_add(1);
        let mut height = 0;
        match self.nodes.get(node) {
            Some(Node::Ref(target)) => {
                height = self.measure(*target, next, states)?.saturating_add(1);
            }
            Some(Node::Rules(rules)) => {
                for &child in rules.properties.values().chain(rules.items.iter()) {
                    height = height.max(self.measure(child, next, states)?.saturating_add(1));
                }
                for conditional in &rules.conditionals {
                    let below = self.condition_height(conditional.when).saturating_add(1);
                    if depth.saturating_add(below) > MAX_DEPTH {
                        let at = self
                            .condition_pointers
                            .get(conditional.when)
                            .cloned()
                            .unwrap_or_default();
                        return Err(too_deep(at));
                    }
                    height = height.max(below);
                }
            }
            None => {}
        }
        if let Some(state) = states.get_mut(node) {
            *state = Measure::Done(height);
        }
        Ok(height)
    }
}
