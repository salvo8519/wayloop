//! Tests of the grammar of commit headers and pull request titles (SPEC-0001 R11; AC18 to
//! AC22).

use serde_json::{Value, json};
use wayloop_schemas::value::PolicyName;

use super::{HEADER_MAX_CHARS, HeaderNames, check, has_breaking_change, is_generated, requires};
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
    assert!(has_breaking_change(
        "BREAKING CHANGE: the v1 envelope is gone."
    ));
    assert!(has_breaking_change(
        "Drops the v1 envelope.\n\nBREAKING CHANGE: clients read v2.\n\nRefs: #12\n"
    ));
    assert!(has_breaking_change(
        "Drops it.\r\n \t\r\nBREAKING CHANGE: clients read v2.\r\n"
    ));
    // D4 (1) of #267: a footer, a line of the last paragraph, where the trailers are, even when
    // it is the only paragraph or blank lines follow it.
    for body in [
        "Drops the v1 envelope.\n\nRefs: #12\nBREAKING CHANGE: clients read v2.\n",
        "Drops it.\n\nRefs: #12\nBREAKING CHANGE: v2.\nSigned-off-by: A <a@b.c>\n\n \n",
        "Drops the v1 envelope.\nBREAKING CHANGE: in the only paragraph.\n",
        "BREAKING CHANGE:no space is needed\n",
    ] {
        assert!(has_breaking_change(body), "{body}");
    }
    for body in [
        "Drops the v1 envelope.\n\nRefs: #12\n",
        "Drops it.\nBREAKING CHANGE: inside a paragraph.\nMore.\n\nRefs: #12\n",
        "Drops it.\n\n BREAKING CHANGE: indented.\n",
        "Drops it.\n\nBREAKING-CHANGE: another spelling.\n",
        // Text is required after the colon, in a paragraph and in a footer.
        "Drops it.\n\nBREAKING CHANGE:\n",
        "Drops it.\n\nRefs: #12\nBREAKING CHANGE: \t \n",
        // White space other than a space or a tab does not make a line blank.
        "Drops it.\n\u{a0}\nBREAKING CHANGE: v2.\nMore.\n\nRefs: #12\n",
        "",
    ] {
        assert!(!has_breaking_change(body), "{body}");
    }
    let header = Pointer::root().join("header");
    let error = requires(Pointer::root().join("body"), header.clone());
    assert_eq!(error.code(), &codes::COMMIT_HEADER);
    assert_eq!(error.reason(), Reason::Requires);
    assert_eq!(error.pointer().as_str(), "/body");
    assert_eq!(error.pointers(), Some(&[header][..]));
}

#[test]
fn the_headers_that_git_writes_are_generated() {
    // D3 of #267: by their form, never by their author.
    for header in [
        "Merge branch 'main' into feat/12-x",
        "Merge pull request #12 from owner/feat/12-x",
        "Revert \"feat(core): add x\"",
        "fixup! feat(core): add x",
        "squash! feat(core): add x",
    ] {
        assert!(is_generated(header), "{header}");
    }
    for header in [
        "Merged the branch",
        "merge branch 'main'",
        "Revert feat(core): add x",
        "revert: feat(core): add x",
        "amend! feat(core): add x",
        "fixup!feat(core): add x",
        "build(deps): Bump serde from 1.0.1 to 1.0.2",
    ] {
        assert!(!is_generated(header), "{header}");
    }
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

#[test]
fn each_fault_has_its_own_message() {
    let (types, scopes) = (names(&TYPES), names(&SCOPES));
    let header_names = HeaderNames {
        types: Names::new(&types),
        scopes: Names::new(&scopes),
    };
    let messages = |header: &str| -> Vec<String> {
        check(header, &header_names, &Pointer::root())
            .errors
            .iter()
            .map(|e| e.message().to_owned())
            .collect()
    };
    let long = format!("feature(kernel): Add x{}.", "y".repeat(HEADER_MAX_CHARS));
    let mut all = messages(&long);
    all.extend(messages("feat: add x"));
    all.extend(messages("feat(core):add x"));
    assert_eq!(all.len(), 7, "{all:?}");
    let mut distinct = all.clone();
    distinct.sort();
    distinct.dedup();
    assert_eq!(distinct.len(), all.len(), "{all:?}");
    assert!(all[4].contains(&HEADER_MAX_CHARS.to_string()), "{all:?}");
    assert!(all[5].contains("revert"), "{all:?}");
}
