//! Commit headers and pull request titles (SPEC-0001 R11; Naming grammar, Commit header and
//! pull request title).
//!
//! [`check`] reads a header in the grammar
//! `<type>(<scope>)[!]: <subject>[ [SPEC-<seq>[ R<k>[, R<k>]*]]]`: `<type>` is a type of the
//! policy; `<scope>` is a scope of the policy, required except for `revert`; `<subject>` starts
//! as [`starts_well`] tells and, without its trailing white space, has no final full stop; the
//! header is at most [`HEADER_MAX_CHARS`] characters, reference included. Each fault is one
//! error with the code `commit_header` at the pointer of the header. [`has_breaking_change`] and
//! [`requires`] give the rule of the breaking mark `!` of a commit, whose body then needs a
//! `BREAKING CHANGE:` paragraph or footer. [`is_generated`] tells the headers that git writes,
//! which are exempt.
//!
//! Every rule reads the header once, or a bounded part of it, so the cost grows with its length
//! and never with its square.

use wayloop_schemas::Format;

use super::Names;
use crate::artifacts::codes;
use crate::schema::format::accepts;
use crate::{Error, Pointer, Reason};

/// The most characters of a header, reference included: Unicode scalar values, as
/// `scripts/conventions.py` counts them. A deliberate copy of `MAX_HEADER` of that script, whose
/// rules the core must agree with; `tests/process/test_conventions.py` keeps the two equal.
pub(crate) const HEADER_MAX_CHARS: usize = 72;

/// The type whose scope is optional; the branch of the Revert button of GitHub starts with it
/// too, `revert-<n>-`.
pub(crate) const REVERT: &str = "revert";

/// The start of the paragraph or of the footer that a breaking change needs in the body of its
/// commit.
pub(crate) const BREAKING_CHANGE: &str = "BREAKING CHANGE:";

/// The starts of the headers that git writes itself, exempt from the grammar (#267, decided by
/// the maintainer on 2026-11-25): a merge, a revert, and the commits that an autosquash rebase
/// folds into others. A deliberate copy of `GENERATED` of `scripts/conventions.py`;
/// `tests/process/test_conventions.py` keeps the two equal.
pub(crate) const GENERATED: [&str; 4] = ["Merge ", "Revert \"", "fixup! ", "squash! "];

/// The characters of white space of a blank line of a body and around the text of a
/// `BREAKING CHANGE:` line: a space and a tab.
const BLANK: [char; 2] = [' ', '\t'];

/// The types and the scopes a header is checked against.
#[derive(Debug, Clone)]
pub(crate) struct HeaderNames<'p> {
    pub(crate) types: Names<'p>,
    pub(crate) scopes: Names<'p>,
}

/// What [`check`] found in a header.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Checked {
    /// Every error of the header, in the order of [`Fault`].
    pub(crate) errors: Vec<Error>,
    /// Whether the header is in the form of the grammar and marks a breaking change with `!`.
    pub(crate) breaking: bool,
}

/// A fault of a header, in the order in which its errors come.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fault {
    /// Not `<type>(<scope>)[!]: <subject>`.
    Form,
    /// The type is not a type of the policy.
    Type,
    /// No scope, and the type is not `revert`.
    NoScope,
    /// The scope is not a scope of the policy.
    Scope,
    /// The subject does not start as [`starts_well`] requires.
    Start,
    /// The subject, without its trailing white space, ends with a full stop.
    FullStop,
    /// The header is longer than [`HEADER_MAX_CHARS`] characters.
    Length,
}

/// A header in the form of the grammar, in its parts.
#[derive(Debug, Clone, Copy)]
struct Parsed<'h> {
    kind: &'h str,
    /// The scope; `None` when the header has none, or `()`.
    scope: Option<&'h str>,
    breaking: bool,
    /// The header up to its subject: `<type>(<scope>)[!]: `.
    lead: &'h str,
    /// The subject without its reference.
    subject: &'h str,
    /// The reference with the space before it, ` [SPEC-<seq>...]`; empty when there is none.
    reference: &'h str,
}

