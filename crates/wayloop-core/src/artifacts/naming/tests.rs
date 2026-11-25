//! Tests of the naming stage across the set (SPEC-0001 T17: R11; AC1 for `commit.invalid.yaml`,
//! AC18, AC30). The rules themselves are tested in `crate::naming::branch` and
//! `crate::naming::commit`; the contract cases are in `tests/contract/validate/v1/`.

// The helpers take JSON values by value so that a test reads as the document it builds.
#![allow(clippy::needless_pass_by_value)]

use serde_json::{Value, json};
use wayloop_schemas::Source;
use wayloop_schemas::examples::{commit, pull_request};
use wayloop_schemas::validate::InputKind;
use wayloop_schemas::value::IssueNumber;

use crate::artifacts::tests::{
    found, input_entry, passed_with, read, read_input, the_only_error, validate, with,
};
use crate::artifacts::{Validation, codes};
use crate::{Code, Pointer, Reason};

/// The subset of the policy of `examples/policy.yaml`.
fn example_policy() -> Value {
    json!({
        "types": crate::policy::TYPES,
        "scopes": ["core", "cli", "docs", "specs", "ci"],
        "areas": ["core", "cli", "docs"]
    })
}

fn commit_entry(document: Value) -> Value {
    input_entry(document, InputKind::Commit)
}

fn pull_request_entry(document: Value) -> Value {
    input_entry(document, InputKind::PullRequest)
}

/// Validates the inputs `documents` with `policy`.
fn with_policy(documents: Vec<Value>, policy: Value) -> Validation {
    validate(&read_input(
        json!({"documents": documents, "policy": policy}),
    ))
}

fn coded(code: Code, pointer: &str, reason: Reason) -> (Code, String, Reason) {
    (code, pointer.to_owned(), reason)
}

#[test]
fn ac1_the_invalid_commit_is_refused_for_its_scope_with_the_scopes_of_the_policy() {
    // `commit.invalid.yaml`.
    let invalid = json!({
        "schema_version": 1,
        "header": "feat(kernel): store durations in seconds",
        "body": "Signed-off-by: Example Owner <owner@example.org>\n"
    });
    let validation = with_policy(vec![commit_entry(invalid)], example_policy());
    let error = the_only_error(&validation);
    assert_eq!(
        found(&validation.errors),
        [coded(codes::COMMIT_HEADER, "/header", Reason::Enum)]
    );
    assert_eq!(error.document(), Some(0));
    assert_eq!(
        error.expected(),
        Some(
            &json!(["core", "cli", "docs", "specs", "ci"])
                .as_array()
                .unwrap()[..]
        )
    );
    assert_eq!(error.fix(), None);
    // The valid commit and pull request of the examples pass against the same policy.
    let validation = with_policy(
        vec![commit_entry(commit()), pull_request_entry(pull_request())],
        example_policy(),
    );
    passed_with(&validation, &["branch_name", "commit_header"]);
    // A policy is given: no default is reported.
    assert_eq!(validation.defaults_applied, []);
}

#[test]
fn ac18_a_breaking_commit_without_its_paragraph_is_refused_at_its_body() {
    let header = "feat(core)!: drop the v1 envelope";
    let breaking = with(commit(), "header", json!(header));
    passed_with(
        &validate(&read(vec![commit_entry(breaking.clone())])),
        &["commit_header"],
    );
    let without_paragraph = with(
        breaking.clone(),
        "body",
        json!("Drops the v1 envelope.\n\nRefs: #12\n"),
    );
    let mut one_line = breaking.clone();
    one_line.as_object_mut().unwrap().remove("body");
    for document in [without_paragraph, one_line] {
        let validation = validate(&read(vec![commit_entry(document.clone())]));
        let error = the_only_error(&validation);
        assert_eq!(
            found(&validation.errors),
            [coded(codes::COMMIT_HEADER, "/body", Reason::Requires)],
            "{document}"
        );
        assert_eq!(
            error.pointers(),
            Some(&[Pointer::root().join("header")][..])
        );
    }
    // A body refused by its schema is not read again for the paragraph.
    let validation = validate(&read(vec![commit_entry(with(breaking, "body", json!("")))]));
    assert_eq!(
        found(&validation.errors),
        [coded(codes::ARTIFACT_SCHEMA, "/body", Reason::Length)]
    );
}

