//! YAML text into a JSON document with the line of each value (plan of SPEC-0001, D1).
//!
//! The parser gives events with their position; this module builds the JSON value from them,
//! so that the rules of the plan hold whatever the parser would otherwise accept:
//!
//! - Scalars follow the YAML 1.2 core schema: only `null`, `~` and the empty scalar are null,
//!   only `true` and `false` (in three casings) are booleans, numbers are decimal, `0o` octal
//!   or `0x` hexadecimal integers and decimal floats. Everything else, `no`, `on`, `yes` and
//!   dates such as `2026-11-17` included, is a string, and a quoted or block scalar is always
//!   a string. The tags `!!str`, `!!null`, `!!bool`, `!!int` and `!!float` are accepted when
//!   the scalar has that type (`!!float 1` is the float 1, as the float of the core schema
//!   matches it), `!!seq` and `!!map` on their collections; any other tag is refused.
//! - A key is a scalar written in place, used as written; a key given twice in one mapping
//!   is refused with its line.
//! - An alias repeats the value of its anchor once that value is complete. Every value counts
//!   towards [`MAX_VALUES`], those an alias repeats included, and nesting is bounded by
//!   [`MAX_NESTING`]. The bytes of the document are counted against a room the caller gives,
//!   at most [`MAX_DOCUMENT_BYTES`](wayloop_core::artifacts::locations::MAX_DOCUMENT_BYTES):
//!   the text of every scalar and key, the pointer of every value given a line, the bytes of
//!   every value an alias repeats, and every copy kept for an anchor. A document beyond a
//!   bound is refused before the value that crosses it is copied: an alias bomb, a large value
//!   repeated by aliases and nested anchors never reach memory.
//! - The file holds one document, a mapping. A number JSON cannot hold (`.inf`, `.nan`, a
//!   value beyond a double) is refused. A message quotes user text, a key or a value, cut to
//!   80 characters.
//!
//! Integers are written as JSON writes them: within the range of a 64-bit integer as such,
//! beyond it as the nearest double, as a JSON parser reads them; the core applies the integer
//! rule of the contracts to the value (plan of SPEC-0001, Numbers).
//!
//! The line of a member of a mapping is the line of its key; of an element of a sequence and
//! of the root, the line where the value starts. A refusal gives the line of the fault and a
//! message for a person, which the core reports as a `syntax` error of the file.
//!
//! The parser is built with [`Parser::new_from_str`], which reads the text in place; its
//! buffered input for iterators, the only user of the `arraydeque` crate, is not used.

use std::collections::BTreeMap;

use saphyr_parser::{Event, Parser, ScalarStyle, Tag};
use serde_json::{Map, Number, Value};
use wayloop_core::Pointer;
use wayloop_core::artifacts::locations::{MAX_NESTING, MAX_VALUES};
use wayloop_schemas::validate::{Lines, ParseError};
use wayloop_schemas::value::Line;

use crate::refusal::{fault, line, quoted};

/// A parsed YAML file: its document, the line of each of its values, and its bytes as counted.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Parsed {
    pub(crate) document: Map<String, Value>,
    pub(crate) lines: Lines,
    /// The bytes the document took, as counted against the room.
    pub(crate) bytes: usize,
}

/// The document of the YAML `text`, with the line of each value, taking at most `room` bytes
/// as counted; or the line and the reason it is refused.
pub(crate) fn parse(text: &str, room: usize) -> Result<Parsed, ParseError> {
    let mut builder = Builder {
        room,
        ..Builder::default()
    };
    let mut parser = Parser::new_from_str(text);
    while let Some(next) = parser.next_event() {
        let (event, span) = next.map_err(|e| fault(e.marker().line(), e.info()))?;
        let at = span.start.line();
        match event {
            Event::DocumentStart(_) => {
                builder.documents += 1;
                if builder.documents > 1 {
                    return Err(fault(at, "the file holds more than one YAML document"));
                }
            }
            Event::Scalar(text, style, anchor, tag) => {
                if builder.awaits_key() {
                    builder.key(text.into_owned(), at, anchor, tag.as_deref())?;
                } else {
                    let value = scalar(&text, style, tag.as_deref(), at)?;
                    builder.value(value, text.len(), at, anchor)?;
                }
            }
            Event::SequenceStart(anchor, tag) => {
                collection_tag(tag.as_deref(), "seq", at)?;
                builder.open(Content::Sequence(Vec::new()), at, anchor)?;
            }
            Event::MappingStart(anchor, tag) => {
                collection_tag(tag.as_deref(), "map", at)?;
                let mapping = Content::Mapping {
                    map: Map::new(),
                    key: None,
                };
                builder.open(mapping, at, anchor)?;
            }
            Event::SequenceEnd | Event::MappingEnd => builder.close()?,
            Event::Alias(anchor) => builder.alias(anchor, at)?,
            Event::Nothing | Event::StreamStart | Event::StreamEnd | Event::DocumentEnd => {}
        }
    }
    builder.finish()
}

