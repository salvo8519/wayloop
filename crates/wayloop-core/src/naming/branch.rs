//! Branch names (SPEC-0001 R11; Naming grammar, Branches).
//!
//! [`errors`] checks a branch name against the grammar: `<type>/<issue>-<slug>`, with a type of
//! the policy, or `sprint/S<NN>/<issue>-<slug>`; `<issue>` matches `[1-9][0-9]*` and `<slug>` is
//! one to five words of `[a-z0-9]+` joined by single hyphens. `dependabot/<anything>`,
//! `release/v<anything>` and `revert-<n>-<anything>` are exempt. Each fault is one error with
//! the code `branch_name` at the pointer of the branch.
//!
//! [`branch_name`] produces the name of a branch from a type, an issue and a title. Its slug
//! is the title in the form of [`super::titles::key`], decomposed, without combining marks,
//! with every run of characters other than ASCII letters and digits turned into one hyphen,
//! cut after five words; `work` when nothing is left.
//!
//! Every rule reads the name once, or a bounded part of it, so the cost grows with its length
//! and never with its square.

use unicode_normalization::UnicodeNormalization as _;
use unicode_normalization::char::is_combining_mark;
use wayloop_schemas::value::{IssueNumber, PolicyName};

use super::Names;
use super::titles::key;
use crate::artifacts::codes;
use crate::schema::format::{all_digits, hyphenated_words_fix, is_counter, is_hyphenated_words};
use crate::{Error, Pointer, Reason};

/// The most words of the slug of a branch.
pub(crate) const SLUG_MAX_WORDS: usize = 5;

/// The slug of a produced branch whose title leaves no word.
pub(crate) const EMPTY_SLUG: &str = "work";

/// The first segment of the branch of a sprint lane, `sprint/S<NN>/<issue>-<slug>`.
const SPRINT: &str = "sprint";

/// The fewest digits of the number of a sprint, `S<NN>`.
const SPRINT_MIN_DIGITS: usize = 2;

/// Prefixes of the branches exempt from the grammar, besides `revert-<n>-`: the branches of
/// Dependabot and of releases.
const EXEMPT: [&str; 2] = ["dependabot/", "release/v"];

/// The prefix of the branch GitHub creates for its Revert button, `revert-<n>-<anything>`.
const REVERT: &str = "revert-";

