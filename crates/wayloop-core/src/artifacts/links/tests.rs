//! Tests of the links between artifacts and of the hierarchy of plan items (SPEC-0001 T12: R3,
//! R4, R9, R15; AC2, AC5, AC6, AC7, AC14, AC24). The invalid examples of AC1 that break a link
//! are contract cases in `tests/contract/validate/v1/`.

use serde_json::{Value, json};
use wayloop_schemas::NotDoneReason;
use wayloop_schemas::examples::{
    change, council_report, epic, gate, handover, milestone, specification, story, task,
};

use crate::artifacts::tests::{
    at, entry, found, linked, project, read, read_input, stored, the_only_error, titled, validate,
    validated, with,
};
use crate::artifacts::{Validation, checks, codes};
use crate::{Code, Error, Reason};

/// The index of the first document after the project.
fn after() -> Option<u64> {
    u64::try_from(project().len()).ok()
}

/// The only warning of `validation`, which has no error.
fn only_warning(validation: &Validation) -> &Error {
    assert_eq!(validation.failures, []);
    assert_eq!(validation.errors, []);
    let [warning] = &validation.warnings[..] else {
        panic!("{:?}", validation.warnings);
    };
    warning
}

fn coded(code: Code, pointer: &str, reason: Reason) -> Vec<(Code, String, Reason)> {
    vec![(code, pointer.to_owned(), reason)]
}

/// `document`, an epic or a task of the examples, called `id`, with a title no other epic or
/// task of the project has (R8): its sequence, from 100 for an epic.
fn item(document: Value, id: &str) -> Value {
    let sequence: u32 = id.rsplit('-').next().unwrap().parse().unwrap();
    let offset = if id.starts_with("EPIC") { 100 } else { 0 };
    titled(with(document, "id", json!(id)), offset + sequence)
}

/// A task of the examples called `id` in `status`, with `member` set to `value`.
fn task_with(id: &str, status: &str, member: &str, value: Value) -> Value {
    let task = with(item(task(), id), "status", json!(status));
    with(task, member, value)
}

/// A story of the examples called `id`, approved, with `member` set to `value`.
fn story_with(id: &str, member: &str, value: Value) -> Value {
    let story = with(with(story(), "id", json!(id)), "status", json!("approved"));
    with(story, member, value)
}

#[test]
fn the_example_project_links_with_no_error_and_no_warning_and_the_check_runs() {
    let validation = validated(Vec::new());
    assert_eq!(validation.failures, []);
    assert_eq!(validation.errors, []);
    assert_eq!(validation.warnings, []);
    assert!(validation.checks.contains(&checks::LINKS));
    let not_done: Vec<&str> = validation.not_done.iter().map(|n| n.check().id()).collect();
    assert!(!not_done.contains(&checks::LINKS.id()), "{not_done:?}");
    // Without `adrs`, the citations of ADRs are not checked, and the result says why.
    let adr = validation
        .not_done
        .iter()
        .find(|n| n.check() == &checks::ADR_CITATIONS)
        .map(wayloop_schemas::NotDone::reason);
    assert_eq!(adr, Some(NotDoneReason::InputAbsent));
    assert!(!validation.checks.contains(&checks::ADR_CITATIONS));
}

#[test]
fn ac2_a_story_linked_to_a_missing_specification_names_the_story_the_field_and_the_target() {
    let story = story_with("STORY-0002", "specification", json!(["SPEC-0042"]));
    let validation = validated(vec![story]);
    let error = the_only_error(&validation);
    assert_eq!(
        found(&validation.errors),
        coded(codes::LINK_MISSING, "/specification/0", Reason::Missing)
    );
    assert_eq!(error.document(), after());
    assert!(error.message().contains("STORY-0002"), "{error:?}");
    assert!(error.message().contains("SPEC-0042"), "{error:?}");
    assert_eq!(error.fix(), None);
    // The specification does not say that a link error names another document.
    assert_eq!(error.related(), None);
}

#[test]
fn ac5_a_task_citing_a_missing_requirement_names_the_task_the_field_and_the_requirement() {
    let cited = json!([{"spec": "SPEC-0001", "requirements": ["R1", "R9"]}]);
    let validation = validated(vec![task_with(
        "TASK-0002",
        "approved",
        "specification",
        cited,
    )]);
    let error = the_only_error(&validation);
    assert_eq!(
        found(&validation.errors),
        coded(
            codes::LINK_MISSING,
            "/specification/0/requirements/1",
            Reason::Missing
        )
    );
    assert_eq!(error.document(), after());
    assert!(error.message().contains("TASK-0002"), "{error:?}");
    assert!(error.message().contains("SPEC-0001 R9"), "{error:?}");
}