/// What a value counts towards the bounds: values and bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Cost {
    values: usize,
    bytes: usize,
}

/// A complete value with an anchor, which an alias repeats.
#[derive(Debug)]
struct Anchored {
    value: Value,
    /// What it counted when it was built: its values, itself included, and its bytes.
    cost: Cost,
    /// How many levels deep it is, itself included.
    nesting: usize,
}

/// What an open collection holds so far.
#[derive(Debug)]
enum Content {
    Sequence(Vec<Value>),
    Mapping {
        map: Map<String, Value>,
        /// The key whose value comes next, with its line.
        key: Option<(String, usize)>,
    },
}

/// A collection being built.
#[derive(Debug)]
struct Frame {
    content: Content,
    pointer: Pointer,
    anchor: usize,
    /// The line where it starts.
    at: usize,
    /// What was counted before it opened.
    before: Cost,
    /// The deepest nesting of the values it holds so far.
    nesting: usize,
}

#[derive(Debug, Default)]
struct Builder {
    frames: Vec<Frame>,
    root: Option<(Value, usize)>,
    lines: BTreeMap<String, Line>,
    anchors: BTreeMap<usize, Anchored>,
    /// Every value and byte counted so far, those repeated by aliases included.
    counted: Cost,
    /// The bytes the document may take.
    room: usize,
    documents: usize,
}

impl Builder {
    /// Whether the next scalar is a key: the innermost collection is a mapping without a key.
    fn awaits_key(&self) -> bool {
        matches!(
            self.frames.last(),
            Some(Frame {
                content: Content::Mapping { key: None, .. },
                ..
            })
        )
    }

    /// Counts `cost` at line `at`, refusing more than [`MAX_VALUES`] values or more bytes than
    /// the room.
    fn count(&mut self, cost: Cost, at: usize) -> Result<(), ParseError> {
        self.counted.values = self.counted.values.saturating_add(cost.values);
        self.counted.bytes = self.counted.bytes.saturating_add(cost.bytes);
        if self.counted.values > MAX_VALUES {
            return Err(fault(
                at,
                format!(
                    "the file holds more than {MAX_VALUES} values, counting those its aliases \
                     repeat"
                ),
            ));
        }
        if self.counted.bytes > self.room {
            return Err(fault(
                at,
                format!(
                    "the document would take more than {} bytes in memory, counting its text, \
                     the pointers of its lines and the values its aliases and anchors repeat",
                    self.room
                ),
            ));
        }
        Ok(())
    }

    /// Refuses a value `nesting` levels deep at the current position beyond [`MAX_NESTING`].
    fn nest(&self, nesting: usize, at: usize) -> Result<(), ParseError> {
        if self.frames.len().saturating_add(nesting) > MAX_NESTING {
            return Err(fault(
                at,
                format!("the values are nested deeper than {MAX_NESTING} levels"),
            ));
        }
        Ok(())
    }

    /// The pointer of the next value, its bytes counted and its line recorded: the line of its
    /// key in a mapping, `at` otherwise. A key cannot be a collection or an alias.
    fn next_pointer(&mut self, at: usize) -> Result<Pointer, ParseError> {
        let (pointer, line_of) = match self.frames.last() {
            None => (Pointer::root(), at),
            Some(frame) => match &frame.content {
                Content::Sequence(items) => (frame.pointer.join_index(items.len()), at),
                Content::Mapping {
                    key: Some((key, key_line)),
                    ..
                } => (frame.pointer.join(key), *key_line),
                Content::Mapping { key: None, .. } => {
                    return Err(fault(
                        at,
                        "a key must be a scalar written in place, not a collection or an alias",
                    ));
                }
            },
        };
        let bytes = pointer.as_str().len();
        self.count(Cost { values: 0, bytes }, at)?;
        self.lines
            .insert(pointer.as_str().to_owned(), line(line_of));
        Ok(pointer)
    }

