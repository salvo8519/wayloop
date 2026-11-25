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