#[test]
fn ac6_links_to_a_living_specification_moved_and_renamed_still_resolve() {
    // The specification is read from another folder, with another number and capability: the
    // links of the story, the task and the change resolve by identifier; only the location of
    // the file is warned about (R14).
    let documents: Vec<Value> = project()
        .into_iter()
        .map(|document| {
            if document["id"] == "SPEC-0001" {
                stored(document, "specs/0042-time-log/spec.md", 1)
            } else {
                entry(document)
            }
        })
        .collect();
    let validation = validate(&read(documents));
    assert_eq!(validation.errors, []);
    assert_eq!(
        found(&validation.warnings),
        coded(codes::IDENTIFIER_LOCATION, "/id", Reason::Conflict)
    );
}

#[test]
fn ac7_a_story_linked_to_a_superseded_specification_gets_its_replacements_and_the_fix() {
    let story = story_with("STORY-0003", "specification", json!(["SPEC-0003"]));
    let validation = validated(vec![story]);
    let error = the_only_error(&validation);
    assert_eq!(
        found(&validation.errors),
        coded(codes::LINK_SUPERSEDED, "/specification/0", Reason::State)
    );
    assert_eq!(error.document(), after());
    assert!(error.message().contains("SPEC-0003"), "{error:?}");
    // One replacement: the values accepted, and the fix applicable as is (decided by the
    // maintainer on 2026-11-25).
    assert_eq!(error.expected(), Some(&[json!("SPEC-0001")][..]));
    assert_eq!(error.fix(), Some(&json!("SPEC-0001")));
}

#[test]
fn several_replacements_are_the_values_accepted_with_no_fix() {
    let replaced = item(epic(), "EPIC-0002");
    let replaced = with(replaced, "status", json!("superseded"));
    let replaced = with(replaced, "superseded_by", json!(["EPIC-0001", "EPIC-0003"]));
    let replacement = with(item(epic(), "EPIC-0003"), "status", json!("draft"));
    let replacement = with(replacement, "issue", Value::Null);
    let story = story_with("STORY-0002", "epic", json!("EPIC-0002"));
    let validation = validated(vec![replaced, replacement, story]);
    let error = the_only_error(&validation);
    assert_eq!(
        found(&validation.errors),
        coded(codes::LINK_SUPERSEDED, "/epic", Reason::State)
    );
    assert_eq!(
        error.expected(),
        Some(&[json!("EPIC-0001"), json!("EPIC-0003")][..])
    );
    // No single value to apply as is.
    assert_eq!(error.fix(), None);
}

/// A draft milestone `M2` of the examples.
fn milestone_2() -> Value {
    let milestone = with(milestone(), "id", json!("M2"));
    let milestone = with(milestone, "status", json!("draft"));
    with(milestone, "tracker_number", Value::Null)
}

#[test]
fn ac14_a_task_whose_milestone_differs_from_that_of_its_epic_is_refused_naming_both() {
    let refused = task_with("TASK-0002", "approved", "milestone", json!("M2"));
    let validation = validated(vec![milestone_2(), refused]);
    let error = the_only_error(&validation);
    assert_eq!(
        found(&validation.errors),
        coded(codes::HIERARCHY_MISMATCH, "/milestone", Reason::Conflict)
    );
    assert_eq!(error.pointers(), Some(&[at("/epic")][..]));
    assert_eq!(error.expected(), Some(&[json!("M1")][..]));
    assert!(error.message().contains("M2"), "{error:?}");
    assert!(error.message().contains("M1"), "{error:?}");
    // The same task done passes: a terminal artifact is not changed again (R15 holds for a
    // non-terminal status).
    let done = task_with("TASK-0002", "done", "milestone", json!("M2"));
    let validation = validated(vec![milestone_2(), done]);
    assert_eq!(validation.errors, []);
    assert_eq!(validation.warnings, []);
}

