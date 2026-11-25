//! Tests of the parts of artifacts and of the operations of change deltas (SPEC-0001 T13: R18,
//! R20; AC27, AC29). The invalid examples are contract cases in `tests/contract/validate/v1/`.

use serde_json::{Value, json};
use wayloop_schemas::artifact::{Specification, Verification};
use wayloop_schemas::examples::{change, council_report, specification, story, task};
use wayloop_schemas::value::RequirementId;

use super::{components, next_requirement};
use crate::artifacts::tests::{found, project, the_only_error, titled, validated, with};
use crate::artifacts::{Validation, checks, codes};
use crate::{Code, Error, Reason};

/// The index of the `n`-th document after the project.
fn after(n: u64) -> Option<u64> {
    u64::try_from(project().len()).ok().map(|first| first + n)
}

/// The errors of `validation`, which has no failure and no warning.
fn errors(validation: &Validation) -> &[Error] {
    assert_eq!(validation.failures, []);
    assert_eq!(validation.warnings, []);
    &validation.errors
}

fn coded(code: Code, pointer: &str, reason: Reason) -> (Code, String, Reason) {
    (code, pointer.to_owned(), reason)
}

/// One error `part_invalid` with the reason `conflict` at each dependency of `at`.
fn conflicts(at: &[&str]) -> Vec<(Code, String, Reason)> {
    at.iter()
        .map(|p| coded(codes::PART_INVALID, p, Reason::Conflict))
        .collect()
}

fn pointers(error: &Error) -> Vec<String> {
    error
        .pointers()
        .unwrap_or_default()
        .iter()
        .map(ToString::to_string)
        .collect()
}

/// A task of the examples called `id` (`TASK-<n>`), approved, with a title of its own (R8).
fn task_called(id: &str) -> Value {
    let sequence: u32 = id.rsplit('-').next().unwrap().parse().unwrap();
    let task = titled(with(task(), "id", json!(id)), sequence);
    // An approved task needs no tracker number; the example is implementing.
    let task = with(task, "status", json!("approved"));
    with(task, "issue", Value::Null)
}

/// The task `id` depending on `targets`.
fn depending(id: &str, targets: &[&str]) -> Value {
    with(task_called(id), "depends_on", json!(targets))
}

/// The task `id`, superseded by `by`.
fn superseded_task(id: &str, by: &[&str]) -> Value {
    let task = with(task_called(id), "status", json!("superseded"));
    with(task, "superseded_by", json!(by))
}

/// The council report of the examples called `COUNCIL-0002`, with `member` set to `value`.
fn council_with(member: &str, value: Value) -> Value {
    with(
        with(council_report(), "id", json!("COUNCIL-0002")),
        member,
        value,
    )
}

fn decision(id: &str) -> Value {
    json!({"id": id, "statement": "Store entries locally.", "reason": "Offline use."})
}

fn question(id: &str, recommendation: &str, answer: Option<&str>) -> Value {
    let mut question = json!({
        "id": id,
        "question": "Is synchronisation in the first milestone?",
        "options": ["Yes", "No", "Yes"],
        "recommendation": recommendation
    });
    if let Some(answer) = answer {
        question["answer"] = json!(answer);
    }
    question
}

/// The change delta of the examples called `CHANGE-0002`, with the operations `operations`.
fn change_with(operations: Value) -> Value {
    with(
        with(change(), "id", json!("CHANGE-0002")),
        "operations",
        operations,
    )
}

// The check -----------------------------------------------------------------------------------

#[test]
fn the_example_project_has_valid_parts_and_the_check_runs() {
    let validation = validated(Vec::new());
    assert_eq!(errors(&validation), []);
    assert!(validation.checks.contains(&checks::PARTS));
    let not_done: Vec<&str> = validation.not_done.iter().map(|n| n.check().id()).collect();
    assert!(!not_done.contains(&checks::PARTS.id()), "{not_done:?}");
    // The check runs right after the links, in the order of the stages of the plan.
    let links = validation.checks.iter().position(|c| *c == checks::LINKS);
    let parts = validation.checks.iter().position(|c| *c == checks::PARTS);
    assert_eq!(parts, links.map(|l| l + 1));
}

// R18: duplicate part identifiers --------------------------------------------------------------

