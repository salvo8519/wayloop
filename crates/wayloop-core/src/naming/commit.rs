//! Commit headers and pull request titles (SPEC-0001 R11; Naming grammar, Commit header and
//! pull request title).
//!
//! [`check`] reads a header in the grammar
//! `<type>(<scope>)[!]: <subject>[ [SPEC-<seq>[ R<k>[, R<k>]*]]]`: `<type>` is a type of the
//! policy; `<scope>` is a scope of the policy, required except for `revert`; `<subject>` starts
//! with a lower-case letter and has no final full stop; the header is at most
//! [`HEADER_MAX_CHARS`] characters, reference included. Each fault is one error with the code
//! `commit_header` at the pointer of the header. [`has_breaking_paragraph`] and [`requires`]
//! give the rule of the breaking mark `!` of a commit, whose body then needs a paragraph
//! starting with `BREAKING CHANGE:`.
//!
//! Every rule reads the header once, or a bounded part of it, so the cost grows with its length
//! and never with its square.

use wayloop_schemas::Format;

use super::Names;
use crate::artifacts::codes;
use crate::schema::format::accepts;
use crate::{Error, Pointer, Reason};

/// The most characters of a header, reference included: Unicode scalar values, as git and
/// `scripts/conventions.py` count them. A deliberate copy of `MAX_HEADER` of that script, whose
/// rules the core must agree with; `tests/process/test_conventions.py` keeps the two equal.
pub(crate) const HEADER_MAX_CHARS: usize = 72;

/// The type whose scope is optional.
pub(crate) const REVERT: &str = "revert";

/// The start of the paragraph that a breaking change needs in the body of its commit.
pub(crate) const BREAKING_CHANGE: &str = "BREAKING CHANGE:";

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
    /// The subject does not start with a lower-case letter.
    Start,
    /// The subject ends with a full stop.
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

/// Whether `body` has a paragraph that starts with `BREAKING CHANGE:`: a line that starts the
/// body or follows a blank line.
pub(crate) fn has_breaking_paragraph(body: &str) -> bool {
    let mut starts_paragraph = true;
    for line in body.lines() {
        if starts_paragraph && line.starts_with(BREAKING_CHANGE) {
            return true;
        }
        starts_paragraph = line.trim().is_empty();
    }
    false
}

/// The error of a commit whose header, at `header`, marks a breaking change while its body, at
/// `body`, has no paragraph that starts with `BREAKING CHANGE:`.
pub(crate) fn requires(body: Pointer, header: Pointer) -> Error {
    let message = format!(
        "The header marks a breaking change with `!`, so the body needs a paragraph that starts \
         with `{BREAKING_CHANGE}`."
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
            if !p.subject.chars().next().is_some_and(char::is_lowercase) {
                found.push(Fault::Start);
            }
            if p.subject.ends_with('.') {
                found.push(Fault::FullStop);
            }
        }
    }
    if header.chars().nth(HEADER_MAX_CHARS).is_some() {
        found.push(Fault::Length);
    }
    (parsed, found)
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