#[test]
fn a_story_whose_milestone_differs_from_that_of_its_epic_is_refused() {
    let story = story_with("STORY-0002", "milestone", json!("M2"));
    let validation = validated(vec![milestone_2(), story]);
    let error = the_only_error(&validation);
    assert_eq!(
        found(&validation.errors),
        coded(codes::HIERARCHY_MISMATCH, "/milestone", Reason::Conflict)
    );
    assert_eq!(error.expected(), Some(&[json!("M1")][..]));
}

#[test]
fn a_task_whose_epic_differs_from_that_of_its_story_is_refused_naming_both() {
    let other = with(item(epic(), "EPIC-0002"), "status", json!("approved"));
    let other = with(other, "issue", Value::Null);
    let refused = task_with("TASK-0002", "approved", "epic", json!("EPIC-0002"));
    let validation = validated(vec![other, refused]);
    let error = the_only_error(&validation);
    assert_eq!(
        found(&validation.errors),
        coded(codes::HIERARCHY_MISMATCH, "/epic", Reason::Conflict)
    );
    assert_eq!(error.pointers(), Some(&[at("/story")][..]));
    assert_eq!(error.expected(), Some(&[json!("EPIC-0001")][..]));
}

#[test]
fn ac24_a_done_task_citing_a_superseded_specification_passes_with_a_warning() {
    let cited = json!([{"spec": "SPEC-0003", "requirements": ["R1"]}]);
    let done = task_with("TASK-0002", "done", "specification", cited);
    let validation = validated(vec![done]);
    let warning = only_warning(&validation);
    assert_eq!(
        found(&validation.warnings),
        coded(
            codes::LINK_BROKEN_TERMINAL,
            "/specification/0/spec",
            Reason::State
        )
    );
    assert_eq!(warning.document(), after());
    assert!(warning.message().contains("SPEC-0003"), "{warning:?}");
    assert_eq!(warning.expected(), Some(&[json!("SPEC-0001")][..]));
    assert_eq!(warning.fix(), Some(&json!("SPEC-0001")));
}

#[test]
fn a_missing_link_of_a_done_task_is_a_warning_with_the_reason_missing() {
    let done = task_with("TASK-0002", "done", "story", json!("STORY-0009"));
    let validation = validated(vec![done]);
    only_warning(&validation);
    assert_eq!(
        found(&validation.warnings),
        coded(codes::LINK_BROKEN_TERMINAL, "/story", Reason::Missing)
    );
}

#[test]
fn a_removed_requirement_is_not_in_force_for_a_task_and_a_warning_once_it_is_done() {
    let cited = json!([{"spec": "SPEC-0001", "requirements": ["R4"]}]);
    let approved = task_with("TASK-0002", "approved", "specification", cited.clone());
    let validation = validated(vec![approved]);
    let error = the_only_error(&validation);
    assert_eq!(
        found(&validation.errors),
        coded(
            codes::LINK_SUPERSEDED,
            "/specification/0/requirements/0",
            Reason::State
        )
    );
    assert_eq!((error.fix(), error.expected()), (None, None));
    let done = task_with("TASK-0002", "done", "specification", cited);
    let validation = validated(vec![done]);
    only_warning(&validation);
    assert_eq!(
        found(&validation.warnings),
        coded(
            codes::LINK_BROKEN_TERMINAL,
            "/specification/0/requirements/0",
            Reason::State
        )
    );
}

#[test]
fn an_operation_of_a_change_names_an_active_requirement_of_its_specification() {
    let operations = json!([
        {"op": "modified", "requirement": "R7", "text": "THE TOOL SHALL round durations."},
        {"op": "removed", "requirement": "R4"},
        {"op": "renamed", "requirement": "R3", "name": "Positive"},
        {"op": "added", "text": "THE TOOL SHALL export a month."}
    ]);
    let delta = with(
        with(change(), "id", json!("CHANGE-0002")),
        "operations",
        operations,
    );
    let validation = validated(vec![delta]);
    assert_eq!(
        found(&validation.errors),
        [
            (
                codes::LINK_MISSING,
                "/operations/0/requirement".to_owned(),
                Reason::Missing
            ),
            (
                codes::LINK_SUPERSEDED,
                "/operations/1/requirement".to_owned(),
                Reason::State
            ),
        ]
    );
}