/// Every error of the header `header`, at `at`, against the types and scopes of `names`, and
/// whether it marks a breaking change. The errors of the subject carry in `fix` the header with
/// its subject corrected, when the correction can be derived and the corrected header has no
/// fault.
pub(crate) fn check(header: &str, names: &HeaderNames<'_>, at: &Pointer) -> Checked {
    let (parsed, found) = faults(header, names);
    let subject_faults = found
        .iter()
        .any(|fault| matches!(fault, Fault::Start | Fault::FullStop));
    let fix = parsed
        .filter(|_| subject_faults)
        .and_then(|p| corrected(&p, names));
    let errors = found
        .into_iter()
        .map(|fault| {
            let error = Error::new(
                codes::COMMIT_HEADER,
                message(fault),
                at.clone(),
                reason(fault),
            );
            match fault {
                Fault::Type => error.with_expected(names.types.expected()),
                Fault::NoScope | Fault::Scope => error.with_expected(names.scopes.expected()),
                Fault::Start | Fault::FullStop => match &fix {
                    Some(fix) => error.with_fix(fix.as_str()),
                    None => error,
                },
                Fault::Form | Fault::Length => error,
            }
        })
        .collect();
    Checked {
        errors,
        breaking: parsed.is_some_and(|p| p.breaking),
    }
}

/// Whether `header` is written by git itself, so exempt from the grammar: it starts with one
/// of [`GENERATED`].
pub(crate) fn is_generated(header: &str) -> bool {
    GENERATED.iter().any(|start| header.starts_with(start))
}

/// Whether `body` states a breaking change: a line `BREAKING CHANGE: <text>`, with text after
/// the colon, that starts a paragraph (the first line of the body or a line after a blank one)
/// or is a footer, a line of the last paragraph, where the trailers are (#267, decided by the
/// maintainer on 2026-11-25). A blank line holds only spaces and tabs.
pub(crate) fn has_breaking_change(body: &str) -> bool {
    let mut starts_paragraph = true;
    // Whether a line of the latest paragraph, not its first, states the change.
    let mut footer = false;
    for line in body.lines() {
        if line.trim_matches(BLANK).is_empty() {
            starts_paragraph = true;
            continue;
        }
        let states = line
            .strip_prefix(BREAKING_CHANGE)
            .is_some_and(|text| !text.trim_matches(BLANK).is_empty());
        if starts_paragraph {
            if states {
                return true;
            }
            footer = false;
            starts_paragraph = false;
        } else {
            footer |= states;
        }
    }
    // A line of the last paragraph, where the trailers are, is a footer.
    footer
}

/// The error of a commit whose header, at `header`, marks a breaking change while its body, at
/// `body`, states none (see [`has_breaking_change`]).
pub(crate) fn requires(body: Pointer, header: Pointer) -> Error {
    let message = format!(
        "The header marks a breaking change with `!`, so the body needs a paragraph or a footer \
         `{BREAKING_CHANGE} <what breaks>`."
    );
    Error::new(codes::COMMIT_HEADER, message, body, Reason::Requires).with_pointers([header])
}

/// The header read in its parts, when it is in the form of the grammar, and its faults.
fn faults<'h>(header: &'h str, names: &HeaderNames<'_>) -> (Option<Parsed<'h>>, Vec<Fault>) {
    let parsed = parse(header);
    let mut found = Vec::new();
    match &parsed {
        None => found.push(Fault::Form),
        Some(p) => {
            if !names.types.contains(p.kind) {
                found.push(Fault::Type);
            }
            match p.scope {
                None if p.kind != REVERT => found.push(Fault::NoScope),
                Some(scope) if !names.scopes.contains(scope) => found.push(Fault::Scope),
                _ => {}
            }
            if !starts_well(p.subject) {
                found.push(Fault::Start);
            }
            if p.subject.trim_end().ends_with('.') {
                found.push(Fault::FullStop);
            }
        }
    }
    if header.chars().nth(HEADER_MAX_CHARS).is_some() {
        found.push(Fault::Length);
    }
    (parsed, found)
}

/// Whether `subject` starts as the grammar requires: with a lower-case ASCII letter, `a` to `z`,
/// or with any character outside ASCII. It never starts with an upper-case ASCII letter, `A` to
/// `Z`, nor with any other ASCII character: a digit, punctuation such as a backtick, or white
/// space. The rule reads no table of Unicode, so it gives one verdict under every version of
/// Unicode, in the core and in `scripts/conventions.py` (#267, decided by the maintainer on
/// 2026-11-25).
fn starts_well(subject: &str) -> bool {
    subject
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || !c.is_ascii())
}