/// The header of `parsed` with its subject corrected: its first letter in lower case when it
/// starts a capitalised word, and without its final full stops; `None` when the correction
/// cannot be derived or the corrected header still has a fault.
fn corrected(parsed: &Parsed<'_>, names: &HeaderNames<'_>) -> Option<String> {
    let subject = parsed.subject.trim_end_matches('.').trim_end();
    let mut chars = subject.chars();
    let first = chars.next()?;
    let rest = chars.as_str();
    let start = if first.is_lowercase() {
        first.to_string()
    } else if first.is_uppercase() && !rest.chars().next().is_some_and(char::is_uppercase) {
        first.to_lowercase().collect()
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
        Fault::Start => "The subject does not start with a lower-case letter.".to_owned(),
        Fault::FullStop => "The subject ends with a full stop.".to_owned(),
        Fault::Length => {
            format!("The header is longer than {HEADER_MAX_CHARS} characters, reference included.")
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};
    use wayloop_schemas::value::PolicyName;

    use super::{HEADER_MAX_CHARS, HeaderNames, check, has_breaking_paragraph, requires};
    use crate::artifacts::codes;
    use crate::naming::Names;
    use crate::policy::{SCOPES, TYPES};
    use crate::{Pointer, Reason};

    fn names(list: &[&str]) -> Vec<PolicyName> {
        list.iter().map(|n| PolicyName::new(*n)).collect()
    }

    /// The reasons and fixes of the errors of `header` against `types` and `scopes`; each is a
    /// `commit_header` at `/header`.
    fn faults_with(header: &str, types: &[&str], scopes: &[&str]) -> Vec<(Reason, Option<String>)> {
        let (types, scopes) = (names(types), names(scopes));
        let names = HeaderNames {
            types: Names::new(&types),
            scopes: Names::new(&scopes),
        };
        let at = Pointer::root().join("header");
        check(header, &names, &at)
            .errors
            .iter()
            .map(|e| {
                assert_eq!(e.code(), &codes::COMMIT_HEADER, "{header}");
                assert_eq!(e.pointer(), &at, "{header}");
                let fix = e.fix().and_then(Value::as_str).map(str::to_owned);
                (e.reason(), fix)
            })
            .collect()
    }

    /// [`faults_with`] the default policy.
    fn faults(header: &str) -> Vec<(Reason, Option<String>)> {
        faults_with(header, &TYPES, &SCOPES)
    }

    fn breaking(header: &str) -> bool {
        let (types, scopes) = (names(&TYPES), names(&SCOPES));
        let names = HeaderNames {
            types: Names::new(&types),
            scopes: Names::new(&scopes),
        };
        check(header, &names, &Pointer::root()).breaking
    }

    const NONE: [(Reason, Option<String>); 0] = [];

    /// The scopes of `examples/policy.yaml` of SPEC-0001.
    const EXAMPLE_SCOPES: [&str; 5] = ["core", "cli", "docs", "specs", "ci"];

    #[test]
    fn valid_headers_pass() {
        for header in [
            "feat(state): add the transition table",
            "fix(ledger): reject a broken hash chain [SPEC-0003 R4, R5]",
            "feat(cli)!: rename the next command",
            "revert: feat(state): add the transition table",
            "revert(core): drop the hash chain",
            "feat(tracker): read sub-issues into the cache [SPEC-0005 R1]",
            "feat(core): store \u{e8} as a letter [SPEC-10000]",
            "docs(docs): explain version 1.2 of the envelope",
        ] {
            assert_eq!(faults(header), NONE, "{header}");
        }
    }

    #[test]
    fn ac18_a_breaking_header_needs_its_paragraph() {
        assert!(breaking("feat(core)!: drop the v1 envelope"));
        assert!(!breaking("feat(core): drop the v1 envelope"));
        // A header out of the grammar marks nothing.
        assert!(!breaking("feat(core)!:drop the v1 envelope"));
        assert!(has_breaking_paragraph(
            "BREAKING CHANGE: the v1 envelope is gone."
        ));
        assert!(has_breaking_paragraph(
            "Drops the v1 envelope.\n\nBREAKING CHANGE: clients read v2.\n\nRefs: #12\n"
        ));
        assert!(has_breaking_paragraph(
            "Drops it.\r\n  \r\nBREAKING CHANGE: clients read v2.\r\n"
        ));
        for body in [
            "Drops the v1 envelope.\n\nRefs: #12\n",
            "Drops the v1 envelope.\nBREAKING CHANGE: inside a paragraph.\n",
            "Drops it.\n\n BREAKING CHANGE: indented.\n",
            "Drops it.\n\nBREAKING-CHANGE: another spelling.\n",
            "",
        ] {
            assert!(!has_breaking_paragraph(body), "{body}");
        }
        let header = Pointer::root().join("header");
        let error = requires(Pointer::root().join("body"), header.clone());
        assert_eq!(error.code(), &codes::COMMIT_HEADER);
        assert_eq!(error.reason(), Reason::Requires);
        assert_eq!(error.pointer().as_str(), "/body");
        assert_eq!(error.pointers(), Some(&[header][..]));
    }

    #[test]
    fn ac19_revert_needs_no_scope_and_another_type_does() {
        assert_eq!(faults("revert: drop the hash chain"), NONE);
        assert_eq!(
            faults("feat: chain events by hash"),
            [(Reason::Missing, None)]
        );
        assert_eq!(
            faults("feat(): chain events by hash"),
            [(Reason::Missing, None)]
        );
    }

    #[test]
    fn ac19_a_missing_scope_lists_the_scopes_of_the_policy() {
        let scopes = names(&EXAMPLE_SCOPES);
        let types = names(&TYPES);
        let header_names = HeaderNames {
            types: Names::new(&types),
            scopes: Names::new(&scopes),
        };
        let found = check(
            "feat: chain events by hash",
            &header_names,
            &Pointer::root(),
        );
        let [error] = &found.errors[..] else {
            panic!("{found:?}");
        };
        assert_eq!(
            error.expected(),
            Some(&json!(EXAMPLE_SCOPES).as_array().unwrap()[..])
        );
    }

    #[test]
    fn ac20_a_header_of_73_characters_is_too_long_and_one_of_72_passes() {
        let at_most = format!("feat(core): {}", "x".repeat(HEADER_MAX_CHARS - 12));
        assert_eq!(at_most.chars().count(), 72);
        assert_eq!(faults(&at_most), NONE);
        let over = format!("{at_most}y");
        assert_eq!(over.chars().count(), 73);
        assert_eq!(faults(&over), [(Reason::Length, None)]);
        // Characters, not bytes: 72 letters with accents pass.
        let accented = format!("feat(core): {}", "\u{e8}".repeat(HEADER_MAX_CHARS - 12));
        assert_eq!(faults(&accented), NONE);
        // The reference counts.
        let referenced = format!("feat(core): {} [SPEC-0001 R1]", "x".repeat(50));
        assert_eq!(faults(&referenced), [(Reason::Length, None)]);
    }

    #[test]
    fn ac21_a_type_or_a_scope_outside_the_policy_lists_the_policy() {
        let types = names(&TYPES);
        let scopes = names(&EXAMPLE_SCOPES);
        let header_names = HeaderNames {
            types: Names::new(&types),
            scopes: Names::new(&scopes),
        };
        let at = Pointer::root().join("header");
        for (header, expected) in [
            ("feature(core): add x", json!(TYPES)),
            ("feat(kernel): add x", json!(EXAMPLE_SCOPES)),
        ] {
            let found = check(header, &header_names, &at);
            let [error] = &found.errors[..] else {
                panic!("{header}: {found:?}");
            };
            assert_eq!(error.reason(), Reason::Enum, "{header}");
            assert_eq!(error.expected(), Some(&expected.as_array().unwrap()[..]));
            assert_eq!(error.fix(), None);
        }
        // The default policy has the scope `ledger`, the example policy does not.
        assert_eq!(faults("feat(ledger): add x"), NONE);
        assert_eq!(
            faults_with("feat(ledger): add x", &TYPES, &EXAMPLE_SCOPES),
            [(Reason::Enum, None)]
        );
    }

    #[test]
    fn ac22_a_capital_or_a_full_stop_in_the_subject_is_a_syntax_error_with_a_fix() {
        let fixed = Some("feat(core): add the ledger".to_owned());
        assert_eq!(
            faults("feat(core): Add the ledger"),
            [(Reason::Syntax, fixed.clone())]
        );
        assert_eq!(
            faults("feat(core): add the ledger."),
            [(Reason::Syntax, fixed.clone())]
        );
        assert_eq!(
            faults("feat(core): Add the ledger..."),
            [(Reason::Syntax, fixed.clone()), (Reason::Syntax, fixed)]
        );
        assert_eq!(
            faults("feat(ledger): chain events by hash [SPEC-0003 R2]"),
            NONE
        );
        // The full stop before the reference counts; the fix keeps the reference.
        assert_eq!(
            faults("feat(ledger): chain events by hash. [SPEC-0003 R2]"),
            [(
                Reason::Syntax,
                Some("feat(ledger): chain events by hash [SPEC-0003 R2]".to_owned())
            )]
        );
        // An acronym, a digit or a mark has no derived fix.
        for header in [
            "feat(core): API of the ledger",
            "feat(core): 2 ledgers",
            "feat(core): `wayloop init` creates a project",
        ] {
            assert_eq!(faults(header), [(Reason::Syntax, None)], "{header}");
        }
        // No fix when another fault remains in the corrected header.
        assert_eq!(
            faults("feature(core): Add x"),
            [(Reason::Enum, None), (Reason::Syntax, None)]
        );
    }

    #[test]
    fn a_header_out_of_the_grammar_is_a_syntax_error() {
        for header in [
            "Added guards",
            "feat(core):add x",
            "feat(core) : add x",
            "feat(core):",
            "feat(core): ",
            "feat(core: add x",
            ": add x",
            "my feat(core): add x",
        ] {
            assert_eq!(faults(header), [(Reason::Syntax, None)], "{header}");
        }
        // A bracket group that is no reference is part of the subject.
        assert_eq!(faults("feat(core): add x [wip]"), NONE);
        assert_eq!(faults("feat(core): add x. [SPEC-1 R1]"), NONE);
    }

    #[test]
    fn a_long_header_is_read_once() {
        // Linear in the length: a header of a million characters is checked at once.
        let header = format!("feat(core): {}", "a ".repeat(500_000));
        assert_eq!(faults(&header), [(Reason::Length, None)]);
        let header = format!("feat(core): add{}", "(".repeat(500_000));
        assert_eq!(faults(&header), [(Reason::Length, None)]);
    }
}
