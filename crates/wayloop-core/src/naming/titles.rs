//! Titles of epics and tasks (SPEC-0001 R7, R8; Naming grammar, Titles).
//!
//! [`format`] refuses a title with a type prefix, a bracket tag at its start or at its end, or a
//! final full stop (R7): one error per rule broken, with the code `title_format` and the reason
//! `syntax` at `/title`, each with `fix` the title with every rule applied, when anything is
//! left of it. A title that starts with a command name, such as `wayloop init`, has no prefix.
//!
//! [`key`] is the form in which two titles are compared (R8): Unicode NFC, full case folding,
//! NFC again, then trimmed, with every run of white space collapsed into one space.
//!
//! The rules are applied until none is broken, so their cost grows with the square of the
//! length of the title; the callers pass titles of at most `TITLE_MAX_CHARS` characters, the
//! limit of the schema.

use caseless::default_case_fold_str;
use unicode_normalization::UnicodeNormalization as _;
use wayloop_schemas::contract::rules::{TITLE, TITLE_MAX_CHARS};
use wayloop_schemas::value::PolicyName;

use crate::artifacts::codes;
use crate::{Error, Pointer, Reason};

/// The pointer of the title of an artifact.
pub(crate) fn pointer() -> Pointer {
    Pointer::root().join(TITLE)
}

/// The types of the policy that a title may not start with, case-folded once: a prefix is found
/// without regard to case (R7), unlike the exact lookup of `super::Names` for branches and
/// headers (R11).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Prefixes {
    folded: Vec<String>,
    /// The length of the longest folded type, in characters: folding never shortens a word, so
    /// a prefix word is never longer.
    longest: usize,
}

impl Prefixes {
    /// The prefixes of the types `types` of the policy.
    pub(crate) fn new(types: &[PolicyName]) -> Self {
        let folded: Vec<String> = types
            .iter()
            .map(|t| default_case_fold_str(t.as_str()))
            .collect();
        let longest = folded
            .iter()
            .map(|t| t.chars().count())
            .max()
            .unwrap_or_default();
        Self { folded, longest }
    }

    /// Whether `word` is one of the types, without regard to case.
    fn contains(&self, word: &str) -> bool {
        let word = default_case_fold_str(word);
        self.folded.contains(&word)
    }
}

/// Every error of the form of `title`, whose type prefixes are `prefixes`, in the order of the
/// grammar: prefix, bracket tag, full stop. A rule broken once another is fixed counts too, for
/// example the prefix of `[EPIC] feat: Export`.
pub(crate) fn format(title: &str, prefixes: &Prefixes) -> Vec<Error> {
    debug_assert!(
        title.chars().nth(TITLE_MAX_CHARS).is_none(),
        "a title longer than the schema allows reached the rules"
    );
    let (fix, broken) = corrected(title, prefixes);
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

    /// `text` without what breaks the rule, or `None` when the rule holds. A run of final full
    /// stops is removed at once.
    fn without<'t>(self, text: &'t str, prefixes: &Prefixes) -> Option<&'t str> {
        match self {
            Rule::Prefix => without_prefix(text, prefixes),
            Rule::Tag => without_tag(text),
            Rule::FullStop => {
                let text = text.trim_end();
                text.ends_with('.')
                    .then(|| text.trim_end_matches(|c: char| c == '.' || c.is_whitespace()))
            }
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
    let recomposed: String = folded.nfc().collect();
    recomposed.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `title` without its type prefix `<word>[(<anything>)][!]:`, when the word is one of
/// `prefixes` without regard to case. The word is looked for only within the length of the
/// longest type.
fn without_prefix<'t>(title: &'t str, prefixes: &Prefixes) -> Option<&'t str> {
    let text = title.trim_start();
    let (end, _) = text
        .char_indices()
        .take(prefixes.longest.saturating_add(1))
        .find(|(_, c)| matches!(c, ':' | '(' | '!'))?;
    let (word, rest) = text.split_at_checked(end)?;
    // The scope ends at the first `):` or `)!:`, so it may hold `)` itself.
    let rest = match rest.strip_prefix('(') {
        Some(scoped) => scoped
            .match_indices(')')
            .find_map(|(at, _)| after_mark(scoped.get(at + 1..)?))?,
        None => after_mark(rest)?,
    };
    prefixes.contains(word).then_some(rest)
}

/// `text` after its leading `:` or `!:`.
fn after_mark(text: &str) -> Option<&str> {
    text.strip_prefix(':').or_else(|| text.strip_prefix("!:"))
}

