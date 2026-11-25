//! Titles of epics and tasks (SPEC-0001 R7, R8; Naming grammar, Titles).
//!
//! [`format`] refuses a title with a type prefix, a bracket tag at its start or at its end, or a
//! final full stop (R7): one error per rule broken, with the code `title_format` and the reason
//! `syntax` at `/title`, each with `fix` the title with every rule applied, when anything is
//! left of it. A title that starts with a command name, such as `wayloop init`, has no prefix.
//!
//! [`key`] is the form in which two titles are compared (R8): Unicode NFC, then case folding,
//! then trimmed, with every run of white space collapsed into one space.

use caseless::default_case_fold_str;
use unicode_normalization::UnicodeNormalization as _;
use wayloop_schemas::value::PolicyName;

use crate::artifacts::codes;
use crate::{Error, Pointer, Reason};

/// The member that holds the title of an artifact.
pub(crate) const TITLE: &str = "title";

/// The pointer of the title of an artifact.
pub(crate) fn pointer() -> Pointer {
    Pointer::root().join(TITLE)
}

/// Every error of the form of `title`, whose type prefixes are the types `types` of the policy,
/// in the order of the grammar: prefix, bracket tag, full stop. A rule broken once another is
/// fixed counts too, for example the prefix of `[EPIC] feat: Export`.
pub(crate) fn format(title: &str, types: &[PolicyName]) -> Vec<Error> {
    let (fix, broken) = corrected(title, types);
    Rule::ALL
        .into_iter()
        .filter(|rule| broken.contains(rule))
        .map(|rule| {
            let error = Error::new(
                codes::TITLE_FORMAT,
                rule.message(),
                pointer(),
                Reason::Syntax,
            );
            if fix.is_empty() {
                error
            } else {
                error.with_fix(fix)
            }
        })
        .collect()
}

/// A rule of the form of titles (R7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rule {
    Prefix,
    Tag,
    FullStop,
}

impl Rule {
    /// The rules in the order of the grammar.
    const ALL: [Rule; 3] = [Rule::Prefix, Rule::Tag, Rule::FullStop];

    /// `text` without what breaks the rule, or `None` when the rule holds.
    fn without<'t>(self, text: &'t str, types: &[PolicyName]) -> Option<&'t str> {
        match self {
            Rule::Prefix => without_prefix(text, types),
            Rule::Tag => without_tag(text),
            Rule::FullStop => text.trim_end().strip_suffix('.'),
        }
    }

    fn message(self) -> &'static str {
        match self {
            Rule::Prefix => {
                "The title starts with a type prefix of the policy; the type belongs to commits \
                 and pull requests, not to the title."
            }
            Rule::Tag => {
                "The title carries a bracket tag at its start or at its end; the type of the \
                 artifact is its `kind`."
            }
            Rule::FullStop => "The title ends with a full stop; a title is not a sentence.",
        }
    }
}

/// The form of `title` that two equal titles share (R8).
pub(crate) fn key(title: &str) -> String {
    let composed: String = title.nfc().collect();
    let folded = default_case_fold_str(&composed);
    folded.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `title` without its type prefix `<word>:` or `<word>(<anything>):`, when the word is one of
/// `types` without regard to case.
fn without_prefix<'t>(title: &'t str, types: &[PolicyName]) -> Option<&'t str> {
    let text = title.trim_start();
    let end = text.find([':', '('])?;
    let (word, rest) = text.split_at_checked(end)?;
    let rest = match rest.strip_prefix('(') {
        Some(scoped) => scoped.split_once("):")?.1,
        None => rest.strip_prefix(':')?,
    };
    let word = default_case_fold_str(word);
    types
        .iter()
        .any(|t| default_case_fold_str(t.as_str()) == word)
        .then_some(rest)
}

/// `title` without a bracket tag `[...]` at its start, or else at its end.
fn without_tag(title: &str) -> Option<&str> {
    let text = title.trim();
    let at_start = text
        .strip_prefix('[')
        .and_then(|inner| inner.split_once(']'))
        .map(|(_, rest)| rest);
    at_start.or_else(|| {
        text.strip_suffix(']')
            .and_then(|inner| inner.rsplit_once('['))
            .map(|(rest, _)| rest)
    })
}

/// `title` with every rule applied until none is broken, trimmed, empty when nothing is left,
/// and the rules it broke. Each step removes at least one character, so the loop ends.
fn corrected<'t>(title: &'t str, types: &[PolicyName]) -> (&'t str, Vec<Rule>) {
    let mut text = title.trim();
    let mut broken = Vec::new();
    loop {
        let next = Rule::ALL
            .into_iter()
            .find_map(|rule| Some((rule, rule.without(text, types)?)));
        let Some((rule, shorter)) = next else {
            return (text, broken);
        };
        broken.push(rule);
        text = shorter.trim();
    }
}

#[cfg(test)]
mod tests {
    use wayloop_schemas::value::PolicyName;

    use super::{format, key};
    use crate::Reason;
    use crate::artifacts::codes;

    /// The types of `examples/policy.yaml` of SPEC-0001.
    fn types() -> Vec<PolicyName> {
        [
            "feat", "fix", "perf", "refactor", "docs", "test", "build", "ci", "chore", "revert",
        ]
        .map(PolicyName::new)
        .to_vec()
    }

