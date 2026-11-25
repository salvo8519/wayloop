//! The normalisation of a living specification written in Markdown into its document model
//! (SPEC-0001 R6, R12; Markdown form of a living specification), so that it is validated by the
//! schema of the type `specification` like a document sent in normalised form.
//!
//! The text is read line by line, without a Markdown parser: the title `# SPEC-<seq>: <title>`,
//! the header rows of the table below it, and one `##` section after another. A line inside a
//! fenced block (` ``` ` or `~~~`) is never a heading. Headings, header rows and status words are
//! English whatever the language of the project (R6); a section whose heading is not one of
//! [`SECTIONS`], a translated heading included, is kept as prose in `extra_sections`, and a
//! required member whose section is missing is then refused by the schema with the reason
//! `missing`.
//!
//! Every value written gets the line it was read from, so that an error of the schema carries
//! the file and the line in `source`; the root is at line 1. The few faults that have no member
//! in the document model are refused here, with the code `artifact_schema` and their line: a
//! header row that is not one of [`HEADER_ROWS`] or repeats one (`unknown_field`, at the row as
//! written), a section of the template that repeats one already read (`duplicate`, at the member
//! of the section, with the line of the second heading; its content is not read), text outside
//! the title, the header and the sections (`syntax`, at the root), and a verification text with
//! no word of [`VERIFICATION_WORDS`] (`enum`, the words in `expected`). These choices were
//! decided by the maintainer on 2026-11-25 (SPEC-0001, Markdown form of a living
//! specification).
//! The document is otherwise written as read, never corrected: a malformed value is refused by
//! the schema at its member.

use std::collections::{BTreeMap, BTreeSet};

use core::num::NonZeroU64;

use serde_json::{Map, Value};
use wayloop_schemas::Format;
use wayloop_schemas::artifact::{ArtifactKind, RequirementStatus, Verification};
use wayloop_schemas::contract::SPECIFICATION;
use wayloop_schemas::contract::rules::STATUS;
use wayloop_schemas::value::{Line, ProjectPath};
use wayloop_schemas::{DefaultApplied, DefaultReason};

use super::{codes, locations};
use crate::schema::format;
use crate::{Error, Pointer, Reason, Source};

#[cfg(test)]
pub(crate) mod tests;

/// What normalising a Markdown text gave.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Normalised {
    /// The document model, for the schema of the type `specification`.
    pub(crate) document: Map<String, Value>,
    /// The line of each value of the document, by its pointer.
    pub(crate) lines: BTreeMap<Pointer, Line>,
    /// The faults the document model cannot hold, each with its source when the text was read
    /// from a file.
    pub(crate) errors: Vec<Error>,
    /// The members the Markdown form does not carry, `kind` and `schema_version`, with the value
    /// written: defaults of the document (decided by the maintainer on 2026-11-25).
    pub(crate) defaults_applied: Vec<DefaultApplied>,
    /// The members left out of the document because a fault refused here stands for them: the
    /// schema reports nothing more at these pointers, so that one fault gives one error.
    pub(crate) withheld: BTreeSet<Pointer>,
}

/// The reason of a default of a member that the Markdown form of a living specification does not
/// carry, `kind` and `schema_version`: the value is the one the form implies, the type
/// `specification` and the current version of its schema.
pub(crate) const IMPLIED_BY_FORM: DefaultReason = DefaultReason::new("implied_by_form");

/// How the value of a header row is written into its member.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowValue {
    /// As written.
    Text,
    /// In lower case: a status word, `Draft` for `draft`.
    Status,
    /// The first word: `M0` of `M0 · Foundations`.
    FirstWord,
    /// A list, split at each of the characters.
    List(&'static [char]),
}