/// `title` without a bracket tag `[...]` at its start, or else at its end. The tag ends at the
/// bracket that closes the first one, counting nested brackets; an unbalanced group is no tag.
fn without_tag(title: &str) -> Option<&str> {
    let text = title.trim();
    leading_tag_end(text)
        .and_then(|end| text.get(end..))
        .or_else(|| trailing_tag_start(text).and_then(|start| text.get(..start)))
}

/// The byte index after the bracket that closes the `[` at the start of `text`.
fn leading_tag_end(text: &str) -> Option<usize> {
    if !text.starts_with('[') {
        return None;
    }
    let mut depth = 0_usize;
    for (at, c) in text.char_indices() {
        match c {
            '[' => depth += 1,
            ']' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(at + c.len_utf8());
                }
            }
            _ => {}
        }
    }
    None
}

/// The byte index of the bracket that opens the `]` at the end of `text`.
fn trailing_tag_start(text: &str) -> Option<usize> {
    if !text.ends_with(']') {
        return None;
    }
    let mut depth = 0_usize;
    for (at, c) in text.char_indices().rev() {
        match c {
            ']' => depth += 1,
            '[' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(at);
                }
            }
            _ => {}
        }
    }
    None
}

/// `title` with every rule applied until none is broken, trimmed, empty when nothing is left,
/// and the rules it broke. Each step removes at least one character, so the loop ends.
fn corrected<'t>(title: &'t str, prefixes: &Prefixes) -> (&'t str, Vec<Rule>) {
    let mut text = title.trim();
    let mut broken = Vec::new();
    loop {
        let next = Rule::ALL
            .into_iter()
            .find_map(|rule| Some((rule, rule.without(text, prefixes)?)));
        let Some((rule, shorter)) = next else {
            return (text, broken);
        };
        if !broken.contains(&rule) {
            broken.push(rule);
        }
        text = shorter.trim();
    }
}

#[cfg(test)]
mod tests {
    use wayloop_schemas::value::PolicyName;

    use wayloop_schemas::contract::rules::TITLE_MAX_CHARS;

    use super::{Prefixes, format, key};
    use crate::Reason;
    use crate::artifacts::codes;

    /// The prefixes of the types of the default policy, which are those of
    /// `examples/policy.yaml` of SPEC-0001.
    fn types() -> Prefixes {
        prefixes(&crate::policy::TYPES)
    }

    fn prefixes(types: &[&str]) -> Prefixes {
        let names: Vec<PolicyName> = types.iter().map(|t| PolicyName::new(*t)).collect();
        Prefixes::new(&names)
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
            // The scope may hold `)`: it ends at the first `):` or `)!:`.
            "docs(a) b): Monthly export of time entries",
            "docs(a) b)!: Monthly export of time entries",
            // The breaking mark of a commit header (decided by the maintainer on 2026-11-25).
            "feat!: Monthly export of time entries",
            "feat(core)!: Monthly export of time entries",
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
            "feat(export)! monthly export",
            "feat!! monthly export",
        ] {
            assert_eq!(fixes(title), NONE, "{title}");
        }
        // With another policy, its types are the prefixes.
        // A type outside ASCII is matched after folding, whose form may be longer.
        assert_eq!(
            format("STRASSE: Export", &prefixes(&["Stra\u{df}e"])).len(),
            1
        );
        let export = prefixes(&["Export"]);
        assert_eq!(format("export: monthly totals", &export).len(), 1);
        assert_eq!(format("feat: monthly totals", &export), []);
        assert_eq!(format("feat: monthly totals", &prefixes(&[])), []);
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
        // A nested tag ends at the bracket that closes its first one.
        assert_eq!(fixes("[a [b]] Export"), [Some("Export".to_owned())]);
        assert_eq!(fixes("Export [a [b]]"), [Some("Export".to_owned())]);
        // A bracket inside the title, or an unbalanced group, is no tag.
        assert_eq!(fixes("Export the [month] of time entries"), NONE);
        assert_eq!(fixes("[Export the month of time entries"), NONE);
        assert_eq!(fixes("Export the month of time entries]"), NONE);
        assert_eq!(fixes("Export the [month] of entries]"), NONE);
        assert_eq!(fixes("[Export the [month] of entries"), NONE);
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
        // NFC again after folding: a folded letter composes with its combining mark.
        assert_eq!(
            key("Export \u{17f}\u{301} entries"),
            key("Export \u{15b} entries")
        );
        assert_eq!(key("Stra\u{df}\u{301}e"), key("STRAS\u{15a}E"));
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

    #[test]
    fn a_long_run_of_full_stops_is_removed_in_one_step() {
        let title = format!("Export{}", ".".repeat(TITLE_MAX_CHARS - 6));
        let errors = format(&title, &prefixes(&[]));
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].fix().and_then(|f| f.as_str()), Some("Export"));
    }
}
