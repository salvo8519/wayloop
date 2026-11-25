//! Branch names (SPEC-0001 R11; Naming grammar, Branches).
//!
//! [`errors`] checks a branch name against the grammar: `<type>/<issue>-<slug>`, with a type of
//! the policy, or `sprint/S<NN>/<issue>-<slug>`; `<issue>` matches `[1-9][0-9]*` within the range
//! of an issue number, `<slug>` is one to five words of `[a-z0-9]+` joined by single hyphens, and
//! the name has at most [`BRANCH_MAX_CHARS`] characters. `dependabot/<anything>`,
//! `release/v<anything>` and `revert-<n>-<anything>` are exempt. Each fault is one error with
//! the code `branch_name` at the pointer of the branch.
//!
//! [`branch_name`] produces the name of a branch from a type, an issue and a title. Its slug
//! is the title in the form of [`super::titles::key`], decomposed, without combining marks,
//! with every run of characters other than ASCII letters and digits turned into one hyphen,
//! cut after five words and to the words that fit in [`BRANCH_MAX_CHARS`] characters; `work`
//! when nothing is left.
//!
//! Every rule reads the name once, or a bounded part of it, so the cost grows with its length
//! and never with its square.

use unicode_normalization::UnicodeNormalization as _;
use unicode_normalization::char::is_combining_mark;
use wayloop_schemas::number::is_safe;
use wayloop_schemas::value::{IssueNumber, PolicyName};

use super::Names;
use super::commit::REVERT;
use super::titles::key;
use crate::artifacts::codes;
use crate::schema::format::{all_digits, hyphenated_words_fix, is_counter, is_hyphenated_words};
use crate::{Error, Pointer, Reason};

/// The most words of the slug of a branch.
pub(crate) const SLUG_MAX_WORDS: usize = 5;

/// The most characters of a branch name, Unicode scalar values (#267, decided by the maintainer
/// on 2026-11-25). A deliberate copy of `BRANCH_MAX_CHARS` of `scripts/conventions.py`, whose
/// rules the core must agree with; `tests/process/test_conventions.py` keeps the two equal.
pub(crate) const BRANCH_MAX_CHARS: usize = 100;

/// The slug of a produced branch whose title leaves no word.
pub(crate) const EMPTY_SLUG: &str = "work";

/// The first segment of the branch of a sprint lane, `sprint/S<NN>/<issue>-<slug>`.
const SPRINT: &str = "sprint";

/// The fewest digits of the number of a sprint, `S<NN>`.
const SPRINT_MIN_DIGITS: usize = 2;

/// Prefixes of the branches exempt from the grammar, besides `revert-<n>-`: the branches of
/// Dependabot and of releases.
const EXEMPT: [&str; 2] = ["dependabot/", "release/v"];

/// The name of the branch of `issue`, of the type `kind`, with the slug of `title`
/// (SPEC-0001 R11, AC12). `kind` is a type of the policy; the name then passes the grammar of
/// branches, whatever the title. `None` when `issue` is beyond the range of an issue number,
/// or when `kind` and `issue` are so long that no slug fits in [`BRANCH_MAX_CHARS`] characters.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the claim of SPEC-0002 (T6, Q16) derives the names of its branches with it"
    )
)]
pub(crate) fn branch_name(kind: &PolicyName, issue: IssueNumber, title: &str) -> Option<String> {
    if !is_safe(i128::from(issue.get().get())) {
        return None;
    }
    let slug = slug(title);
    fitted(
        &format!("{}/{}-", kind.as_str(), issue),
        slug.as_deref().unwrap_or(EMPTY_SLUG),
    )
}

/// The slug of `text`: at most [`SLUG_MAX_WORDS`] words of `[a-z0-9]+` joined by single
/// hyphens, or `None` when `text` leaves no word.
fn slug(text: &str) -> Option<String> {
    let ascii: String = key(text)
        .nfd()
        .filter(|c| !is_combining_mark(*c))
        .map(|c| if c.is_ascii() { c } else { ' ' })
        .collect();
    let words = hyphenated_words_fix(&ascii)?;
    let cut: Vec<&str> = words
        .split('-')
        .filter(|word| !word.is_empty())
        .take(SLUG_MAX_WORDS)
        .collect();
    (!cut.is_empty()).then(|| cut.join("-"))
}

/// `lead` followed by the ASCII `slug` cut to the room [`BRANCH_MAX_CHARS`] leaves: its whole
/// words that fit, or else its first word cut to the room; `None` when `lead` leaves no room.
fn fitted(lead: &str, slug: &str) -> Option<String> {
    let room = BRANCH_MAX_CHARS
        .checked_sub(lead.chars().count())
        .filter(|room| *room > 0)?;
    let cut = if slug.len() <= room {
        slug
    } else {
        // The last hyphen within the room ends the whole words that fit.
        match slug.get(..=room).and_then(|head| head.rfind('-')) {
            Some(end) => slug.get(..end)?,
            None => slug.get(..room)?,
        }
    };
    Some(format!("{lead}{cut}"))
}

/// A fault of a branch name, in the order in which its errors come.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fault {
    /// The type is not a type of the policy.
    Type,
    /// Not in the form of the grammar.
    Form,
    /// The slug has more than [`SLUG_MAX_WORDS`] words.
    Words,
    /// The name has more than [`BRANCH_MAX_CHARS`] characters.
    Length,
}

/// A branch name read for its faults.
struct Read<'b> {
    faults: Vec<Fault>,
    /// The name before its slug, `<type>/<issue>-` or `sprint/S<NN>/<issue>-`, and the slug, when
    /// the name has them and its type and issue are valid: what a fix keeps and rewrites.
    fixable: Option<(&'b str, &'b str)>,
}