#[test]
fn an_acceptance_criterion_names_a_requirement_of_its_own_specification() {
    let criteria = json!([{"id": "AC1", "requirements": ["R1", "R9"], "text": "Given.",
                           "verification": ["tdd"]}]);
    let draft = with(specification(), "id", json!("SPEC-0005"));
    let draft = with(
        with(draft, "status", json!("draft")),
        "acceptance_criteria",
        criteria,
    );
    let validation = validated(vec![draft]);
    assert_eq!(
        found(&validation.errors),
        coded(
            codes::LINK_MISSING,
            "/acceptance_criteria/0/requirements/1",
            Reason::Missing
        )
    );
}

#[test]
fn every_link_of_every_type_resolves_through_the_set() {
    // One missing target per type: the member of the link and, for a hand-over record, which is
    // never changed again, a warning.
    let missing = [
        (
            with(council_report(), "id", json!("COUNCIL-0002")),
            "brief",
            json!("BRIEF-0009"),
            "/brief",
        ),
        (
            with(
                with(gate(), "id", json!("GATE-0002")),
                "status",
                json!("draft"),
            ),
            "subject",
            json!("COUNCIL-0009"),
            "/subject",
        ),
        (
            milestone_2(),
            "council_report",
            json!("COUNCIL-0009"),
            "/council_report",
        ),
        (
            with(item(epic(), "EPIC-0002"), "status", json!("approved")),
            "milestone",
            json!("M9"),
            "/milestone",
        ),
        (
            with(specification(), "id", json!("SPEC-0005")),
            "stories",
            json!(["STORY-0009"]),
            "/stories/0",
        ),
        (
            with(specification(), "id", json!("SPEC-0005")),
            "milestone",
            json!("M9"),
            "/milestone",
        ),
        (
            with(change(), "id", json!("CHANGE-0002")),
            "tasks",
            json!(["TASK-0009"]),
            "/tasks/0",
        ),
        (
            with(change(), "id", json!("CHANGE-0002")),
            "specification",
            json!("SPEC-0009"),
            "/specification",
        ),
        (
            task_with("TASK-0002", "draft", "depends_on", json!(["TASK-0009"])),
            "issue",
            Value::Null,
            "/depends_on/0",
        ),
        (
            task_with("TASK-0002", "draft", "expanded_from", json!("TASK-0009")),
            "issue",
            Value::Null,
            "/expanded_from",
        ),
    ];
    for (document, member, value, pointer) in missing {
        let document = with(document, member, value);
        let validation = validated(vec![document.clone()]);
        assert_eq!(
            found(&validation.errors),
            coded(codes::LINK_MISSING, pointer, Reason::Missing),
            "{document}"
        );
        assert_eq!(validation.warnings, [], "{document}");
    }
    let record = with(handover(), "id", json!("HANDOVER-12-2"));
    let validation = validated(vec![with(record, "task", json!("TASK-0009"))]);
    only_warning(&validation);
    assert_eq!(
        found(&validation.warnings),
        coded(codes::LINK_BROKEN_TERMINAL, "/task", Reason::Missing)
    );
}

#[test]
fn links_that_name_a_superseded_task_by_design_accept_it() {
    // A subtask names the task it was expanded from, which is superseded by it; a dependency
    // on a superseded task is refused by the check of parts (R18), not here.
    let expanded = task_with(
        "TASK-0004",
        "superseded",
        "superseded_by",
        json!(["TASK-0005"]),
    );
    let subtask = task_with("TASK-0005", "draft", "expanded_from", json!("TASK-0004"));
    let subtask = with(
        with(subtask, "depends_on", json!(["TASK-0004"])),
        "issue",
        Value::Null,
    );
    let validation = validated(vec![expanded, subtask]);
    // No error of a link: the only error is that of the parts, at the dependency, and the
    // subtask, the only replacement of the task, is not offered to itself.
    let error = the_only_error(&validation);
    assert_eq!(
        found(&validation.errors),
        coded(codes::PART_INVALID, "/depends_on/0", Reason::Conflict)
    );
    assert_eq!((error.fix(), error.expected()), (None, None));
}

#[test]
fn an_artifact_refused_by_its_schema_has_no_link_checked_and_still_resolves() {
    // The story has an unknown member: its broken epic is not reported, and the task that
    // serves it finds it.
    let story = story_with("STORY-0002", "zz", json!(1));
    let story = with(story, "epic", json!("EPIC-0009"));
    let serving = task_with("TASK-0002", "approved", "story", json!("STORY-0002"));
    let validation = validated(vec![story, serving]);
    assert_eq!(
        found(&validation.errors),
        coded(codes::ARTIFACT_SCHEMA, "/zz", Reason::UnknownField)
    );
}