/// `header` in its parts, or `None` when it is not `<type>(<scope>)[!]: <subject>` with a type
/// without white space and a subject of at least one character. The scope ends at the first
/// `)`.
fn parse(header: &str) -> Option<Parsed<'_>> {
    let end = header.find(['(', '!', ':'])?;
    let (kind, rest) = header.split_at(end);
    if kind.is_empty() || kind.contains(char::is_whitespace) {
        return None;
    }
    let (scope, rest) = match rest.strip_prefix('(') {
        Some(scoped) => {
            let (scope, rest) = scoped.split_once(')')?;
            (Some(scope).filter(|s| !s.is_empty()), rest)
        }
        None => (None, rest),
    };
    let (breaking, rest) = match rest.strip_prefix('!') {
        Some(rest) => (true, rest),
        None => (false, rest),
    };
    let subject = rest.strip_prefix(": ").filter(|s| !s.is_empty())?;
    let lead = header.get(..header.len() - subject.len())?;
    let (subject, reference) = split_reference(subject);
    Some(Parsed {
        kind,
        scope,
        breaking,
        lead,
        subject,
        reference,
    })
}

/// `subject` without its final reference ` [SPEC-<seq>[ R<k>[, R<k>]*]]`, and the reference
/// with the space before it; the whole subject and an empty reference when it ends with none.
fn split_reference(subject: &str) -> (&str, &str) {
    let reference = subject
        .strip_suffix(']')
        .and_then(|open| open.rfind(" [").map(|at| (at, open.get(at + 2..))))
        .and_then(|(at, inner)| inner.filter(|inner| is_reference(inner)).map(|_| at));
    match reference {
        Some(at) => subject.split_at(at),
        None => (subject, ""),
    }
}

/// Whether `inner` is `SPEC-<seq>[ R<k>[, R<k>]*]`, a reference without its brackets.
fn is_reference(inner: &str) -> bool {
    let (spec, requirements) = match inner.split_once(' ') {
        Some((spec, requirements)) => (spec, Some(requirements)),
        None => (inner, None),
    };
    accepts(Format::SpecId, spec)
        && requirements
            .is_none_or(|list| list.split(", ").all(|r| accepts(Format::RequirementId, r)))
}

/// The header of `parsed` with its subject corrected: its first letter in lower case when it is
/// an upper-case ASCII letter that starts a capitalised word, not an acronym (the next character
/// is not an upper-case ASCII letter), and without its final full stops and white space; `None`
/// when the correction cannot be derived or the corrected header still has a fault.
fn corrected(parsed: &Parsed<'_>, names: &HeaderNames<'_>) -> Option<String> {
    let subject = parsed
        .subject
        .trim_end_matches(|c: char| c == '.' || c.is_whitespace());
    let mut chars = subject.chars();
    let first = chars.next()?;
    let rest = chars.as_str();
    let start = if starts_well(subject) {
        first
    } else if first.is_ascii_uppercase() && !rest.starts_with(|c: char| c.is_ascii_uppercase()) {
        first.to_ascii_lowercase()
    } else {
        return None;
    };
    let header = format!("{}{start}{rest}{}", parsed.lead, parsed.reference);
    let (_, found) = faults(&header, names);
    found.is_empty().then_some(header)
}

fn reason(fault: Fault) -> Reason {
    match fault {
        Fault::Form | Fault::Start | Fault::FullStop => Reason::Syntax,
        Fault::Type | Fault::Scope => Reason::Enum,
        Fault::NoScope => Reason::Missing,
        Fault::Length => Reason::Length,
    }
}

fn message(fault: Fault) -> String {
    match fault {
        Fault::Form => "The header is not `<type>(<scope>)[!]: <subject>`.".to_owned(),
        Fault::Type => "The type of the header is not a type of the policy.".to_owned(),
        Fault::NoScope => format!(
            "The header has no scope; a scope of the policy is required, except for the type \
             `{REVERT}`."
        ),
        Fault::Scope => "The scope of the header is not a scope of the policy.".to_owned(),
        Fault::Start => "The subject starts with an upper-case ASCII letter or another ASCII \
                         character that is not a lower-case letter; it must start with a letter \
                         from `a` to `z` or a character outside ASCII."
            .to_owned(),
        Fault::FullStop => "The subject ends with a full stop.".to_owned(),
        Fault::Length => {
            format!("The header is longer than {HEADER_MAX_CHARS} characters, reference included.")
        }
    }
}

#[cfg(test)]
mod tests;
