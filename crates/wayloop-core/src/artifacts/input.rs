//! The canonical input of `validate`, version 1, after it passed the input schema of the
//! command (plan of SPEC-0001, Canonical input of `wayloop validate`).
//!
//! The schema cannot state every rule of the input. [`rules`] refuses, on the JSON value and
//! with the code `command_input`, a member of an entry that belongs to another format
//! (`conflict`), an `origin` with no member (`missing`) or more than one (`conflict`), `lines`
//! without a file origin (`requires`, or `conflict` with another origin), an `unreadable` entry
//! without a file origin (the same reasons: only a file can fail to parse), a key of `lines`
//! that is not an RFC 6901 pointer (`syntax`) and a line that is not an integer from 1
//! (`type`, `range`). Every error is found in one pass, entry by entry.
//!
//! An input that passed the schema and these rules is then read into its contract type,
//! [`ValidateInput`]: a failure there is a defect of the engine (ADR 0030 G1).

use core::num::NonZeroU64;
use std::collections::BTreeMap;

use serde::Deserialize as _;
use serde_json::{Map, Value};
use wayloop_schemas::validate::{DocumentEntry, EntryFormat, InputKind, Origin, ValidateInput};

use super::kinds::internal;
use crate::command::{COMMAND_INPUT, INTERNAL_ERROR};
use crate::{Error, Pointer, Reason, Source};
use wayloop_schemas::number::{integer, is_safe};

/// The set of `validate`, as the checks read it.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Input {
    pub(crate) documents: Vec<Entry>,
}

/// Where a document was read: the one member of its origin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Place {
    File(String),
    Issue(NonZeroU64),
    Milestone(NonZeroU64),
}

/// One entry of `documents`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Entry {
    /// The origin as given, written back in the summary of the document.
    pub(crate) given: Option<Origin>,
    /// The one member of the origin.
    pub(crate) origin: Option<Place>,
    pub(crate) body: Body,
    /// The line of each value of the document, by its pointer as written.
    pub(crate) lines: BTreeMap<String, NonZeroU64>,
}

/// What an entry holds, by its `format`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Body {
    /// A document in its normalised form; `input_kind` for an input.
    Document {
        document: Map<String, Value>,
        input_kind: Option<InputKind>,
    },
    /// The text of a living specification.
    Markdown,
    /// A file that did not parse, with the line and the message of the parser.
    Unreadable { line: NonZeroU64, message: String },
}

/// The members an entry of each format may give.
fn allowed(format: Option<EntryFormat>) -> &'static [&'static str] {
    match format {
        Some(EntryFormat::Document) => &["document", "format", "input_kind", "lines", "origin"],
        Some(EntryFormat::Markdown) => &["format", "origin", "text"],
        Some(EntryFormat::Unreadable) | None => &["error", "format", "origin"],
    }
}

/// Every error of the rules of `input`, which passed the input schema of the command, that its
/// schema cannot state, with the code `command_input`; the input is refused when there is one.
pub(crate) fn rules(input: &Value) -> Vec<Error> {
    let entries = input
        .get("documents")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let mut errors = Vec::new();
    let at = Pointer::root().join("documents");
    for (i, entry) in entries.iter().enumerate() {
        if let Some(members) = entry.as_object() {
            entry_rules(members, &at.join_index(i), &mut errors);
        }
    }
    errors
}

/// `input`, which passed its schema and [`rules`], read into its contract type and then into
/// the entries the checks read; an internal error when it does not read (ADR 0030 G1).
pub(crate) fn read(input: &Value) -> Result<Input, Box<Error>> {
    let typed = ValidateInput::deserialize(input).map_err(|e| {
        Box::new(internal(
            INTERNAL_ERROR,
            format!("An input the schema accepts does not read into its type: {e}."),
        ))
    })?;
    Ok(Input {
        documents: typed.documents.into_iter().map(entry).collect(),
    })
}

/// The entry the checks read from an entry of the contract.
fn entry(entry: DocumentEntry) -> Entry {
    let given = entry.origin;
    let origin = given.clone().and_then(place);
    let lines = entry
        .lines
        .map(|lines| lines.0.into_iter().map(|(k, v)| (k, v.get())).collect())
        .unwrap_or_default();
    let body = match (entry.format, entry.error) {
        (EntryFormat::Document, _) => Body::Document {
            document: entry.document.map(|d| d.0).unwrap_or_default(),
            input_kind: entry.input_kind,
        },
        (EntryFormat::Markdown, _) => Body::Markdown,
        (EntryFormat::Unreadable, error) => {
            let (line, message) = error.map_or((NonZeroU64::MIN, String::new()), |e| {
                (e.line.get(), e.message)
            });
            Body::Unreadable { line, message }
        }
    };
    Entry {
        given,
        origin,
        body,
        lines,
    }
}

/// The one member of an origin, which [`rules`] has checked.
fn place(origin: Origin) -> Option<Place> {
    match (origin.file, origin.issue, origin.milestone) {
        (Some(file), None, None) => Some(Place::File(file.as_str().to_owned())),
        (None, Some(issue), None) => Some(Place::Issue(issue.get())),
        (None, None, Some(milestone)) => Some(Place::Milestone(milestone.get())),
        _ => None,
    }
}