/// The header rows of a living specification: the label as written, the member of the
/// document model and how its value is written.
const HEADER_ROWS: [(&str, &str, RowValue); 7] = [
    ("Status", STATUS, RowValue::Status),
    ("Milestone", "milestone", RowValue::FirstWord),
    ("Owner", "owner", RowValue::Text),
    ("Related", "related", RowValue::List(&[',', ';'])),
    ("Commit scope", "commit_scope", RowValue::Text),
    ("Superseded by", "superseded_by", RowValue::List(&[','])),
    ("Language", "language", RowValue::Text),
];

/// The sections of the template of a living specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    Problem,
    Scope,
    UserStories,
    Requirements,
    AcceptanceCriteria,
    Interfaces,
    OpenQuestions,
}

/// The members of the document model written from the sections.
const PROBLEM: &str = "problem";
const STORIES: &str = "stories";
const REQUIREMENTS: &str = "requirements";
const ACCEPTANCE_CRITERIA: &str = "acceptance_criteria";
const INTERFACES: &str = "interfaces";
const OPEN_QUESTIONS: &str = "open_questions";

impl Section {
    /// The member a section fills, the first of the two of Scope and User stories: where a
    /// repeated section is refused.
    const fn member(self) -> &'static str {
        match self {
            Section::Problem => PROBLEM,
            Section::Scope => SCOPE_PARTS[0].1,
            Section::UserStories => STORIES,
            Section::Requirements => REQUIREMENTS,
            Section::AcceptanceCriteria => ACCEPTANCE_CRITERIA,
            Section::Interfaces => INTERFACES,
            Section::OpenQuestions => OPEN_QUESTIONS,
        }
    }
}

/// The heading of each section of the template, as written after `## `.
const SECTIONS: [(&str, Section); 7] = [
    ("Problem", Section::Problem),
    ("Scope", Section::Scope),
    ("User stories", Section::UserStories),
    ("Requirements (EARS)", Section::Requirements),
    ("Acceptance criteria", Section::AcceptanceCriteria),
    ("Interfaces", Section::Interfaces),
    ("Open questions", Section::OpenQuestions),
];

/// The words of a verification text and the value each gives, in the order of SPEC-0001; a
/// word of two words matches both, in order. Matched without regard to case.
pub(crate) const VERIFICATION_WORDS: [(&str, Verification); 11] = [
    ("TDD", Verification::Tdd),
    ("BDD", Verification::Bdd),
    ("BDD scenario", Verification::Bdd),
    ("end-to-end", Verification::EndToEnd),
    ("property", Verification::Property),
    ("property test", Verification::Property),
    ("contract", Verification::Contract),
    ("contract test", Verification::Contract),
    ("benchmark", Verification::Benchmark),
    ("judge", Verification::Judge),
    ("none", Verification::None),
];

/// The title line: `# ` and the identifier, a colon and the title.
const TITLE: &str = "# ";
/// A section heading.
const HEADING: &str = "## ";
/// The markers of the paragraphs of the section Scope and their members.
const SCOPE_PARTS: [(&str, &str); 2] = [
    ("**In scope:**", "scope_in"),
    ("**Out of scope:**", "scope_out"),
];
/// The markers of a list item.
const ITEM_MARKERS: [&str; 3] = ["- ", "* ", "+ "];
/// The markers of a fenced block.
const FENCES: [&str; 2] = ["```", "~~~"];
/// The emphasis around the identifier of a part.
const BOLD: &str = "**";
/// The words before the verification and the rubric of an acceptance criterion.
const VERIFICATION: &str = "Verification:";
const RUBRIC: &str = "Rubric:";
/// The checkboxes of an open question, unresolved and resolved.
const CHECKBOXES: [(&str, bool); 3] = [("[ ] ", false), ("[x] ", true), ("[X] ", true)];

/// One line of the text and its number.
#[derive(Debug, Clone, Copy)]
struct Numbered<'a> {
    line: Line,
    text: &'a str,
    /// Inside a fenced block, or a fence itself: never a heading nor an item.
    fenced: bool,
}