/// Every error of the branch name `branch`, at `at`, whose types are those of `types`: none for
/// an exempt branch; else, in order, the type outside the policy (`enum`, with the types in
/// `expected`), the form (`syntax`) or the number of words of the slug (`length`), then the
/// number of characters (`length`). The errors of the slug and of the length carry in `fix` the
/// branch with the slug written by the rule of [`branch_name`], when its type and issue are
/// valid and a slug fits. Such a name has no fault by construction (D4 of #267: a fix breaks no
/// other rule): its type, sprint and issue are those that passed, and its slug is one to five
/// words cut to the room the limit leaves; a property test checks it.
pub(crate) fn errors(branch: &str, types: &Names<'_>, at: &Pointer) -> Vec<Error> {
    if is_exempt(branch) {
        return Vec::new();
    }
    let read = read(branch, types);
    let fix = read
        .fixable
        .filter(|_| {
            read.faults
                .iter()
                .any(|f| matches!(f, Fault::Form | Fault::Words | Fault::Length))
        })
        .and_then(|(lead, words)| fitted(lead, &slug(words)?));
    read.faults
        .into_iter()
        .map(|fault| {
            let error = Error::new(
                codes::BRANCH_NAME,
                message(fault),
                at.clone(),
                reason(fault),
            );
            match (fault, &fix) {
                (Fault::Type, _) => error.with_expected(types.expected()),
                (Fault::Form | Fault::Words | Fault::Length, Some(fix)) => {
                    error.with_fix(fix.as_str())
                }
                (Fault::Form | Fault::Words | Fault::Length, None) => error,
            }
        })
        .collect()
}

/// The faults of a branch that is not exempt, and what a fix keeps.
fn read<'b>(branch: &'b str, types: &Names<'_>) -> Read<'b> {
    let mut faults = Vec::new();
    let named = match branch.split_once('/') {
        // A sprint lane names its sprint where the type is: `sprint/S<NN>/<issue>-<slug>`.
        Some((SPRINT, rest)) => match rest.split_once('/') {
            Some((sprint, named)) if is_sprint(sprint) => Some(named),
            Some(_) => None,
            None => Some(typed(SPRINT, rest, types, &mut faults)),
        },
        Some(("", _)) | None => None,
        Some((head, rest)) => Some(typed(head, rest, types, &mut faults)),
    };
    let parts = named.and_then(|named| {
        named
            .split_once('-')
            .filter(|(issue, _)| is_issue(issue))
            .map(|(_, words)| words)
    });
    let fixable = match parts {
        None => {
            faults.push(Fault::Form);
            None
        }
        Some(words) => {
            if !is_hyphenated_words(words) {
                faults.push(Fault::Form);
            } else if words.split('-').nth(SLUG_MAX_WORDS).is_some() {
                faults.push(Fault::Words);
            }
            let lead = branch.get(..branch.len() - words.len());
            lead.filter(|_| !faults.contains(&Fault::Type))
                .map(|lead| (lead, words))
        }
    };
    if branch.chars().nth(BRANCH_MAX_CHARS).is_some() {
        faults.push(Fault::Length);
    }
    Read { faults, fixable }
}

/// `rest`, the branch after its type `head`, after noting in `faults` a type outside `types`.
fn typed<'b>(head: &str, rest: &'b str, types: &Names<'_>, faults: &mut Vec<Fault>) -> &'b str {
    if !types.contains(head) {
        faults.push(Fault::Type);
    }
    rest
}

/// Whether `issue` is the number of an issue: `[1-9][0-9]*`, within the range of an issue
/// number, the safe integers of the contracts.
fn is_issue(issue: &str) -> bool {
    is_counter(issue) && issue.parse::<i128>().is_ok_and(is_safe)
}

/// Whether `branch` is exempt from the grammar: `dependabot/<anything>`, `release/v<anything>`
/// or `revert-<n>-<anything>`.
fn is_exempt(branch: &str) -> bool {
    EXEMPT.iter().any(|prefix| branch.starts_with(prefix))
        || branch
            .strip_prefix(REVERT)
            .and_then(|rest| rest.strip_prefix('-'))
            .and_then(|rest| rest.split_once('-'))
            .is_some_and(|(n, _)| all_digits(n))
}

/// Whether `sprint` is `S<NN>`, two digits or more.
fn is_sprint(sprint: &str) -> bool {
    sprint
        .strip_prefix('S')
        .is_some_and(|n| n.len() >= SPRINT_MIN_DIGITS && all_digits(n))
}

fn reason(fault: Fault) -> Reason {
    match fault {
        Fault::Type => Reason::Enum,
        Fault::Form => Reason::Syntax,
        Fault::Words | Fault::Length => Reason::Length,
    }
}

fn message(fault: Fault) -> String {
    match fault {
        Fault::Type => "The type of the branch is not a type of the policy.".to_owned(),
        Fault::Form => "The branch is not `<type>/<issue>-<slug>` or \
                        `sprint/S<NN>/<issue>-<slug>`: the issue is a number from 1, and the slug \
                        is one to five words of lower-case ASCII letters and digits joined by \
                        single hyphens."
            .to_owned(),
        Fault::Words => format!("The slug of the branch has more than {SLUG_MAX_WORDS} words."),
        Fault::Length => format!("The branch has more than {BRANCH_MAX_CHARS} characters."),
    }
}

#[cfg(test)]
mod tests;