#[test]
fn ac27_a_duplicate_decision_is_refused_at_its_second_carrier_naming_the_first() {
    let report = council_with("decisions", json!([decision("D1"), decision("D1")]));
    let validation = validated(vec![report]);
    let error = the_only_error(&validation);
    assert_eq!(
        found(&validation.errors),
        [coded(
            codes::PART_INVALID,
            "/decisions/1/id",
            Reason::Duplicate
        )]
    );
    assert_eq!(error.document(), after(0));
    assert_eq!(pointers(error), ["/decisions/0/id", "/decisions/1/id"]);
    assert!(error.message().contains("D1"), "{error:?}");
    assert_eq!(error.fix(), None);
    assert_eq!(error.related(), None);
}

#[test]
fn every_carrier_after_the_first_is_refused_and_names_only_the_first_and_itself() {
    // Each error stays the size of one condition, so that many duplicates give a report
    // linear in their number.
    let decisions = json!([
        decision("D1"),
        decision("D2"),
        decision("D1"),
        decision("D1")
    ]);
    let validation = validated(vec![council_with("decisions", decisions)]);
    assert_eq!(
        found(errors(&validation)),
        [
            coded(codes::PART_INVALID, "/decisions/2/id", Reason::Duplicate),
            coded(codes::PART_INVALID, "/decisions/3/id", Reason::Duplicate),
        ]
    );
    assert_eq!(
        pointers(&validation.errors[1]),
        ["/decisions/0/id", "/decisions/3/id"]
    );
}

#[test]
fn duplicate_parts_of_a_specification_and_of_a_story_are_refused_in_the_order_of_their_members() {
    let mut spec = with(specification(), "id", json!("SPEC-0002"));
    spec["requirements"][1]["id"] = json!("R1");
    spec["acceptance_criteria"][1]["id"] = json!("AC1");
    spec["open_questions"] = json!([
        {"id": "Q1", "text": "First?", "resolved": false},
        {"text": "Without an identifier.", "resolved": false},
        {"text": "Another without one.", "resolved": true},
        {"id": "Q1", "text": "Again?", "resolved": false}
    ]);
    // The criteria cite R2, which the second requirement no longer is.
    spec["acceptance_criteria"][0]["requirements"] = json!(["R1"]);
    let mut story = with(story(), "id", json!("STORY-0002"));
    let criterion = story["acceptance_criteria"][0].clone();
    story["acceptance_criteria"] = json!([criterion.clone(), criterion]);
    let validation = validated(vec![spec, story]);
    assert_eq!(
        found(errors(&validation)),
        [
            coded(
                codes::PART_INVALID,
                "/acceptance_criteria/1/id",
                Reason::Duplicate
            ),
            coded(
                codes::PART_INVALID,
                "/open_questions/3/id",
                Reason::Duplicate
            ),
            coded(codes::PART_INVALID, "/requirements/1/id", Reason::Duplicate),
            coded(
                codes::PART_INVALID,
                "/acceptance_criteria/1/id",
                Reason::Duplicate
            ),
        ]
    );
    let documents: Vec<Option<u64>> = validation.errors.iter().map(Error::document).collect();
    assert_eq!(documents, [after(0), after(0), after(0), after(1)]);
    assert_eq!(
        pointers(&validation.errors[1]),
        ["/open_questions/0/id", "/open_questions/3/id"]
    );
}

// R18: dissent, recommendation and answer -------------------------------------------------------

#[test]
fn ac27_a_dissent_naming_a_decision_the_report_does_not_have_is_missing() {
    let report = council_with(
        "dissent",
        json!([{"role": "critic", "position": "Too early.", "decision": "D4"}]),
    );
    let validation = validated(vec![report]);
    let error = the_only_error(&validation);
    assert_eq!(
        found(&validation.errors),
        [coded(
            codes::PART_INVALID,
            "/dissent/0/decision",
            Reason::Missing
        )]
    );
    // The decisions are the values accepted: the error points to them, without copying them,
    // so that its size does not grow with the report.
    assert_eq!(pointers(error), ["/decisions"]);
    assert_eq!(error.expected(), None);
    assert_eq!(error.fix(), None);
    assert!(error.message().contains("D4"), "{error:?}");
}