    /// The fixes of the errors of `title`, one per rule broken; each error is a `title_format`
    /// with the reason `syntax` at `/title`, with no `expected`.
    fn fixes(title: &str) -> Vec<Option<String>> {
        format(title, &types())
            .iter()
            .map(|e| {
                assert_eq!(
                    (e.code(), e.pointer().to_string(), e.reason()),
                    (&codes::TITLE_FORMAT, "/title".to_owned(), Reason::Syntax),
                    "{title}"
                );
                assert_eq!(e.expected(), None, "{title}");
                e.fix().and_then(|f| f.as_str()).map(str::to_owned)
            })
            .collect()
    }

    const NONE: [Option<String>; 0] = [];

    #[test]
    fn a_type_prefix_is_refused_without_regard_to_case_with_a_fix() {
        // AC4 (R7): `<word>:` and `<word>(<anything>):`, where the word is a type of the policy.
        for title in [
            "feat: Monthly export of time entries",
            "Feat:Monthly export of time entries",
            "FIX(export): Monthly export of time entries",
            "docs(a: b): Monthly export of time entries",
        ] {
            assert_eq!(
                fixes(title),
                [Some("Monthly export of time entries".to_owned())],
                "{title}"
            );
        }
    }

    #[test]
    fn a_word_that_is_no_type_of_the_policy_is_no_prefix() {
        // AC10 (R7): a title may start with a command name, or with another word and a colon.
        for title in [
            "wayloop init creates a project structure that validates",
            "Export: monthly totals of time entries",
            "feature: monthly export",
            "feat monthly export",
            "feat(export) monthly export",
        ] {
            assert_eq!(fixes(title), NONE, "{title}");
        }
        // With another policy, its types are the prefixes.
        let export = [PolicyName::new("Export")];
        assert_eq!(format("export: monthly totals", &export).len(), 1);
        assert_eq!(format("feat: monthly totals", &export), []);
        assert_eq!(format("feat: monthly totals", &[]), []);
    }

    #[test]
    fn a_bracket_tag_at_either_end_is_refused_with_a_fix() {
        // AC1 (`epic.invalid.yaml`) and AC4 (R7).
        for title in [
            "[EPIC] Monthly export of time entries",
            "[EPIC]Monthly export of time entries",
            "Monthly export of time entries [WIP]",
            "  [EPIC] Monthly export of time entries  ",
        ] {
            assert_eq!(
                fixes(title),
                [Some("Monthly export of time entries".to_owned())],
                "{title}"
            );
        }
        // A bracket inside the title, or one not closed, is no tag.
        assert_eq!(fixes("Export the [month] of time entries"), NONE);
        assert_eq!(fixes("[Export the month of time entries"), NONE);
        assert_eq!(fixes("Export the month of time entries]"), NONE);
    }

    #[test]
    fn a_final_full_stop_is_refused_with_a_fix() {
        // AC4 (R7).
        assert_eq!(
            fixes("Monthly export of time entries."),
            [Some("Monthly export of time entries".to_owned())]
        );
        assert_eq!(
            fixes("Monthly export of time entries... "),
            [Some("Monthly export of time entries".to_owned())]
        );
        assert_eq!(fixes("Export version 1.2 of the entries"), NONE);
    }

    #[test]
    fn each_rule_broken_is_an_error_whose_fix_applies_every_rule() {
        // One error per rule, in the order of the grammar, each once; each fix is a title that
        // passes. A rule broken once another is fixed counts too.
        let fix = Some("Monthly export".to_owned());
        for title in [
            "feat: Monthly export [WIP].",
            "[EPIC] feat: Monthly export.",
            "feat: [WIP] Monthly export. [x]",
        ] {
            assert_eq!(
                fixes(title),
                [fix.clone(), fix.clone(), fix.clone()],
                "{title}"
            );
        }
        let messages: Vec<String> = format("[EPIC] feat: Monthly export.", &types())
            .iter()
            .map(|e| e.message().to_owned())
            .collect();
        assert!(messages[0].contains("prefix"), "{messages:?}");
        assert!(messages[1].contains("bracket tag"), "{messages:?}");
        assert!(messages[2].contains("full stop"), "{messages:?}");
        assert_eq!(format("Monthly export", &types()), []);
    }

    #[test]
    fn a_fix_is_given_only_when_something_is_left() {
        assert_eq!(fixes("[EPIC]"), [None]);
        assert_eq!(fixes("feat: [WIP]."), [None, None, None]);
        assert_eq!(fixes("."), [None]);
    }

    #[test]
    fn titles_are_compared_after_nfc_case_folding_and_white_space() {
        // AC4 (R8): one key for titles equal after normalisation.
        let decomposed = "Export of the cafe\u{301} entries";
        let composed = "export of the  café   ENTRIES ";
        assert_eq!(key(decomposed), key(composed));
        assert_eq!(key(composed), "export of the café entries");
        // Case folding, not lower-casing: the sharp s folds to `ss`, the final sigma to sigma.
        assert_eq!(key("Stra\u{df}e"), key("STRASSE"));
        assert_eq!(
            key("\u{3a3}\u{3bf}\u{3c6}\u{3bf}\u{3c2}"),
            key("\u{3c3}\u{3bf}\u{3c6}\u{3bf}\u{3c3}")
        );
        // Every white space character counts, a tab and a no-break space included.
        assert_eq!(
            key("Monthly\texport\u{a0}of entries"),
            "monthly export of entries"
        );
        assert_ne!(key("Monthly export"), key("Monthly exports"));
    }
}