/// One entry of a section read as a list: a list item or a paragraph, its lines joined.
#[derive(Debug, Clone)]
struct Entry {
    line: Line,
    text: String,
    /// A list item that may take indented lines after a blank line.
    item: bool,
}

/// The living specification `text`, read from `file` when it was, normalised into its document
/// model, with the line of each value and the faults the model cannot hold.
pub(crate) fn normalise(text: &str, file: Option<&ProjectPath>) -> Normalised {
    let mut out = Writer {
        normalised: Normalised::default(),
        file,
    };
    let lines = numbered(text);
    if let Some(first) = lines.first() {
        out.at(&Pointer::root(), first.line);
    }
    out.implied("kind", ArtifactKind::Specification.id());
    out.implied("schema_version", SPECIFICATION.version.get());
    if let Some(capability) = file.and_then(|f| locations::specification_capability(f.as_str())) {
        out.put("capability", capability, None);
    }
    let start = lines
        .iter()
        .position(|l| !l.fenced && l.text.starts_with(HEADING))
        .unwrap_or(lines.len());
    let (preamble, body) = lines.split_at(start);
    out.preamble(preamble);
    let mut read: Vec<Section> = Vec::new();
    let mut extra: Vec<(Line, Value)> = Vec::new();
    for (heading, content) in sections(body) {
        let title = heading.text.get(HEADING.len()..).unwrap_or_default().trim();
        let known = SECTIONS
            .iter()
            .find(|(name, _)| *name == title)
            .map(|(_, section)| *section);
        if let Some(section) = known.filter(|section| read.contains(section)) {
            let message = format!("The section `{title}` is given more than once.");
            let pointer = Pointer::root().join(section.member());
            let error = Error::new(codes::ARTIFACT_SCHEMA, message, pointer, Reason::Duplicate);
            out.refuse(error, heading.line);
        } else if let Some(section) = known {
            read.push(section);
            out.section(section, heading.line, content);
        } else {
            let mut prose_section = Map::new();
            prose_section.insert("heading".to_owned(), title.into());
            if let Some(text) = prose(content) {
                prose_section.insert("text".to_owned(), text.into());
            }
            extra.push((heading.line, Value::Object(prose_section)));
        }
    }
    if let Some((first, _)) = extra.first() {
        out.list("extra_sections", *first, extra);
    }
    out.normalised
}

/// The lines of `text`, numbered from 1 and marked when inside a fenced block.
fn numbered(text: &str) -> Vec<Numbered<'_>> {
    let mut fence = false;
    core::iter::successors(Some(NonZeroU64::MIN), |n| n.checked_add(1))
        .zip(text.lines())
        .map(|(n, text)| {
            let marker = FENCES.iter().any(|f| text.trim_start().starts_with(f));
            let fenced = fence || marker;
            if marker {
                fence = !fence;
            }
            Numbered {
                line: Line::new(n),
                text,
                fenced,
            }
        })
        .collect()
}