    /// Sets `value`, `nesting` levels deep, in the innermost collection, or as the root.
    fn attach(&mut self, value: Value, nesting: usize, at: usize) {
        let Some(frame) = self.frames.last_mut() else {
            self.root = Some((value, at));
            return;
        };
        frame.nesting = frame.nesting.max(nesting);
        match &mut frame.content {
            Content::Sequence(items) => items.push(value),
            Content::Mapping { map, key } => {
                if let Some((key, _)) = key.take() {
                    map.insert(key, value);
                }
            }
        }
    }

    /// Keeps a copy of `value`, which counted `cost`, for the aliases of `anchor` when it has
    /// one; the copy counts its bytes again, before it is made.
    fn keep(
        &mut self,
        anchor: usize,
        value: &Value,
        cost: Cost,
        nesting: usize,
        at: usize,
    ) -> Result<(), ParseError> {
        if anchor == 0 {
            return Ok(());
        }
        let copy = Cost {
            values: 0,
            bytes: cost.bytes,
        };
        self.count(copy, at)?;
        let anchored = Anchored {
            value: value.clone(),
            cost,
            nesting,
        };
        self.anchors.insert(anchor, anchored);
        Ok(())
    }

    /// The key `text` of the innermost mapping, at line `at`.
    fn key(
        &mut self,
        text: String,
        at: usize,
        anchor: usize,
        tag: Option<&Tag>,
    ) -> Result<(), ParseError> {
        if tag.is_some_and(|t| !is_core(t, "str")) {
            return Err(unsupported(tag, at));
        }
        let cost = Cost {
            values: 1,
            bytes: text.len(),
        };
        self.count(cost, at)?;
        // The line of the first, when the key is given already; no let chain, for the minimum
        // supported Rust version.
        let given = match self.frames.last() {
            Some(Frame {
                content: Content::Mapping { map, .. },
                pointer,
                ..
            }) if map.contains_key(&text) => Some(
                self.lines
                    .get(pointer.join(&text).as_str())
                    .map_or(0, |l| l.get().get()),
            ),
            _ => None,
        };
        if let Some(first) = given {
            return Err(fault(
                at,
                format!(
                    "the key `{}` is given twice; it was first given at line {first}",
                    quoted(&text)
                ),
            ));
        }
        self.keep(anchor, &Value::String(text.clone()), cost, 1, at)?;
        if let Some(Frame {
            content: Content::Mapping { key, .. },
            ..
        }) = self.frames.last_mut()
        {
            *key = Some((text, at));
        }
        Ok(())
    }

    /// A scalar `value`, written in `bytes` bytes, at line `at`.
    fn value(
        &mut self,
        value: Value,
        bytes: usize,
        at: usize,
        anchor: usize,
    ) -> Result<(), ParseError> {
        let _ = self.next_pointer(at)?;
        let cost = Cost { values: 1, bytes };
        self.count(cost, at)?;
        self.nest(1, at)?;
        self.keep(anchor, &value, cost, 1, at)?;
        self.attach(value, 1, at);
        Ok(())
    }

    /// Opens a collection at line `at`.
    fn open(&mut self, content: Content, at: usize, anchor: usize) -> Result<(), ParseError> {
        let pointer = self.next_pointer(at)?;
        let before = self.counted;
        self.count(
            Cost {
                values: 1,
                bytes: 0,
            },
            at,
        )?;
        self.nest(1, at)?;
        self.frames.push(Frame {
            content,
            pointer,
            anchor,
            at,
            before,
            nesting: 0,
        });
        Ok(())
    }

    /// Closes the innermost collection and sets it in its parent.
    fn close(&mut self) -> Result<(), ParseError> {
        let Some(frame) = self.frames.pop() else {
            return Ok(());
        };
        let value = match frame.content {
            Content::Sequence(items) => Value::Array(items),
            Content::Mapping { map, .. } => Value::Object(map),
        };
        let cost = Cost {
            values: self.counted.values.saturating_sub(frame.before.values),
            bytes: self.counted.bytes.saturating_sub(frame.before.bytes),
        };
        let nesting = frame.nesting.saturating_add(1);
        self.keep(frame.anchor, &value, cost, nesting, frame.at)?;
        self.attach(value, nesting, frame.at);
        Ok(())
    }

