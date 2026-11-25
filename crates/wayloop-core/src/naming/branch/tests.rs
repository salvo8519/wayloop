//! Tests of the grammar of branches and of the names it produces (SPEC-0001 R11; AC12, AC15 to
//! AC17).

use core::num::NonZeroU64;

use proptest::prelude::*;
use serde_json::{Value, json};
use wayloop_schemas::number::SAFE_INTEGER;
use wayloop_schemas::value::{IssueNumber, PolicyName};

use super::{BRANCH_MAX_CHARS, EMPTY_SLUG, SLUG_MAX_WORDS, branch_name, errors};
use crate::artifacts::codes;
use crate::naming::Names;
use crate::policy::TYPES;
use crate::{Pointer, Reason};

fn types() -> Vec<PolicyName> {
    TYPES.iter().map(|t| PolicyName::new(*t)).collect()
}

/// The reasons and fixes of the errors of `branch` with the types `types`; each is a
/// `branch_name` at `/branch`.
fn faults_with(branch: &str, types: &[PolicyName]) -> Vec<(Reason, Option<String>)> {
    let at = Pointer::root().join("branch");
    errors(branch, &Names::new(types), &at)
        .iter()
        .map(|e| {
            assert_eq!(e.code(), &codes::BRANCH_NAME, "{branch}");
            assert_eq!(e.pointer(), &at, "{branch}");
            let fix = e.fix().and_then(Value::as_str).map(str::to_owned);
            (e.reason(), fix)
        })
        .collect()
}

/// [`faults_with`] the types of the default policy.
fn faults(branch: &str) -> Vec<(Reason, Option<String>)> {
    faults_with(branch, &types())
}

fn issue(n: u64) -> IssueNumber {
    IssueNumber::new(NonZeroU64::new(n).unwrap())
}