/// The sections of `body`, which starts with a heading: each heading and the lines up to the
/// next one.
fn sections<'a, 'b>(body: &'b [Numbered<'a>]) -> Vec<(Numbered<'a>, &'b [Numbered<'a>])> {
    let mut found = Vec::new();
    let mut rest = body;
    while let Some((heading, after)) = rest.split_first() {
        let end = after
            .iter()
            .position(|l| !l.fenced && l.text.starts_with(HEADING))
            .unwrap_or(after.len());
        let (content, next) = after.split_at(end);
        found.push((*heading, content));
        rest = next;
    }
    found
}

/// The text of `lines` as written, without the blank lines around it; `None` when blank.
fn prose(lines: &[Numbered<'_>]) -> Option<String> {
    let first = lines.iter().position(|l| !l.text.trim().is_empty())?;
    let last = lines.iter().rposition(|l| !l.text.trim().is_empty())?;
    let kept: Vec<&str> = lines.get(first..=last)?.iter().map(|l| l.text).collect();
    Some(kept.join("\n"))
}

/// The entries of `lines` read as a list: each list item and each paragraph, its lines trimmed
/// and joined with a space. A line indented after a blank line continues the item before it.
fn entries(lines: &[Numbered<'_>]) -> Vec<Entry> {
    let mut found: Vec<Entry> = Vec::new();
    let mut open = false;
    let mut gap = false;
    for line in lines {
        let trimmed = line.text.trim();
        if trimmed.is_empty() {
            gap = true;
            continue;
        }
        let item = ITEM_MARKERS
            .iter()
            .find_map(|m| line.text.strip_prefix(m))
            .filter(|_| !line.fenced);
        let indented = line.text.starts_with(char::is_whitespace);
        let last_is_item = found.last().is_some_and(|e| e.item);
        let continues = open && item.is_none() && (!gap || (indented && last_is_item));
        match (continues, found.last_mut()) {
            (true, Some(last)) => {
                last.text.push(' ');
                last.text.push_str(trimmed);
            }
            _ => found.push(Entry {
                line: line.line,
                text: item.unwrap_or(trimmed).trim().to_owned(),
                item: item.is_some(),
            }),
        }
        open = true;
        gap = false;
    }
    found
}

/// `text` split at the first `**...**` that starts it: the label and the text after it.
fn label(text: &str) -> (Option<&str>, &str) {
    text.strip_prefix(BOLD)
        .and_then(|rest| rest.split_once(BOLD))
        .map_or((None, text), |(label, rest)| {
            (Some(label), rest.trim_start())
        })
}

/// `text` without the colon that starts it.
fn after_colon(text: &str) -> &str {
    text.strip_prefix(':').unwrap_or(text).trim()
}

/// The values of the verification words of `text`, in the order written.
fn verification_values(text: &str) -> Vec<Verification> {
    let tokens: Vec<&str> = text
        .split(|c: char| !(c.is_alphanumeric() || c == '-'))
        .filter(|t| !t.is_empty())
        .collect();
    let mut values = Vec::new();
    let mut i = 0;
    while let Some(rest) = tokens.get(i..).filter(|r| !r.is_empty()) {
        let matched = VERIFICATION_WORDS
            .iter()
            .filter_map(|(word, value)| {
                let words: Vec<&str> = word.split(' ').collect();
                let fits = words.len() <= rest.len()
                    && words
                        .iter()
                        .zip(rest)
                        .all(|(w, t)| w.eq_ignore_ascii_case(t));
                fits.then_some((words.len(), *value))
            })
            .max_by_key(|(len, _)| *len);
        match matched {
            Some((len, value)) => {
                values.push(value);
                i += len;
            }
            None => i += 1,
        }
    }
    values
}

/// Writes the document model and the line of each value.
struct Writer<'f> {
    normalised: Normalised,
    file: Option<&'f ProjectPath>,
}

impl Writer<'_> {
    /// Records `line` for the value at `pointer`.
    fn at(&mut self, pointer: &Pointer, line: Line) {
        self.normalised.lines.insert(pointer.clone(), line);
    }

    /// Writes `value` at the member `key` of the root, read at `line` when it was.
    fn put(&mut self, key: &str, value: impl Into<Value>, line: Option<Line>) {
        if let Some(line) = line {
            self.at(&Pointer::root().join(key), line);
        }
        self.normalised
            .document
            .insert(key.to_owned(), value.into());
    }

    /// Writes `value` at the member `key` of the root, which the form does not carry, and lists
    /// it among the defaults applied.
    fn implied(&mut self, key: &str, value: impl Into<Value>) {
        let value = value.into();
        let at = Pointer::root().join(key);
        let message =
            format!("The Markdown form of a living specification does not carry `{key}`.");
        if let Some(default) = DefaultApplied::new(at, value.clone(), IMPLIED_BY_FORM) {
            self.normalised
                .defaults_applied
                .push(default.with_message(message));
        }
        self.put(key, value, None);
    }

    /// Writes the list `values` at the member `key`, read from `line`, each value at its line.
    fn list(&mut self, key: &str, line: Line, values: Vec<(Line, Value)>) {
        let at = Pointer::root().join(key);
        let mut list = Vec::with_capacity(values.len());
        for (i, (value_line, value)) in values.into_iter().enumerate() {
            self.at(&at.join_index(i), value_line);
            list.push(value);
        }
        self.put(key, list, Some(line));
    }

    /// Adds `error`, a fault read at `line`, with its source when the text was read from a file.
    fn refuse(&mut self, error: Error, line: Line) {
        let error = match self.file {
            Some(file) => error.with_location(Source::File {
                file: file.clone(),
                line: Some(line),
            }),
            None => error,
        };
        self.normalised.errors.push(error);
    }

    /// The title and the header rows; any other text is refused.
    fn preamble(&mut self, lines: &[Numbered<'_>]) {
        let mut titled = false;
        let mut rows: Vec<&str> = Vec::new();
        for line in lines {
            let text = line.text.trim();
            if text.is_empty() {
                continue;
            }
            if let Some(title) = line.text.strip_prefix(TITLE).filter(|_| !titled) {
                titled = true;
                self.title(title.trim(), line.line);
            } else if text.starts_with('|') && !line.fenced {
                self.row(text, line.line, &mut rows);
            } else {
                let message = "Text outside the title, the header rows and the sections of a \
                               living specification.";
                let error = Error::new(
                    codes::ARTIFACT_SCHEMA,
                    message,
                    Pointer::root(),
                    Reason::Syntax,
                );
                self.refuse(error, line.line);
            }
        }
    }

    /// The title `SPEC-<seq>: <title>`; without a colon, the whole text is the title.
    fn title(&mut self, text: &str, line: Line) {
        let (id, title) = match text.split_once(':') {
            Some((id, title)) => (Some(id.trim()), title.trim()),
            None => (None, text),
        };
        if let Some(id) = id.filter(|id| !id.is_empty()) {
            self.put("id", id, Some(line));
        }
        if !title.is_empty() {
            self.put("title", title, Some(line));
        }
    }

    /// A header row `| <label> | <value> |`; the head and the separator of the table are
    /// skipped. `rows` holds the labels read already.
    fn row<'a>(&mut self, text: &'a str, line: Line, rows: &mut Vec<&'a str>) {
        let inner = text.trim_matches('|');
        let (label, value) = inner.split_once('|').unwrap_or((inner, ""));
        let (label, value) = (label.trim(), value.trim_end_matches('|').trim());
        let separator = label.chars().all(|c| matches!(c, '-' | ':' | ' '));
        if separator {
            return;
        }
        let known = HEADER_ROWS
            .iter()
            .find(|(name, _, _)| *name == label)
            .filter(|_| !rows.contains(&label));
        let Some((_, key, how)) = known else {
            let message = format!(
                "The header row `{label}` is not a row of a living specification, or repeats one."
            );
            let error = Error::new(
                codes::ARTIFACT_SCHEMA,
                message,
                Pointer::root().join(label),
                Reason::UnknownField,
            );
            self.refuse(error, line);
            return;
        };
        rows.push(label);
        if value.is_empty() {
            return;
        }
        match how {
            RowValue::Text => self.put(key, value, Some(line)),
            RowValue::Status => self.put(key, value.to_lowercase(), Some(line)),
            RowValue::FirstWord => {
                let first = value.split_whitespace().next().unwrap_or_default();
                self.put(key, first, Some(line));
            }
            RowValue::List(separators) => {
                let values = value
                    .split(*separators)
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .map(|v| (line, Value::from(v)))
                    .collect();
                self.list(key, line, values);
            }
        }
    }

    /// The section `section`, whose heading is at `line`.
    fn section(&mut self, section: Section, line: Line, content: &[Numbered<'_>]) {
        let plain = |entries: Vec<Entry>| -> Vec<(Line, Value)> {
            entries
                .into_iter()
                .map(|e| (e.line, Value::from(e.text)))
                .collect()
        };
        match section {
            Section::Problem => self.put(PROBLEM, prose(content).unwrap_or_default(), Some(line)),
            Section::Scope => self.scope(content),
            Section::UserStories => {
                let (stories, prose_stories): (Vec<Entry>, Vec<Entry>) = entries(content)
                    .into_iter()
                    .partition(|e| format::accepts(Format::StoryId, &e.text));
                self.list(STORIES, line, plain(stories));
                self.list("user_stories", line, plain(prose_stories));
            }
            Section::Requirements => {
                let values = parts(content, requirement);
                self.list(REQUIREMENTS, line, values);
            }
            Section::AcceptanceCriteria => {
                let mut values = Vec::new();
                for (i, entry) in entries(content).into_iter().enumerate() {
                    let (value, refused) = criterion(&entry.text);
                    if let Some(error) = refused {
                        let pointer = Pointer::root()
                            .join(ACCEPTANCE_CRITERIA)
                            .join_index(i)
                            .join("verification");
                        self.normalised.withheld.insert(pointer.clone());
                        let error =
                            Error::new(codes::ARTIFACT_SCHEMA, error, pointer, Reason::Enum)
                                .with_expected(VERIFICATION_WORDS.iter().map(|(word, _)| *word));
                        self.refuse(error, entry.line);
                    }
                    values.push((entry.line, value));
                }
                self.list(ACCEPTANCE_CRITERIA, line, values);
            }
            Section::Interfaces => self.list(INTERFACES, line, plain(entries(content))),
            Section::OpenQuestions => {
                let values = parts(content, question);
                self.list(OPEN_QUESTIONS, line, values);
            }
        }
    }

    /// The section Scope: the paragraphs `**In scope:**` and `**Out of scope:**`, each one
    /// entry per list item or the paragraph as one entry, at the line of its marker. Text
    /// before them is refused; a section with neither gives no member, so `scope_in` is then
    /// missing.
    fn scope(&mut self, content: &[Numbered<'_>]) {
        let mut parts: Vec<(&str, Line, Vec<Numbered<'_>>)> = Vec::new();
        for numbered in content {
            let marker = SCOPE_PARTS
                .iter()
                .find_map(|(m, key)| Some((*key, numbered.text.strip_prefix(m)?)));
            match (marker, parts.last_mut()) {
                (Some((key, rest)), _) => parts.push((
                    key,
                    numbered.line,
                    vec![Numbered {
                        text: rest,
                        ..*numbered
                    }],
                )),
                (None, Some((_, _, lines))) => lines.push(*numbered),
                (None, None) if numbered.text.trim().is_empty() => {}
                (None, None) => {
                    let message = "Text of the section Scope outside its paragraphs \
                                   `**In scope:**` and `**Out of scope:**`.";
                    let error = Error::new(
                        codes::ARTIFACT_SCHEMA,
                        message,
                        Pointer::root(),
                        Reason::Syntax,
                    );
                    self.refuse(error, numbered.line);
                }
            }
        }
        for (key, at, lines) in parts {
            let values = entries(&lines)
                .into_iter()
                .map(|e| (e.line, Value::from(e.text)))
                .collect();
            self.list(key, at, values);
        }
    }
}

/// The entries of `content`, each read into a part by `read`, at its line.
fn parts(content: &[Numbered<'_>], read: fn(&str) -> Map<String, Value>) -> Vec<(Line, Value)> {
    entries(content)
        .into_iter()
        .map(|e| (e.line, Value::Object(read(&e.text))))
        .collect()
}

/// A requirement: `**R<k>**: <text>`, `**R<k>** *<name>*: <text>` or
/// `**R<k>** (removed): <text>`; without its label, the whole item is its text.
fn requirement(item: &str) -> Map<String, Value> {
    let mut part = Map::new();
    let (id, mut rest) = label(item);
    if let Some(id) = id {
        part.insert("id".to_owned(), id.into());
    }
    if let Some((name, after)) = rest
        .strip_prefix('*')
        .and_then(|r| r.split_once('*'))
        .filter(|_| id.is_some())
    {
        part.insert("name".to_owned(), name.into());
        rest = after.trim_start();
    }
    let removed = format!("({})", RequirementStatus::Removed.id());
    if let Some(after) = rest.strip_prefix(&removed).filter(|_| id.is_some()) {
        part.insert(STATUS.to_owned(), RequirementStatus::Removed.id().into());
        rest = after;
    }
    part.insert("text".to_owned(), after_colon(rest).into());
    part
}

/// An acceptance criterion `**AC<k>** (R<i>, R<j>): <text> Verification: <words>. Rubric:
/// <text>`, and the message of its refusal when the verification text has no mapped word.
fn criterion(item: &str) -> (Value, Option<String>) {
    let mut part = Map::new();
    let (id, mut rest) = label(item);
    if let Some(id) = id {
        part.insert("id".to_owned(), id.into());
    }
    if let Some((cited, after)) = rest.strip_prefix('(').and_then(|r| r.split_once(')')) {
        let cited: Vec<Value> = cited
            .split(',')
            .map(str::trim)
            .filter(|r| !r.is_empty())
            .map(Value::from)
            .collect();
        part.insert("requirements".to_owned(), cited.into());
        rest = after;
    }
    let rest = after_colon(rest);
    let mut refused = None;
    let text = match rest.split_once(VERIFICATION) {
        None => rest.to_owned(),
        Some((text, after)) => {
            let (words, tail) = after.split_once('.').unwrap_or((after, ""));
            let values = verification_values(words);
            if values.is_empty() {
                refused = Some(format!(
                    "The verification text `{}` names no verification technique.",
                    words.trim()
                ));
            } else {
                let values: Vec<Value> = values.iter().map(|v| v.id().into()).collect();
                part.insert("verification".to_owned(), values.into());
            }
            let tail = tail.trim();
            match tail.strip_prefix(RUBRIC) {
                Some(rubric) => {
                    part.insert("rubric".to_owned(), rubric.trim().into());
                    text.trim().to_owned()
                }
                // Text after the verification is not dropped: it stays in the criterion.
                None if !tail.is_empty() => format!("{} {tail}", text.trim()),
                None => text.trim().to_owned(),
            }
        }
    };
    part.insert("text".to_owned(), text.into());
    (Value::Object(part), refused)
}

/// An open question `[ ] <text>` or `[x] <text>`, its text optionally starting with
/// `**Q<k>. <question>**`.
fn question(item: &str) -> Map<String, Value> {
    let mut part = Map::new();
    let checked = CHECKBOXES
        .iter()
        .find_map(|(box_, resolved)| Some((*resolved, item.strip_prefix(box_)?)));
    let rest = match checked {
        Some((resolved, rest)) => {
            part.insert("resolved".to_owned(), resolved.into());
            rest.trim()
        }
        None => item,
    };
    let identified = match label(rest) {
        (Some(bold), after) => bold
            .split_once('.')
            .filter(|(id, _)| format::accepts(Format::QuestionId, id))
            .map(|(id, question)| (id, question.trim(), after)),
        (None, _) => None,
    };
    let text = match identified {
        Some((id, question, after)) => {
            part.insert("id".to_owned(), id.into());
            [question, after]
                .iter()
                .filter(|t| !t.is_empty())
                .copied()
                .collect::<Vec<_>>()
                .join(" ")
        }
        None => rest.to_owned(),
    };
    part.insert("text".to_owned(), text.into());
    part
}