#[test]
fn ac27_an_answer_or_a_recommendation_outside_the_options_is_refused_with_the_options() {
    let questions = json!([
        question("Q1", "Maybe", Some("Later")),
        question("Q2", "No", None),
        question("Q2", "No", Some("Yes"))
    ]);
    let validation = validated(vec![council_with("questions", questions)]);
    // Within a question, its members in the byte order of their keys: answer, id,
    // recommendation.
    assert_eq!(
        found(errors(&validation)),
        [
            coded(codes::PART_INVALID, "/questions/0/answer", Reason::Enum),
            coded(
                codes::PART_INVALID,
                "/questions/0/recommendation",
                Reason::Enum
            ),
            coded(codes::PART_INVALID, "/questions/2/id", Reason::Duplicate),
        ]
    );
    let answer = &validation.errors[0];
    // Each option once, in the order written.
    assert_eq!(answer.expected(), Some(&[json!("Yes"), json!("No")][..]));
    assert_eq!(pointers(answer), ["/questions/0/options"]);
    assert_eq!(answer.fix(), None);
    assert_eq!(
        validation.errors[1].expected(),
        Some(&[json!("Yes"), json!("No")][..])
    );
}

#[test]
fn a_report_with_a_schema_error_has_no_check_of_its_parts() {
    // Its parts cannot be read in its contract form; the schema error is the report.
    let report = council_with("decisions", json!([decision("D1"), decision("D1")]));
    let report = with(report, "mode", json!("one-mind"));
    let validation = validated(vec![report]);
    assert_eq!(
        found(errors(&validation)),
        [coded(codes::ARTIFACT_SCHEMA, "/mode", Reason::Enum)]
    );
}

// R18: dependencies of tasks -------------------------------------------------------------------

#[test]
fn ac27_a_task_depending_on_itself_is_a_conflict_with_its_identifier() {
    let validation = validated(vec![depending("TASK-0002", &["TASK-0001", "TASK-0002"])]);
    let error = the_only_error(&validation);
    assert_eq!(
        found(&validation.errors),
        [coded(
            codes::PART_INVALID,
            "/depends_on/1",
            Reason::Conflict
        )]
    );
    assert_eq!(error.document(), after(0));
    assert_eq!(pointers(error), ["/id"]);
    assert_eq!(error.related(), None);
}

#[test]
fn a_dependency_on_a_superseded_task_is_a_conflict_with_its_replacement() {
    let validation = validated(vec![
        depending("TASK-0002", &["TASK-0003", "TASK-0005"]),
        superseded_task("TASK-0003", &["TASK-0004"]),
        task_called("TASK-0004"),
        superseded_task("TASK-0005", &["TASK-0004", "TASK-0006"]),
        task_called("TASK-0006"),
    ]);
    assert_eq!(
        found(errors(&validation)),
        [
            coded(codes::PART_INVALID, "/depends_on/0", Reason::Conflict),
            coded(codes::PART_INVALID, "/depends_on/1", Reason::Conflict),
        ]
    );
    let (one, two) = (&validation.errors[0], &validation.errors[1]);
    // As for a link to a superseded artifact (R4): the replacements are the values accepted,
    // and the fix when there is exactly one.
    assert_eq!(one.expected(), Some(&[json!("TASK-0004")][..]));
    assert_eq!(one.fix(), Some(&json!("TASK-0004")));
    assert_eq!(
        two.expected(),
        Some(&[json!("TASK-0004"), json!("TASK-0006")][..])
    );
    assert_eq!(two.fix(), None);
    assert!(one.message().contains("TASK-0003"), "{one:?}");
}

#[test]
fn a_superseded_task_with_an_error_of_its_own_is_still_a_target() {
    let target = with(
        superseded_task("TASK-0003", &["TASK-0001"]),
        "colour",
        json!(1),
    );
    let validation = validated(vec![depending("TASK-0002", &["TASK-0003"]), target]);
    assert_eq!(
        found(errors(&validation)),
        [
            coded(codes::PART_INVALID, "/depends_on/0", Reason::Conflict),
            coded(codes::ARTIFACT_SCHEMA, "/colour", Reason::UnknownField),
        ]
    );
}