    /// An alias of `anchor` at line `at`: the value of the anchor, once it is complete, counted
    /// by its values and its bytes before it is copied.
    fn alias(&mut self, anchor: usize, at: usize) -> Result<(), ParseError> {
        let _ = self.next_pointer(at)?;
        let Some((cost, nesting)) = self.anchors.get(&anchor).map(|a| (a.cost, a.nesting)) else {
            return Err(fault(
                at,
                "an alias refers to an anchor whose value is not complete",
            ));
        };
        self.count(cost, at)?;
        self.nest(nesting, at)?;
        let value = self
            .anchors
            .get(&anchor)
            .map(|a| a.value.clone())
            .unwrap_or_default();
        self.attach(value, nesting, at);
        Ok(())
    }

    /// The document: the root, a mapping.
    fn finish(self) -> Result<Parsed, ParseError> {
        match self.root {
            Some((Value::Object(document), _)) => Ok(Parsed {
                document,
                lines: Lines(self.lines),
                bytes: self.counted.bytes,
            }),
            Some((other, at)) => Err(fault(
                at,
                format!("the document is {}, not a mapping", kind(&other)),
            )),
            None => Err(fault(1, "the file holds no YAML document")),
        }
    }
}

/// The type of a JSON value, for a message.
fn kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "empty",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "a sequence",
        Value::Object(_) => "a mapping",
    }
}

/// Whether `tag` is the tag `!!<suffix>` of the YAML core schema.
fn is_core(tag: &Tag, suffix: &str) -> bool {
    tag.is_yaml_core_schema() && tag.suffix == suffix
}

/// `tag` as written in YAML, cut as a quote: `!!<suffix>` for the core schema, `!<suffix>` for
/// a local tag and the verbatim form `!<...>` otherwise.
fn written(tag: &Tag) -> String {
    let text = if tag.is_yaml_core_schema() {
        format!("!!{}", tag.suffix)
    } else if tag.handle == "!" {
        format!("!{}", tag.suffix)
    } else {
        format!("!<{}{}>", tag.handle, tag.suffix)
    };
    quoted(&text)
}

/// The refusal of a tag the rules do not accept.
fn unsupported(tag: Option<&Tag>, at: usize) -> ParseError {
    let written = tag.map(written).unwrap_or_default();
    fault(at, format!("the tag `{written}` is not supported here"))
}

/// Refuses a tag on a collection other than its own, `!!seq` or `!!map`.
fn collection_tag(tag: Option<&Tag>, own: &str, at: usize) -> Result<(), ParseError> {
    match tag {
        Some(t) if !is_core(t, own) => Err(unsupported(tag, at)),
        _ => Ok(()),
    }
}

/// The value of a plain scalar in the YAML 1.2 core schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Plain {
    Null,
    Bool(bool),
    Int,
    Float,
    /// `.inf` or `.nan`, in any of their spellings, which JSON cannot hold.
    NotFinite,
    Str,
}

impl Plain {
    /// The suffix of the core tag of the type.
    const fn tag(self) -> &'static str {
        match self {
            Plain::Null => "null",
            Plain::Bool(_) => "bool",
            Plain::Int => "int",
            Plain::Float | Plain::NotFinite => "float",
            Plain::Str => "str",
        }
    }
}