#[test]
fn ac30_without_a_policy_the_scope_ledger_passes_and_the_default_is_listed() {
    let document = with(
        commit(),
        "header",
        json!("fix(ledger): chain events by hash"),
    );
    let validation = validate(&read(vec![commit_entry(document)]));
    passed_with(&validation, &["commit_header"]);
    let [default] = &validation.defaults_applied[..] else {
        panic!("{:?}", validation.defaults_applied);
    };
    assert_eq!(default.pointer().as_str(), "/policy");
    assert_eq!(default.reason().id(), "default_policy");
    assert_eq!(default.document(), None);
    // A pull request reads the policy too.
    let validation = validate(&read(vec![pull_request_entry(pull_request())]));
    assert_eq!(validation.defaults_applied.len(), 1);
}

#[test]
fn a_pull_request_is_checked_at_its_branch_and_its_title_with_its_source() {
    let document = with(
        with(pull_request(), "branch", json!("feat/12-Record-entry")),
        "title",
        json!("feat(core)!: Append one entry"),
    );
    let entry = json!({
        "format": "document", "input_kind": "pull_request", "document": document,
        "origin": {"issue": 40}
    });
    let validation = validate(&read(vec![entry]));
    assert_eq!(validation.failures, []);
    assert_eq!(
        found(&validation.errors),
        [
            coded(codes::BRANCH_NAME, "/branch", Reason::Syntax),
            coded(codes::COMMIT_HEADER, "/title", Reason::Syntax),
        ]
    );
    let fixes: Vec<Option<&Value>> = validation.errors.iter().map(|e| e.fix()).collect();
    assert_eq!(
        fixes,
        [
            Some(&json!("feat/12-record-entry")),
            Some(&json!("feat(core)!: append one entry"))
        ]
    );
    let issue = IssueNumber::new(core::num::NonZeroU64::new(40).unwrap());
    for error in &validation.errors {
        assert_eq!(error.document(), Some(0));
        assert_eq!(error.location(), Some(&Source::Issue { issue }));
    }
}

#[test]
fn a_breaking_pull_request_title_needs_no_paragraph_in_its_body() {
    // Its evidence is the label `breaking-change`, checked by SPEC-0021.
    let document = with(
        pull_request(),
        "title",
        json!("feat(core)!: append one entry per record command"),
    );
    passed_with(
        &validate(&read(vec![pull_request_entry(document)])),
        &["commit_header"],
    );
}

#[test]
fn a_member_refused_by_the_schema_is_not_checked_again_and_the_others_are() {
    // The title is not a string: only the schema refuses it; the branch is still checked.
    let document = with(
        with(pull_request(), "title", json!(7)),
        "branch",
        json!("feature/12-record"),
    );
    let validation = validate(&read(vec![pull_request_entry(document)]));
    // The errors of the schema come first, then those of the names (stages of the plan).
    assert_eq!(
        found(&validation.errors),
        [
            coded(codes::ARTIFACT_SCHEMA, "/title", Reason::Type),
            coded(codes::BRANCH_NAME, "/branch", Reason::Enum),
        ]
    );
}

#[test]
fn a_commit_whose_header_git_wrote_is_exempt_whatever_its_body() {
    // D3 of #267: by the form of the header, never by its author.
    for header in [
        "Merge branch 'main' into feat/12-x",
        "Revert \"feat(core)!: drop the v1 envelope\"",
        "fixup! feat(core)!: drop the v1 envelope",
        "squash! Anything at all.",
    ] {
        let document = with(commit(), "header", json!(header));
        let validation = validate(&read(vec![commit_entry(document.clone())]));
        passed_with(&validation, &["commit_header"]);
        let mut one_line = document;
        one_line.as_object_mut().unwrap().remove("body");
        passed_with(
            &validate(&read(vec![commit_entry(one_line)])),
            &["commit_header"],
        );
    }
    // A pull request title is not a message of git: it is checked.
    let document = with(pull_request(), "title", json!("Merge the branch"));
    let validation = validate(&read(vec![pull_request_entry(document)]));
    assert_eq!(
        found(&validation.errors),
        [coded(codes::COMMIT_HEADER, "/title", Reason::Syntax)]
    );
}

