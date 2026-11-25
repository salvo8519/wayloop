//! Tests of the links between artifacts and of the hierarchy of plan items (SPEC-0001 T12: R3,
//! R4, R9, R15; AC2, AC5, AC6, AC7, AC14, AC24). The invalid examples of AC1 that break a link
//! are contract cases in `tests/contract/validate/v1/`.

use serde_json::{Value, json};
use wayloop_schemas::NotDoneReason;
use wayloop_schemas::examples::{
    change, council_report, epic, gate, handover, milestone, specification, story, task,
};

use crate::artifacts::tests::{
    at, entry, found, project, read, read_input, stored, validate, with,
};
use crate::artifacts::{Validation, checks, codes};
use crate::{Code, Error, Reason};

/// The project with `extra` after it, validated; `extra` starts at the index `project().len()`.
fn validated(extra: Vec<Value>) -> Validation {
    let entries = project().into_iter().chain(extra).map(entry).collect();
    validate(&read(entries))
}

/// The index of the first document after the project.
fn after() -> Option<u64> {
    u64::try_from(project().len()).ok()
}

/// The only error of `validation`, which has no warning.
fn only_error(validation: &Validation) -> &Error {
    assert_eq!(validation.failures, []);
    assert_eq!(validation.warnings, []);
    let [error] = &validation.errors[..] else {
        panic!("{:?}", validation.errors);
    };
    error
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

/// A task of the examples called `id` in `status`, with `member` set to `value`.
fn task_with(id: &str, status: &str, member: &str, value: Value) -> Value {
    let task = with(with(task(), "id", json!(id)), "status", json!(status));
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
    let error = only_error(&validation);
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
    let error = only_error(&validation);
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
fn ac7_a_story_linked_to_a_superseded_specification_gets_its_replacements_as_the_fix() {
    let story = story_with("STORY-0003", "specification", json!(["SPEC-0003"]));
    let validation = validated(vec![story]);
    let error = only_error(&validation);
    assert_eq!(
        found(&validation.errors),
        coded(codes::LINK_SUPERSEDED, "/specification/0", Reason::State)
    );
    assert_eq!(error.document(), after());
    assert!(error.message().contains("SPEC-0003"), "{error:?}");
    assert_eq!(error.fix(), Some(&json!(["SPEC-0001"])));
}

#[test]
fn the_fix_of_a_superseded_target_lists_every_replacement() {
    let replaced = with(epic(), "id", json!("EPIC-0002"));
    let replaced = with(replaced, "status", json!("superseded"));
    let replaced = with(replaced, "superseded_by", json!(["EPIC-0001", "EPIC-0003"]));
    let replacement = with(
        with(epic(), "id", json!("EPIC-0003")),
        "status",
        json!("draft"),
    );
    let replacement = with(replacement, "issue", Value::Null);
    let story = story_with("STORY-0002", "epic", json!("EPIC-0002"));
    let validation = validated(vec![replaced, replacement, story]);
    let error = only_error(&validation);
    assert_eq!(
        found(&validation.errors),
        coded(codes::LINK_SUPERSEDED, "/epic", Reason::State)
    );
    assert_eq!(error.fix(), Some(&json!(["EPIC-0001", "EPIC-0003"])));
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
    let error = only_error(&validation);
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
    let error = only_error(&validation);
    assert_eq!(
        found(&validation.errors),
        coded(codes::HIERARCHY_MISMATCH, "/milestone", Reason::Conflict)
    );
    assert_eq!(error.expected(), Some(&[json!("M1")][..]));
}

#[test]
fn a_task_whose_epic_differs_from_that_of_its_story_is_refused_naming_both() {
    let other = with(
        with(epic(), "id", json!("EPIC-0002")),
        "status",
        json!("approved"),
    );
    let other = with(other, "issue", Value::Null);
    let refused = task_with("TASK-0002", "approved", "epic", json!("EPIC-0002"));
    let validation = validated(vec![other, refused]);
    let error = only_error(&validation);
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
    assert_eq!(warning.fix(), Some(&json!(["SPEC-0001"])));
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
    let error = only_error(&validation);
    assert_eq!(
        found(&validation.errors),
        coded(
            codes::LINK_SUPERSEDED,
            "/specification/0/requirements/0",
            Reason::State
        )
    );
    assert_eq!(error.fix(), None);
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
            with(
                with(epic(), "id", json!("EPIC-0002")),
                "status",
                json!("approved"),
            ),
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
    assert_eq!(validation.errors, []);
    assert_eq!(validation.warnings, []);
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