#[test]
fn ac27_each_dependency_of_a_cycle_of_three_tasks_is_a_conflict() {
    let validation = validated(vec![
        depending("TASK-0002", &["TASK-0003"]),
        depending("TASK-0003", &["TASK-0001", "TASK-0004"]),
        depending("TASK-0004", &["TASK-0002"]),
        // Out of the cycle: it depends on the cycle, which closes nothing.
        depending("TASK-0005", &["TASK-0002"]),
    ]);
    assert_eq!(
        found(errors(&validation)),
        conflicts(&["/depends_on/0", "/depends_on/1", "/depends_on/0"])
    );
    let documents: Vec<Option<u64>> = validation.errors.iter().map(Error::document).collect();
    assert_eq!(documents, [after(0), after(1), after(2)]);
    for error in &validation.errors {
        // Another document is involved, but the specification does not say that this error
        // names it in `related`.
        assert_eq!(error.related(), None);
        assert_eq!(error.pointers(), None);
    }
    assert!(
        validation.errors[0].message().contains("TASK-0003"),
        "{:?}",
        validation.errors[0]
    );
}

#[test]
fn a_cycle_of_two_with_a_repeated_dependency_refuses_each_entry() {
    let validation = validated(vec![
        depending("TASK-0002", &["TASK-0003", "TASK-0003"]),
        depending("TASK-0003", &["TASK-0002"]),
    ]);
    assert_eq!(
        found(errors(&validation)),
        conflicts(&["/depends_on/0", "/depends_on/1", "/depends_on/0"])
    );
}

#[test]
fn a_task_in_a_terminal_status_has_no_check_of_its_dependencies() {
    // A done or superseded task is not changed again (R3): what it depended on is history, as
    // for the hierarchy (R15). Nor does a dependency of it close a cycle.
    let done = with(
        depending("TASK-0002", &["TASK-0002", "TASK-0003"]),
        "status",
        json!("done"),
    );
    let done = with(done, "issue", json!(20));
    let superseded = with(
        superseded_task("TASK-0003", &["TASK-0004"]),
        "depends_on",
        json!(["TASK-0002", "TASK-0003"]),
    );
    let validation = validated(vec![done, superseded, task_called("TASK-0004")]);
    assert_eq!(errors(&validation), []);
}

#[test]
fn an_empty_or_unresolved_dependency_is_not_a_fault_of_the_parts() {
    let validation = validated(vec![
        depending("TASK-0002", &[]),
        depending("TASK-0003", &["TASK-0999"]),
    ]);
    // The missing target is a fault of the links only.
    assert_eq!(
        found(errors(&validation)),
        [coded(codes::LINK_MISSING, "/depends_on/0", Reason::Missing)]
    );
}

#[test]
fn the_errors_of_a_document_follow_the_stages_links_then_parts_then_titles() {
    let task = depending("TASK-0002", &["TASK-0002", "TASK-0999"]);
    let task = with(task, "title", json!("Record durations."));
    let validation = validated(vec![task]);
    assert_eq!(
        found(errors(&validation)),
        [
            coded(codes::LINK_MISSING, "/depends_on/1", Reason::Missing),
            coded(codes::PART_INVALID, "/depends_on/0", Reason::Conflict),
            coded(codes::TITLE_FORMAT, "/title", Reason::Syntax),
        ]
    );
}

/// `count` tasks, each depending on the next; the last on the first when `closed`.
fn chain(count: usize, closed: bool) -> Vec<Value> {
    let id = |n: usize| format!("TASK-{:04}", n + 2);
    (0..count)
        .map(|n| {
            let next = if n + 1 < count {
                Some(id(n + 1))
            } else if closed {
                Some(id(0))
            } else {
                None
            };
            let task = with(task_called("TASK-0002"), "id", json!(id(n)));
            let task = with(task, "title", json!(format!("Step {n}")));
            with(
                task,
                "depends_on",
                json!(next.into_iter().collect::<Vec<_>>()),
            )
        })
        .collect()
}

/// The errors of `validate` on `extra` after the project.
fn errors_after_project(extra: Vec<Value>) -> Vec<(Code, String, Reason)> {
    found(&validated(extra).errors)
}