/// The JSON value of a scalar `text` written in `style`, with its `tag`, at line `at`.
fn scalar(
    text: &str,
    style: ScalarStyle,
    tag: Option<&Tag>,
    at: usize,
) -> Result<Value, ParseError> {
    let resolved = match (tag, style) {
        (Some(t), _) if is_core(t, "str") => Plain::Str,
        (Some(t), _) if t.is_yaml_core_schema() => {
            // The float of the core schema also matches a decimal integer, such as `1`.
            let resolved = if t.suffix == "float" && is_float(text) {
                Plain::Float
            } else {
                plain(text)
            };
            if resolved.tag() != t.suffix {
                return Err(fault(
                    at,
                    format!(
                        "the value `{}` does not have the type of its tag `{}`",
                        quoted(text),
                        written(t)
                    ),
                ));
            }
            resolved
        }
        (Some(_), _) => return Err(unsupported(tag, at)),
        (None, ScalarStyle::Plain) => plain(text),
        (None, _) => Plain::Str,
    };
    match resolved {
        Plain::Null => Ok(Value::Null),
        Plain::Bool(b) => Ok(Value::Bool(b)),
        Plain::Int => integer(text)
            .map(Value::Number)
            .ok_or_else(|| beyond(text, at)),
        Plain::Float => text
            .parse::<f64>()
            .ok()
            .and_then(Number::from_f64)
            .map(Value::Number)
            .ok_or_else(|| beyond(text, at)),
        Plain::NotFinite => Err(beyond(text, at)),
        Plain::Str => Ok(Value::String(text.to_owned())),
    }
}

/// The refusal of a number JSON cannot hold.
fn beyond(text: &str, at: usize) -> ParseError {
    fault(
        at,
        format!(
            "the number `{}` has no value in JSON, which holds finite doubles only",
            quoted(text)
        ),
    )
}

/// The type of the plain scalar `text` in the YAML 1.2 core schema (YAML 1.2.2, 10.3.2).
fn plain(text: &str) -> Plain {
    match text {
        "" | "~" | "null" | "Null" | "NULL" => Plain::Null,
        "true" | "True" | "TRUE" => Plain::Bool(true),
        "false" | "False" | "FALSE" => Plain::Bool(false),
        ".nan" | ".NaN" | ".NAN" => Plain::NotFinite,
        _ if is_integer(text) => Plain::Int,
        _ if matches!(unsigned(text), ".inf" | ".Inf" | ".INF") => Plain::NotFinite,
        _ if is_float(text) => Plain::Float,
        _ => Plain::Str,
    }
}

/// `text` without one leading sign.
fn unsigned(text: &str) -> &str {
    text.strip_prefix(['-', '+']).unwrap_or(text)
}

/// One or more ASCII digits of `radix`.
fn digits(text: &str, radix: u32) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_digit(radix))
}

/// `[-+]?[0-9]+`, `0o[0-7]+` or `0x[0-9a-fA-F]+`.
fn is_integer(text: &str) -> bool {
    if let Some(octal) = text.strip_prefix("0o") {
        return digits(octal, 8);
    }
    if let Some(hex) = text.strip_prefix("0x") {
        return digits(hex, 16);
    }
    digits(unsigned(text), 10)
}

/// `[-+]?(\.[0-9]+|[0-9]+(\.[0-9]*)?)([eE][-+]?[0-9]+)?`.
fn is_float(text: &str) -> bool {
    let (mantissa, exponent) = match unsigned(text).split_once(['e', 'E']) {
        Some((mantissa, exponent)) => (mantissa, Some(exponent)),
        None => (unsigned(text), None),
    };
    let mantissa = match mantissa.split_once('.') {
        Some(("", fraction)) => digits(fraction, 10),
        Some((whole, fraction)) => {
            digits(whole, 10) && fraction.chars().all(|c| c.is_ascii_digit())
        }
        None => digits(mantissa, 10),
    };
    mantissa && exponent.is_none_or(|e| digits(unsigned(e), 10))
}

/// The JSON number of an integer of the core schema: a 64-bit integer when it fits, the
/// nearest double otherwise; `None` beyond the range of a double.
fn integer(text: &str) -> Option<Number> {
    let (digits, radix) = if let Some(octal) = text.strip_prefix("0o") {
        (octal, 8)
    } else if let Some(hex) = text.strip_prefix("0x") {
        (hex, 16)
    } else {
        if let Ok(n) = text.parse::<i64>() {
            return Some(Number::from(n));
        }
        if let Ok(n) = text.parse::<u64>() {
            return Some(Number::from(n));
        }
        return text.parse::<f64>().ok().and_then(Number::from_f64);
    };
    if let Ok(n) = u64::from_str_radix(digits, radix) {
        return Some(Number::from(n));
    }
    let value = digits.chars().try_fold(0.0_f64, |value, c| {
        Some(value * f64::from(radix) + f64::from(c.to_digit(radix)?))
    })?;
    Number::from_f64(value)
}

#[cfg(test)]
pub(crate) mod tests;