#[test]
fn ac18_a_breaking_change_may_be_stated_in_a_footer() {
    let breaking = with(
        commit(),
        "header",
        json!("feat(core)!: drop the v1 envelope"),
    );
    let footer = with(
        breaking.clone(),
        "body",
        json!(
            "Drops it.\n\nRefs: #12\nBREAKING CHANGE: clients read v2.\nSigned-off-by: A <a@b.c>\n"
        ),
    );
    passed_with(
        &validate(&read(vec![commit_entry(footer)])),
        &["commit_header"],
    );
    let empty = with(breaking, "body", json!("Drops it.\n\nBREAKING CHANGE:\n"));
    let validation = validate(&read(vec![commit_entry(empty)]));
    assert_eq!(
        found(&validation.errors),
        [coded(codes::COMMIT_HEADER, "/body", Reason::Requires)]
    );
}

#[test]
fn a_policy_over_its_bounds_or_out_of_the_form_of_names_is_refused_by_the_schema() {
    use wayloop_schemas::validate::Policy;

    use crate::{COMMAND_INPUT, Command, Context, run};
    // D1 of #267: the lists are bounded and each name is a lower-case word.
    // And each name has at most 32 characters (#267, decided by the maintainer on 2026-11-25).
    let names = |n: usize| -> Vec<String> { (0..n).map(|i| format!("t{i}")).collect() };
    let longest = "a".repeat(wayloop_schemas::POLICY_NAME_MAX_CHARS);
    let mut areas = names(Policy::AREAS_MAX - 1);
    areas.push(longest.clone());
    let at_most = json!({
        "types": names(Policy::TYPES_MAX),
        "scopes": names(Policy::SCOPES_MAX),
        "areas": areas
    });
    let header = |policy: &Value| {
        json!({
            "documents": [commit_entry(with(commit(), "header", json!("t0(t1): add x")))],
            "policy": policy
        })
    };
    let envelope = run(Command::Validate, &header(&at_most), &Context::default());
    assert_eq!(envelope.exit_code().code(), 0, "{envelope:?}");
    let over = json!({
        "types": names(Policy::TYPES_MAX + 1),
        "scopes": names(Policy::SCOPES_MAX + 1),
        "areas": ["core", "Core", "2fa", "area:core", "", format!("{longest}a")]
    });
    let envelope = run(Command::Validate, &header(&over), &Context::default());
    let Some(report) = envelope.outcome().report() else {
        panic!("refused expected: {envelope:?}");
    };
    assert_eq!(
        found(&report.errors().cloned().collect::<Vec<_>>()),
        [
            (COMMAND_INPUT, "/policy/areas/1".into(), Reason::Syntax),
            (COMMAND_INPUT, "/policy/areas/2".into(), Reason::Syntax),
            (COMMAND_INPUT, "/policy/areas/3".into(), Reason::Syntax),
            (COMMAND_INPUT, "/policy/areas/4".into(), Reason::Syntax),
            (COMMAND_INPUT, "/policy/areas/5".into(), Reason::Length),
            (COMMAND_INPUT, "/policy/scopes".into(), Reason::Length),
            (COMMAND_INPUT, "/policy/types".into(), Reason::Length),
        ]
    );
    // No check of the documents runs against a refused policy.
    assert_eq!(envelope.diagnostics().checks(), []);
}

#[test]
fn a_set_without_a_pull_request_or_a_commit_reads_no_policy() {
    // A hand-over input checks no name: the default policy is not reported (R21).
    let entry = input_entry(
        wayloop_schemas::examples::handover_input(),
        InputKind::HandoverInput,
    );
    let validation = validate(&read(vec![entry]));
    assert_eq!(validation.errors, []);
    assert!(
        validation
            .defaults_applied
            .iter()
            .all(|d| d.pointer().as_str() != "/policy"),
        "{:?}",
        validation.defaults_applied
    );
}