#[test]
fn a_chain_and_a_cycle_of_10000_tasks_are_checked_without_recursion_in_linear_time() {
    // The pure crate reads no clock: a search quadratic in the tasks, or one recursive call per
    // task, would make this test time out or exhaust the stack of its thread.
    const TASKS: usize = 10_000;
    assert_eq!(errors_after_project(chain(TASKS, false)), []);
    let cycled = errors_after_project(chain(TASKS, true));
    assert_eq!(cycled.len(), TASKS);
    assert!(
        cycled
            .iter()
            .all(|e| *e == coded(codes::PART_INVALID, "/depends_on/0", Reason::Conflict))
    );
}

#[test]
fn the_components_of_a_graph_are_its_strongly_connected_parts() {
    // 0 -> 1 -> 2 -> 0 is a cycle; 3 -> 0 and 4 alone are not; 5 -> 5 is a loop.
    let graph = vec![vec![1], vec![2], vec![0], vec![0], vec![], vec![5]];
    let component = components(&graph);
    assert_eq!(component[0], component[1]);
    assert_eq!(component[1], component[2]);
    assert_ne!(component[3], component[0]);
    assert_ne!(component[4], component[0]);
    assert_ne!(component[3], component[4]);
    assert_eq!(component.len(), 6);
    assert_eq!(components(&[]), Vec::<usize>::new());
}

#[test]
fn a_path_of_a_million_nodes_needs_no_recursion() {
    const NODES: usize = 1_000_000;
    let path: Vec<Vec<usize>> = (0..NODES)
        .map(|n| if n + 1 < NODES { vec![n + 1] } else { vec![0] })
        .collect();
    let component = components(&path);
    assert!(component.iter().all(|c| *c == component[0]));
    let open: Vec<Vec<usize>> = (0..NODES)
        .map(|n| if n + 1 < NODES { vec![n + 1] } else { vec![] })
        .collect();
    let component = components(&open);
    let distinct: std::collections::BTreeSet<usize> = component.into_iter().collect();
    assert_eq!(distinct.len(), NODES);
}

// The verification of a task ----------------------------------------------------------------------

#[test]
fn a_task_verified_by_none_and_another_technique_is_refused_at_each_none() {
    let task = with(
        task_called("TASK-0002"),
        "verification",
        json!(["tdd", "none", "property", "none"]),
    );
    let validation = validated(vec![task]);
    assert_eq!(
        found(errors(&validation)),
        [
            coded(codes::ARTIFACT_SCHEMA, "/verification/1", Reason::Enum),
            coded(codes::ARTIFACT_SCHEMA, "/verification/3", Reason::Enum),
        ]
    );
    let error = &validation.errors[0];
    let techniques: Vec<Value> = Verification::ALL
        .into_iter()
        .filter(|v| *v != Verification::None)
        .map(|v| json!(v.id()))
        .collect();
    assert_eq!(error.expected(), Some(&techniques[..]));
    // The first technique beside it, so that the error does not grow with the list.
    assert_eq!(pointers(error), ["/verification/0"]);
    assert_eq!(error.fix(), None);
}

#[test]
fn a_task_verified_by_none_alone_or_repeated_passes() {
    let validation = validated(vec![
        with(task_called("TASK-0002"), "verification", json!(["none"])),
        with(
            task_called("TASK-0003"),
            "verification",
            json!(["none", "none"]),
        ),
        with(
            task_called("TASK-0004"),
            "verification",
            json!(["tdd", "tdd"]),
        ),
    ]);
    assert_eq!(errors(&validation), []);
}

// R20: change deltas ---------------------------------------------------------------------------

#[test]
fn ac29_two_operations_on_one_requirement_are_a_delta_conflict_naming_both() {
    let operations = json!([
        {"op": "modified", "requirement": "R2", "text": "THE TOOL SHALL store hours."},
        {"op": "removed", "requirement": "R2"}
    ]);
    let validation = validated(vec![change_with(operations)]);
    let error = the_only_error(&validation);
    assert_eq!(
        found(&validation.errors),
        [coded(
            codes::DELTA_CONFLICT,
            "/operations/1",
            Reason::Conflict
        )]
    );
    assert_eq!(pointers(error), ["/operations/0", "/operations/1"]);
    assert_eq!(error.document(), after(0));
    assert!(error.message().contains("R2"), "{error:?}");
}