#[test]
fn the_link_errors_of_an_artifact_come_in_the_order_of_its_members() {
    let task = task_with("TASK-0002", "approved", "depends_on", json!(["TASK-0009"]));
    let task = with(
        with(task, "epic", json!("EPIC-0009")),
        "expanded_from",
        json!("TASK-0008"),
    );
    let task = with(task, "milestone", json!("M9"));
    let cited = json!([{"spec": "SPEC-0001", "requirements": ["R9"]},
                       {"spec": "SPEC-0042", "requirements": ["R1"]}]);
    let task = with(
        with(task, "specification", cited),
        "story",
        json!("STORY-0009"),
    );
    let validation = validated(vec![task]);
    let pointers: Vec<String> = validation
        .errors
        .iter()
        .map(|e| e.pointer().to_string())
        .collect();
    assert_eq!(
        pointers,
        [
            "/depends_on/0",
            "/epic",
            "/expanded_from",
            "/milestone",
            "/specification/0/requirements/0",
            "/specification/1/spec",
            "/story",
        ]
    );
    assert!(validation.errors.iter().all(|e| e.document() == after()));
}

/// The project with a living specification `SPEC-0005` whose `related` is `related`, validated
/// with the ADRs `adrs`.
fn cited(related: Value, adrs: &[u64]) -> Validation {
    let draft = with(specification(), "id", json!("SPEC-0005"));
    let draft = with(draft, "related", related);
    let documents: Vec<Value> = project().into_iter().chain([draft]).map(entry).collect();
    validate(&read_input(json!({"documents": documents, "adrs": adrs})))
}

#[test]
fn a_citation_of_an_adr_the_project_does_not_have_is_a_warning_at_its_entry() {
    let related = json!([
        "ADR 0008",
        "#6",
        "ADR 0042 and ADR 9",
        "SPEC-0001",
        "MADR 0042",
        "ADR 0042x",
        "ADR"
    ]);
    let validation = cited(related, &[8, 9]);
    assert_eq!(validation.errors, []);
    assert!(validation.checks.contains(&checks::ADR_CITATIONS));
    assert!(
        validation
            .not_done
            .iter()
            .all(|n| n.check() != &checks::ADR_CITATIONS)
    );
    assert_eq!(
        found(&validation.warnings),
        coded(codes::ADR_CITATION_MISSING, "/related/2", Reason::Missing)
    );
    let warning = &validation.warnings[0];
    assert_eq!(warning.document(), after());
    assert!(warning.message().contains("ADR 0042"), "{warning:?}");
    // `ADR 9` and `ADR 0009` cite one ADR; with none, every citation is warned about, in order.
    let validation = cited(json!(["ADR 0009", "ADR 8"]), &[]);
    assert_eq!(
        found(&validation.warnings),
        [
            (
                codes::ADR_CITATION_MISSING,
                "/related/0".to_owned(),
                Reason::Missing
            ),
            (
                codes::ADR_CITATION_MISSING,
                "/related/1".to_owned(),
                Reason::Missing
            ),
        ]
    );
}

/// An epic of the examples called `id`, superseded by `replacements`.
fn superseded_epic(id: &str, replacements: Value) -> Value {
    let epic = with(item(epic(), id), "status", json!("superseded"));
    with(epic, "superseded_by", replacements)
}