/// The largest issue number, the largest safe integer of the contracts.
fn largest_issue() -> u64 {
    u64::try_from(SAFE_INTEGER).unwrap()
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
    let long = format!("dependabot/{}", "x".repeat(BRANCH_MAX_CHARS));
    for branch in [
        "dependabot/cargo/serde-1.0.200",
        "release/v0.2.0",
        "revert-57-feat/12-ledger-hash-chain",
        "revert-57-Anything At All",
        // Exempt from the length too.
        long.as_str(),
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
fn the_issue_of_a_branch_is_within_the_range_of_an_issue_number() {
    let largest = largest_issue();
    assert_eq!(faults(&format!("feat/{largest}-ledger")), NONE);
    for beyond in [
        format!("feat/{}-ledger", largest + 1),
        format!("feat/{}-ledger", u64::MAX),
        format!("feat/1{}-ledger", "0".repeat(40)),
    ] {
        assert_eq!(faults(&beyond), [(Reason::Syntax, None)], "{beyond}");
    }
}

#[test]
fn a_branch_of_more_than_100_characters_is_too_long_with_its_slug_cut() {
    // 100 characters pass, 101 do not.
    let lead = "feat/42-";
    let at_most = format!("{lead}{}", "a".repeat(BRANCH_MAX_CHARS - lead.len()));
    assert_eq!(at_most.chars().count(), 100);
    assert_eq!(faults(&at_most), NONE);
    // One word too long: the fix cuts it to the room left.
    let over = format!("{at_most}b");
    assert_eq!(faults(&over), [(Reason::Length, Some(at_most.clone()))]);
    // Whole words that fit are kept.
    let words = format!("{lead}{}-{}", "a".repeat(60), "b".repeat(40));
    assert_eq!(
        faults(&words),
        [(Reason::Length, Some(format!("{lead}{}", "a".repeat(60))))]
    );
    // Six long words: two faults, one fix that removes both.
    let six = format!("{lead}{}", ["abcdefghijklmnopq"; 6].join("-"));
    let fix = format!("{lead}{}", ["abcdefghijklmnopq"; 5].join("-"));
    assert_eq!(
        faults(&six),
        [
            (Reason::Length, Some(fix.clone())),
            (Reason::Length, Some(fix))
        ]
    );
    // Linear in the length: a name of a million characters is checked at once.
    let huge = format!("{lead}{}", "a-".repeat(500_000));
    assert_eq!(faults(&huge).len(), 2);
}

#[test]
fn a_fix_is_given_only_when_the_corrected_name_has_no_fault() {
    // A type of 97 characters leaves no room for a slug: no fix.
    let long_type = format!("f{}", "e".repeat(96));
    let types = [PolicyName::new(long_type.as_str())];
    let branch = format!("{long_type}/42-Ledger");
    assert_eq!(
        faults_with(&branch, &types),
        [(Reason::Syntax, None), (Reason::Length, None)]
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
    assert_eq!(faults_with("feature/42-ledger", &other), NONE);
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
        assert_eq!(
            branch_name(&feat, issue(42), title).as_deref(),
            Some(expected),
            "{title}"
        );
        assert_eq!(faults(expected), NONE, "{expected}");
    }
    assert_eq!(EMPTY_SLUG, "work");
    // Beyond the range of an issue number, or with no room for a slug: no name.
    assert_eq!(branch_name(&feat, issue(largest_issue() + 1), "x"), None);
    let long = PolicyName::new(format!("f{}", "e".repeat(99)));
    assert_eq!(branch_name(&long, issue(1), "x"), None);
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
        "[a-z]{20,60}",
    ];
    prop_oneof![
        proptest::collection::vec(word, 0..12).prop_map(|words| words.join(" ")),
        any::<String>(),
    ]
}

#[test]
fn each_fault_has_its_own_message() {
    let types = types();
    let at = Pointer::root().join("branch");
    let long = format!("feature/42-{}", ["a"; 6].join("-")) + &"b".repeat(BRANCH_MAX_CHARS);
    let errors = errors(&format!("{long}-Ledger"), &Names::new(&types), &at);
    let messages: Vec<&str> = errors.iter().map(wayloop_schemas::Error::message).collect();
    assert_eq!(messages.len(), 3, "{messages:?}");
    assert!(messages[0].contains("type"), "{messages:?}");
    assert!(
        messages[1].contains("`<type>/<issue>-<slug>`"),
        "{messages:?}"
    );
    assert!(
        messages[2].contains(&BRANCH_MAX_CHARS.to_string()),
        "{messages:?}"
    );
    let words = errors_of_words();
    assert!(words.contains(&SLUG_MAX_WORDS.to_string()), "{words}");
}

/// The message of a slug of too many words.
fn errors_of_words() -> String {
    let types = types();
    let at = Pointer::root().join("branch");
    let found = errors("feat/42-a-b-c-d-e-f", &Names::new(&types), &at);
    found.first().unwrap().message().to_owned()
}

proptest! {
    #[test]
    fn every_fix_of_a_branch_passes_validation(
        kind in prop_oneof![Just("feat".to_owned()), "[a-z][a-z0-9-]{0,90}"],
        sprint in proptest::option::of("S[0-9]{2,3}"),
        n in 1..=largest_issue(),
        title in titles(),
    ) {
        // D4 (3) of #267: a fix breaks no other rule. The branches are any title after a valid
        // lead, so that their slug, words or length are faulty in every way.
        let types = [PolicyName::new(kind.as_str())];
        let lead = match &sprint {
            Some(sprint) => format!("sprint/{sprint}/{n}-"),
            None => format!("{kind}/{n}-"),
        };
        for error in errors(&format!("{lead}{title}"), &Names::new(&types), &Pointer::root()) {
            if let Some(fix) = error.fix().and_then(Value::as_str) {
                prop_assert_eq!(faults_with(fix, &types), NONE);
                prop_assert!(fix.starts_with(&lead));
            }
        }
    }

    #[test]
    fn ac12_any_produced_name_passes_validation(
        n in 1..=largest_issue(),
        // Types of the allowed form of a policy name (D1 of #267), of any length that leaves
        // room for a slug, and longer.
        kind in "[a-z][a-z0-9-]{0,90}",
        title in titles(),
    ) {
        let types = [PolicyName::new(kind.as_str())];
        let lead = format!("{kind}/{n}-");
        match branch_name(&types[0], issue(n), &title) {
            None => prop_assert!(lead.len() >= BRANCH_MAX_CHARS),
            Some(name) => {
                let rest = name.strip_prefix(&lead).unwrap();
                prop_assert!(rest.is_ascii());
                prop_assert!(rest.split('-').count() <= SLUG_MAX_WORDS);
                let lower_words = rest.split('-').all(|w| {
                    !w.is_empty() && w.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
                });
                prop_assert!(lower_words);
                prop_assert!(name.chars().count() <= BRANCH_MAX_CHARS);
                prop_assert_eq!(faults_with(&name, &types), NONE);
            }
        }
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
        prop_assert_eq!(name, Some(format!("fix/7-{expected}")));
    }
}