#[test]
fn three_operations_on_one_requirement_refuse_the_second_and_the_third() {
    let operations = json!([
        {"op": "renamed", "requirement": "R1", "name": "Append"},
        {"op": "modified", "requirement": "R2", "text": "THE TOOL SHALL store hours."},
        {"op": "modified", "requirement": "R1", "text": "THE TOOL SHALL append."},
        {"op": "removed", "requirement": "R1"}
    ]);
    let validation = validated(vec![change_with(operations)]);
    assert_eq!(
        found(errors(&validation)),
        [
            coded(codes::DELTA_CONFLICT, "/operations/2", Reason::Conflict),
            coded(codes::DELTA_CONFLICT, "/operations/3", Reason::Conflict),
        ]
    );
    assert_eq!(
        pointers(&validation.errors[1]),
        ["/operations/0", "/operations/3"]
    );
}

#[test]
fn each_member_an_operation_refuses_is_an_unknown_field_with_its_op() {
    let operations = json!([
        {"op": "added", "requirement": "R1", "name": "Hours", "text": "THE TOOL SHALL count hours."},
        {"op": "removed", "requirement": "R3", "name": "Positive", "text": "Gone."},
        {"op": "renamed", "requirement": "R1", "name": "Append", "text": "New text."},
        {"op": "modified", "requirement": "R2", "name": "Seconds", "text": "THE TOOL SHALL store seconds."}
    ]);
    let validation = validated(vec![change_with(operations)]);
    // The requirement refused on `added` names no requirement: it is no conflict with the
    // rename of R1.
    assert_eq!(
        found(errors(&validation)),
        [
            coded(
                codes::ARTIFACT_SCHEMA,
                "/operations/0/requirement",
                Reason::UnknownField
            ),
            coded(
                codes::ARTIFACT_SCHEMA,
                "/operations/1/name",
                Reason::UnknownField
            ),
            coded(
                codes::ARTIFACT_SCHEMA,
                "/operations/1/text",
                Reason::UnknownField
            ),
            coded(
                codes::ARTIFACT_SCHEMA,
                "/operations/2/text",
                Reason::UnknownField
            ),
        ]
    );
    let expected_op = [
        "/operations/0/op",
        "/operations/1/op",
        "/operations/1/op",
        "/operations/2/op",
    ];
    for (error, op) in validation.errors.iter().zip(expected_op) {
        assert_eq!(pointers(error), [op]);
        assert_eq!(error.fix(), None);
        assert_eq!(error.expected(), None);
    }
}

#[test]
fn a_conflict_comes_before_the_members_of_its_operation() {
    let operations = json!([
        {"op": "removed", "requirement": "R2"},
        {"op": "removed", "requirement": "R2", "text": "Gone."}
    ]);
    let validation = validated(vec![change_with(operations)]);
    assert_eq!(
        found(errors(&validation)),
        [
            coded(codes::DELTA_CONFLICT, "/operations/1", Reason::Conflict),
            coded(
                codes::ARTIFACT_SCHEMA,
                "/operations/1/text",
                Reason::UnknownField
            ),
        ]
    );
}

// R20: the next requirement identifier -----------------------------------------------------------

fn ids(ids: &[&str]) -> Vec<RequirementId> {
    ids.iter().map(|id| RequirementId::new(*id)).collect()
}

#[test]
fn ac29_with_r1_to_r3_and_r4_removed_the_next_requirement_is_r5() {
    let specification: Specification = serde_json::from_value(specification()).unwrap();
    let next = next_requirement(specification.requirements.iter().map(|r| &r.id));
    assert_eq!(next.as_str(), "R5");
}

#[test]
fn the_next_requirement_follows_the_highest_number_ever_used() {
    let cases: [(&[&str], &str); 6] = [
        (&[], "R1"),
        (&["R9"], "R10"),
        // Numbers, not texts: R10 is above R9 and R3.
        (&["R3", "R10", "R9"], "R11"),
        (&["R199", "R2"], "R200"),
        // Beyond every integer type: the number is incremented as text.
        (
            &["R99999999999999999999999999"],
            "R100000000000000000000000000",
        ),
        // Identifiers outside the form, refused by the schema, are not counted.
        (&["R0", "R07", "X12", "R", "R1x", "r40"], "R1"),
    ];
    for (given, next) in cases {
        assert_eq!(next_requirement(&ids(given)).as_str(), next, "{given:?}");
    }
}