#[test]
fn a_replacement_must_not_be_superseded_and_a_chain_is_a_warning_of_its_holder() {
    // Decided by the maintainer on 2026-11-25: `superseded_by` names artifacts in force. Its
    // holder is superseded, so terminal: a self-reference or a cycle is a warning.
    let validation = validated(vec![superseded_epic("EPIC-0002", json!(["EPIC-0002"]))]);
    let warning = only_warning(&validation);
    assert_eq!(
        found(&validation.warnings),
        coded(
            codes::LINK_BROKEN_TERMINAL,
            "/superseded_by/0",
            Reason::State
        )
    );
    // The only replacement of the target is the target itself: no value to offer.
    assert_eq!((warning.fix(), warning.expected()), (None, None));
    let cycle = vec![
        superseded_epic("EPIC-0002", json!(["EPIC-0003"])),
        superseded_epic("EPIC-0003", json!(["EPIC-0002"])),
    ];
    let validation = validated(cycle);
    assert_eq!(validation.errors, []);
    let warned: Vec<(Option<u64>, String)> = validation
        .warnings
        .iter()
        .map(|w| (w.document(), w.pointer().to_string()))
        .collect();
    let first = after().unwrap();
    assert_eq!(
        warned,
        [
            (Some(first), "/superseded_by/0".to_owned()),
            (Some(first + 1), "/superseded_by/0".to_owned()),
        ]
    );
    // Each epic of the cycle is the only replacement of the other: offering it would make the
    // epic name itself, so neither offers a value.
    for warning in &validation.warnings {
        assert_eq!((warning.fix(), warning.expected()), (None, None));
    }
    // A draft epic is not terminal: its replacement, itself superseded, is an error.
    let draft = with(item(epic(), "EPIC-0004"), "status", json!("draft"));
    let draft = with(
        with(draft, "issue", Value::Null),
        "superseded_by",
        json!(["EPIC-0002"]),
    );
    let replaced = superseded_epic("EPIC-0002", json!(["EPIC-0001"]));
    let validation = validated(vec![replaced, draft]);
    let error = the_only_error(&validation);
    assert_eq!(
        found(&validation.errors),
        coded(codes::LINK_SUPERSEDED, "/superseded_by/0", Reason::State)
    );
    assert_eq!(error.fix(), Some(&json!("EPIC-0001")));
}

#[test]
fn a_superseded_artifact_with_a_missing_link_gives_a_warning() {
    let replaced = with(
        superseded_epic("EPIC-0002", json!(["EPIC-0001"])),
        "milestone",
        json!("M9"),
    );
    let validation = validated(vec![replaced]);
    only_warning(&validation);
    assert_eq!(
        found(&validation.warnings),
        coded(codes::LINK_BROKEN_TERMINAL, "/milestone", Reason::Missing)
    );
}

#[test]
fn at_one_member_the_link_error_comes_before_the_hierarchy_error() {
    // The milestone M9 does not exist, and differs from M1, the milestone of the epic.
    let task = task_with("TASK-0002", "approved", "milestone", json!("M9"));
    let validation = validated(vec![task]);
    assert_eq!(
        found(&validation.errors),
        [
            (
                codes::LINK_MISSING,
                "/milestone".to_owned(),
                Reason::Missing
            ),
            (
                codes::HIERARCHY_MISMATCH,
                "/milestone".to_owned(),
                Reason::Conflict
            ),
        ]
    );
}

#[test]
fn a_markdown_specification_with_an_error_of_its_own_is_not_checked_for_links_but_found() {
    // Decided by the maintainer on 2026-11-25: one rule for every error of a document, of its
    // schema or of the normalisation of its Markdown. The section Problem is given twice and
    // the milestone M9 does not exist: only the repeated section is reported, and the story and
    // the task that link the specification still find it.
    let text = crate::artifacts::markdown::tests::EXAMPLE
        .replacen("| Milestone | M1 ", "| Milestone | M9 ", 1)
        .replacen("## Interfaces", "## Problem\nAgain.\n\n## Interfaces", 1);
    assert_ne!(text, crate::artifacts::markdown::tests::EXAMPLE);
    let markdown = json!({"format": "markdown", "text": text,
                          "origin": {"file": "specs/0001-time-entries/spec.md"}});
    let validation = validate(&read(linked(vec![markdown])));
    assert_eq!(
        found(&validation.errors),
        coded(codes::ARTIFACT_SCHEMA, "/problem", Reason::Duplicate)
    );
    assert_eq!(validation.warnings, []);
    // Without the repeated section, the broken milestone is reported.
    let text = crate::artifacts::markdown::tests::EXAMPLE.replacen(
        "| Milestone | M1 ",
        "| Milestone | M9 ",
        1,
    );
    let markdown = json!({"format": "markdown", "text": text,
                          "origin": {"file": "specs/0001-time-entries/spec.md"}});
    let validation = validate(&read(linked(vec![markdown])));
    assert_eq!(
        found(&validation.errors),
        coded(codes::LINK_MISSING, "/milestone", Reason::Missing)
    );
}