fn entry_rules(members: &Map<String, Value>, at: &Pointer, errors: &mut Vec<Error>) {
    let given = |key: &str| members.get(key).filter(|v| !v.is_null());
    let spelling = given("format").and_then(Value::as_str).unwrap_or_default();
    let format = EntryFormat::from_id(spelling);
    let allowed = allowed(format);
    for key in members
        .iter()
        .filter(|(k, v)| !v.is_null() && !allowed.contains(&k.as_str()))
        .map(|(k, _)| k)
    {
        let message = format!("The member `{key}` does not belong to the format `{spelling}`.");
        errors.push(
            Error::new(COMMAND_INPUT, message, at.join(key), Reason::Conflict)
                .with_pointers([at.join("format")]),
        );
    }
    let origin = given("origin").map(|o| origin_rules(o, &at.join("origin"), errors));
    // In an entry of another format, `lines` is refused above as a conflict, once.
    if let Some(Value::Object(lines)) =
        given("lines").filter(|_| format == Some(EntryFormat::Document))
    {
        needs_file(origin, at, "lines", errors);
        line_rules(lines, &at.join("lines"), errors);
    }
    if format == Some(EntryFormat::Unreadable) {
        needs_file(origin, at, "format", errors);
    }
}

/// What the origin of an entry is, after its rule: a file, another place, or refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Given {
    File,
    Other,
    Refused,
}

/// Refuses the member `member` of the entry at `at` unless the entry has a file origin:
/// `requires` at the origin when it is absent, `conflict` when it is an issue or a milestone.
/// An origin refused already is not reported again.
fn needs_file(origin: Option<Given>, at: &Pointer, member: &str, errors: &mut Vec<Error>) {
    let (message, reason) = match origin {
        Some(Given::File | Given::Refused) => return,
        Some(Given::Other) => (
            format!("`{member}` needs a document read from a file."),
            Reason::Conflict,
        ),
        None => (
            format!("`{member}` needs the file the document was read from."),
            Reason::Requires,
        ),
    };
    errors.push(
        Error::new(COMMAND_INPUT, message, at.join("origin"), reason)
            .with_pointers([at.join(member)]),
    );
}

/// Refuses an origin with no member or more than one.
fn origin_rules(origin: &Value, at: &Pointer, errors: &mut Vec<Error>) -> Given {
    let members: Vec<&String> = origin
        .as_object()
        .map(|m| {
            m.iter()
                .filter(|(_, v)| !v.is_null())
                .map(|(k, _)| k)
                .collect()
        })
        .unwrap_or_default();
    match members.as_slice() {
        [] => {
            let message = "The origin gives none of `file`, `issue` and `milestone`.".to_owned();
            errors.push(Error::new(
                COMMAND_INPUT,
                message,
                at.clone(),
                Reason::Missing,
            ));
            Given::Refused
        }
        [key] if key.as_str() == "file" => Given::File,
        [_] => Given::Other,
        several => {
            let message =
                "The origin gives more than one of `file`, `issue` and `milestone`.".to_owned();
            errors.push(
                Error::new(COMMAND_INPUT, message, at.clone(), Reason::Conflict)
                    .with_pointers(several.iter().map(|key| at.join(key))),
            );
            Given::Refused
        }
    }
}

/// Refuses a key of `lines` that is not a pointer and a line that is not an integer from 1.
fn line_rules(lines: &Map<String, Value>, at: &Pointer, errors: &mut Vec<Error>) {
    for (key, value) in lines.iter().filter(|(_, v)| !v.is_null()) {
        let here = at.join(key);
        if Pointer::parse(key).is_none() {
            let message = format!("The key `{key}` is not an RFC 6901 pointer.");
            errors.push(Error::new(COMMAND_INPUT, message, here, Reason::Syntax));
            continue;
        }
        let Some(n) = integer(value) else {
            let message = "Expected an integer, a line number from 1.".to_owned();
            errors.push(Error::new(COMMAND_INPUT, message, here, Reason::Type));
            continue;
        };
        if n < 1 || !is_safe(n) {
            let message = format!("The line {value} is not a line number from 1.");
            errors.push(Error::new(COMMAND_INPUT, message, here, Reason::Range));
        }
    }
}

impl Entry {
    /// Where the value at `pointer` of the document is stored (SPEC-0001 R12): for a file, the
    /// line given for the pointer or for the nearest value that contains it, and no line when
    /// none is given, never a guess; the issue or the milestone otherwise; `None` for a
    /// document sent directly.
    pub(crate) fn source(&self, pointer: &Pointer) -> Option<Source> {
        Some(match self.origin.as_ref()? {
            Place::File(file) => Source::File {
                file: file.clone(),
                line: self.line_of(pointer.as_str()),
            },
            Place::Issue(issue) => Source::Issue { issue: *issue },
            Place::Milestone(milestone) => Source::Milestone {
                milestone: *milestone,
            },
        })
    }

    fn line_of(&self, pointer: &str) -> Option<NonZeroU64> {
        let mut current = pointer;
        loop {
            if let Some(line) = self.lines.get(current) {
                return Some(*line);
            }
            current = current.rfind('/').and_then(|end| current.get(..end))?;
        }
    }
}