/// The name of the branch of `issue`, of the type `kind`, with the slug of `title`
/// (SPEC-0001 R11). `kind` is a type of the policy; the name then passes the grammar of
/// branches, whatever the title.
#[must_use]
pub fn branch_name(kind: &PolicyName, issue: IssueNumber, title: &str) -> String {
    let slug = slug(title);
    let slug = slug.as_deref().unwrap_or(EMPTY_SLUG);
    format!("{}/{}-{slug}", kind.as_str(), issue)
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

/// Every error of the branch name `branch`, at `at`, whose types are those of `types`: none for
/// an exempt branch; else, in order, the type outside the policy (`enum`, with the types in
/// `expected`), then the form (`syntax`) or the number of words of the slug (`length`). The
/// errors of the slug carry in `fix` the branch with the slug written by the rule of
/// [`branch_name`], when its type and issue are valid and its slug leaves a word.
pub(crate) fn errors(branch: &str, types: &Names<'_>, at: &Pointer) -> Vec<Error> {
    if is_exempt(branch) {
        return Vec::new();
    }
    let Some((head, rest)) = branch.split_once('/') else {
        return vec![syntax(at)];
    };
    let mut errors = Vec::new();
    // A sprint lane names its sprint where the type is: `sprint/S<NN>/<issue>-<slug>`.
    let named = match (head, rest.split_once('/')) {
        (SPRINT, Some((sprint, named))) if is_sprint(sprint) => named,
        (SPRINT, Some(_)) | ("", _) => return vec![syntax(at)],
        _ => {
            if !types.contains(head) {
                let message = "The type of the branch is not a type of the policy.";
                errors.push(
                    Error::new(codes::BRANCH_NAME, message, at.clone(), Reason::Enum)
                        .with_expected(types.expected()),
                );
            }
            rest
        }
    };
    let Some((_, words)) = named.split_once('-').filter(|(issue, _)| is_counter(issue)) else {
        errors.push(syntax(at));
        return errors;
    };
    // The branch before its slug, which a fix keeps: a fix is derived only when it is valid.
    let lead = branch.get(..branch.len() - words.len()).unwrap_or_default();
    let fixable = errors.is_empty();
    let fix = || {
        fixable
            .then(|| slug(words))
            .flatten()
            .map(|slug| format!("{lead}{slug}"))
    };
    let error = if !is_hyphenated_words(words) {
        syntax(at)
    } else if words.split('-').nth(SLUG_MAX_WORDS).is_some() {
        let message = format!("The slug of the branch has more than {SLUG_MAX_WORDS} words.");
        Error::new(codes::BRANCH_NAME, message, at.clone(), Reason::Length)
    } else {
        return errors;
    };
    let error = match fix() {
        Some(fix) => error.with_fix(fix),
        None => error,
    };
    errors.push(error);
    errors
}

/// Whether `branch` is exempt from the grammar: `dependabot/<anything>`, `release/v<anything>`
/// or `revert-<n>-<anything>`.
fn is_exempt(branch: &str) -> bool {
    EXEMPT.iter().any(|prefix| branch.starts_with(prefix))
        || branch
            .strip_prefix(REVERT)
            .and_then(|rest| rest.split_once('-'))
            .is_some_and(|(n, _)| all_digits(n))
}

/// Whether `sprint` is `S<NN>`, two digits or more.
fn is_sprint(sprint: &str) -> bool {
    sprint
        .strip_prefix('S')
        .is_some_and(|n| n.len() >= SPRINT_MIN_DIGITS && all_digits(n))
}

/// The error of a branch that is not in the form of the grammar.
fn syntax(at: &Pointer) -> Error {
    let message = "The branch is not `<type>/<issue>-<slug>` or `sprint/S<NN>/<issue>-<slug>`: \
                   the issue is a number from 1, and the slug is one to five words of lower-case \
                   ASCII letters and digits joined by single hyphens.";
    Error::new(codes::BRANCH_NAME, message, at.clone(), Reason::Syntax)
}

#[cfg(test)]
mod tests {
    use core::num::NonZeroU64;

    use proptest::prelude::*;
    use serde_json::{Value, json};
    use wayloop_schemas::value::{IssueNumber, PolicyName};

    use super::{EMPTY_SLUG, SLUG_MAX_WORDS, branch_name, errors};
    use crate::artifacts::codes;
    use crate::naming::Names;
    use crate::policy::TYPES;
    use crate::{Pointer, Reason};

    fn types() -> Vec<PolicyName> {
        TYPES.iter().map(|t| PolicyName::new(*t)).collect()
    }

    /// The reasons and fixes of the errors of `branch` with the types of the default policy;
    /// each is a `branch_name` at `/branch`.
    fn faults(branch: &str) -> Vec<(Reason, Option<String>)> {
        let types = types();
        let at = Pointer::root().join("branch");
        errors(branch, &Names::new(&types), &at)
            .iter()
            .map(|e| {
                assert_eq!(e.code(), &codes::BRANCH_NAME, "{branch}");
                assert_eq!(e.pointer(), &at, "{branch}");
                let fix = e.fix().and_then(Value::as_str).map(str::to_owned);
                (e.reason(), fix)
            })
            .collect()
    }

    fn issue(n: u64) -> IssueNumber {
        IssueNumber::new(NonZeroU64::new(n).unwrap())
    }

    const NONE: [(Reason, Option<String>); 0] = [];

    #[test]
    fn ac15_a_sprint_branch_passes_and_a_sprint_without_its_number_is_refused() {
        assert_eq!(faults("sprint/S01/42-ledger-hash-chain"), NONE);
        assert_eq!(faults("sprint/S123/42-ledger-hash-chain"), NONE);
        for branch in [
            "sprint/1/42-ledger-hash-chain",
            "sprint/S1/42-ledger-hash-chain",
            "sprint/S/42-ledger-hash-chain",
            "sprint/s01/42-ledger-hash-chain",
            "sprint/S0a/42-ledger-hash-chain",
        ] {
            assert_eq!(faults(branch), [(Reason::Syntax, None)], "{branch}");
        }
    }

    #[test]
    fn ac16_the_exempt_branches_pass_without_further_checks() {
        for branch in [
            "dependabot/cargo/serde-1.0.200",
            "release/v0.2.0",
            "revert-57-feat/12-ledger-hash-chain",
            "revert-57-Anything At All",
        ] {
            assert_eq!(faults(branch), NONE, "{branch}");
        }
        // Only the exact prefixes are exempt: these are read as `<type>/<issue>-<slug>`.
        for branch in ["release/0.2.0", "revert-x-feat/12-a", "revert--feat/12-a"] {
            assert_ne!(faults(branch), NONE, "{branch}");
        }
    }

    #[test]
    fn ac17_a_slug_of_six_words_is_too_long_and_a_capital_is_a_syntax_error() {
        assert_eq!(
            faults("feat/42-one-two-three-four-five-six"),
            [(
                Reason::Length,
                Some("feat/42-one-two-three-four-five".to_owned())
            )]
        );
        assert_eq!(faults("feat/42-one-two-three-four-five"), NONE);
        assert_eq!(
            faults("feat/42-Ledger"),
            [(Reason::Syntax, Some("feat/42-ledger".to_owned()))]
        );
        assert_eq!(
            faults("sprint/S01/42-Ledger_hash"),
            [(Reason::Syntax, Some("sprint/S01/42-ledger-hash".to_owned()))]
        );
    }

    #[test]
    fn a_malformed_issue_or_slug_is_a_syntax_error() {
        for branch in [
            "feat",
            "feat/",
            "/42-ledger",
            "feat/0-ledger",
            "feat/042-ledger",
            "feat/x-ledger",
            "feat/42",
            "feat/42-",
            "feat/42-a/b",
            "feat/42-a--b",
            "feat/42--",
        ] {
            let found = faults(branch);
            assert!(
                found.iter().all(|(reason, _)| *reason == Reason::Syntax),
                "{branch}: {found:?}"
            );
            assert_eq!(found.len(), 1, "{branch}");
        }
        // A slug that leaves no word has no fix; one with words left gets them.
        assert_eq!(faults("feat/42-\u{65e5}\u{672c}"), [(Reason::Syntax, None)]);
        assert_eq!(
            faults("feat/42-citt\u{e0}"),
            [(Reason::Syntax, Some("feat/42-citta".to_owned()))]
        );
    }

    #[test]
    fn a_type_outside_the_policy_is_refused_with_the_types_expected() {
        let types = types();
        let at = Pointer::root().join("branch");
        let found = errors("feature/42-ledger", &Names::new(&types), &at);
        let [error] = &found[..] else {
            panic!("{found:?}");
        };
        assert_eq!(error.reason(), Reason::Enum);
        assert_eq!(
            error.expected(),
            Some(&json!(TYPES).as_array().unwrap()[..])
        );
        assert_eq!(error.fix(), None);
        // A bad type and a bad slug are two errors; the slug has no fix while the type is wrong.
        assert_eq!(
            faults("feature/42-Ledger"),
            [(Reason::Enum, None), (Reason::Syntax, None)]
        );
        // Another policy has other types.
        let other = [PolicyName::new("feature")];
        assert_eq!(errors("feature/42-ledger", &Names::new(&other), &at), []);
    }

    #[test]
    fn ac12_a_produced_name_has_the_form_and_slug_of_the_grammar() {
        let feat = PolicyName::new("feat");
        let cases = [
            ("Ledger hash chain", "feat/42-ledger-hash-chain"),
            ("Citt\u{e0} di Salerno", "feat/42-citta-di-salerno"),
            (
                "Cafe\u{301} cr\u{e8}me br\u{fb}l\u{e9}e",
                "feat/42-cafe-creme-brulee",
            ),
            ("Stra\u{df}e", "feat/42-strasse"),
            (
                "One two three four five six seven",
                "feat/42-one-two-three-four-five",
            ),
            ("  --Don't  stop--  ", "feat/42-don-t-stop"),
            ("\u{65e5}\u{672c}\u{8a9e}", "feat/42-work"),
            ("", "feat/42-work"),
            ("!!! ???", "feat/42-work"),
        ];
        for (title, expected) in cases {
            assert_eq!(branch_name(&feat, issue(42), title), expected, "{title}");
            assert_eq!(faults(expected), NONE, "{expected}");
        }
        assert_eq!(EMPTY_SLUG, "work");
    }

    /// Titles of words of letters with and without accents, of letters outside ASCII and of
    /// punctuation, joined by white space and punctuation.
    fn titles() -> impl Strategy<Value = String> {
        let word = prop_oneof![
            "[A-Za-z0-9]{1,8}",
            "[a-z\u{e0}-\u{ff}\u{100}-\u{17f}]{1,8}",
            "[\u{3b1}-\u{3c9}\u{4e00}-\u{4e20}\u{1f600}-\u{1f610}]{1,4}",
            "[a-z][\u{300}-\u{36f}][a-z]{0,4}",
            "[-_.,:;!?/()\\[\\]]{1,3}",
        ];
        prop_oneof![
            proptest::collection::vec(word, 0..12).prop_map(|words| words.join(" ")),
            any::<String>(),
        ]
    }

    proptest! {
        #[test]
        fn ac12_any_produced_name_passes_validation(
            n in 1_u64..=u64::MAX,
            kind in proptest::sample::select(&TYPES[..]),
            title in titles(),
        ) {
            let name = branch_name(&PolicyName::new(kind), issue(n), &title);
            let rest = name.strip_prefix(&format!("{kind}/{n}-")).unwrap();
            prop_assert!(rest.is_ascii());
            prop_assert!(rest.split('-').count() <= SLUG_MAX_WORDS);
            prop_assert!(rest
                .split('-')
                .all(|w| !w.is_empty() && w.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())));
            prop_assert_eq!(faults(&name), NONE);
        }

        #[test]
        fn ac12_the_slug_of_ascii_words_is_their_first_five_in_lower_case(
            words in proptest::collection::vec("[A-Za-z0-9]{1,8}", 0..9),
        ) {
            // An independent oracle: the words themselves, lower-cased and cut.
            let expected: Vec<String> = words
                .iter()
                .take(SLUG_MAX_WORDS)
                .map(|w| w.to_ascii_lowercase())
                .collect();
            let expected = if expected.is_empty() {
                EMPTY_SLUG.to_owned()
            } else {
                expected.join("-")
            };
            let name = branch_name(&PolicyName::new("fix"), issue(7), &words.join(" ; "));
            prop_assert_eq!(name, format!("fix/7-{expected}"));
        }
    }
}