#[test]
fn bare_numbers_after_a_citation_of_an_adr_cite_adrs_too() {
    // Decided by the maintainer on 2026-11-25: `ADR 0002, 0003` cites two ADRs, in one entry
    // or split into entries as the header row of a Markdown specification is.
    let related = json!([
        "ADR 0002, 0003",
        "#6",
        "0004",
        "ADR 0005",
        "0006",
        "SPEC-0001",
        "0007"
    ]);
    let validation = cited(related, &[2]);
    let warned: Vec<(String, String)> = validation
        .warnings
        .iter()
        .map(|w| (w.pointer().to_string(), w.message().to_owned()))
        .collect();
    let at: Vec<&str> = warned.iter().map(|(p, _)| p.as_str()).collect();
    // 0003 in the first entry, 0005 and 0006; not 0004 after `#6`, nor 0007 after `SPEC-0001`.
    assert_eq!(at, ["/related/0", "/related/3", "/related/4"]);
    assert!(warned[0].1.contains("ADR 0003"), "{warned:?}");
    assert!(warned[2].1.contains("ADR 0006"), "{warned:?}");
}

#[test]
fn every_citation_of_a_large_set_is_checked_against_the_requirements_of_its_specification() {
    // Correctness on a large set: 3,000 requirements cited by one task and named by one
    // criterion, with one missing at the end, all found. The work is bounded by the structure,
    // one map of the requirements of each specification built once (review B1), which this
    // test does not measure.
    const COUNT: usize = 3_000;
    let ids: Vec<String> = (1..=COUNT).map(|k| format!("R{k}")).collect();
    let requirements: Vec<Value> = ids
        .iter()
        .map(|id| json!({"id": id, "text": "THE TOOL SHALL record."}))
        .collect();
    let mut cited_ids: Vec<Value> = ids.iter().map(|id| json!(id)).collect();
    cited_ids.push(json!(format!("R{}", COUNT + 1)));
    let large = with(specification(), "id", json!("SPEC-0005"));
    let large = with(
        with(large, "status", json!("draft")),
        "requirements",
        json!(requirements),
    );
    let criteria = json!([{"id": "AC1", "requirements": ids, "text": "Given.",
                           "verification": ["tdd"]}]);
    let large = with(large, "acceptance_criteria", criteria);
    let reference = json!([{"spec": "SPEC-0005", "requirements": cited_ids}]);
    let task = task_with("TASK-0002", "approved", "specification", reference);
    let validation = validated(vec![large, task]);
    let pointer = format!("/specification/0/requirements/{COUNT}");
    assert_eq!(
        found(&validation.errors),
        coded(codes::LINK_MISSING, &pointer, Reason::Missing)
    );
}

#[test]
fn a_word_that_opens_a_bracket_ends_a_run_of_cited_adrs() {
    // Review N6: a year in brackets after a citation is not an ADR; a citation in brackets is.
    let validation = cited(json!(["ADR 0014 (2026)", "(ADR 0015)", "[2027]"]), &[]);
    let warned: Vec<(String, String)> = validation
        .warnings
        .iter()
        .map(|w| (w.pointer().to_string(), w.message().to_owned()))
        .collect();
    let at: Vec<&str> = warned.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(at, ["/related/0", "/related/1"]);
    assert!(warned[0].1.contains("ADR 0014"), "{warned:?}");
    assert!(warned[1].1.contains("ADR 0015"), "{warned:?}");
}

#[test]
fn a_requirement_listed_twice_is_read_as_its_first_entry() {
    // Review N4: the first entry of a duplicated requirement identifier wins, as the first
    // carrier of an identifier does; here R1 active, then R1 removed.
    let mut requirements = specification()["requirements"].clone();
    requirements
        .as_array_mut()
        .unwrap()
        .push(json!({"id": "R1", "text": "Again.", "status": "removed"}));
    let draft = with(specification(), "id", json!("SPEC-0005"));
    let draft = with(
        with(draft, "status", json!("draft")),
        "requirements",
        requirements,
    );
    let cited = json!([{"spec": "SPEC-0005", "requirements": ["R1"]}]);
    let task = task_with("TASK-0002", "approved", "specification", cited);
    let validation = validated(vec![draft, task]);
    // The task cites an active requirement: no error of a link. The duplicate itself is a
    // fault of the parts (R18).
    assert_eq!(
        found(&validation.errors),
        coded(codes::PART_INVALID, "/requirements/4/id", Reason::Duplicate)
    );
}
