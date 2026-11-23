//! The plan items of SPEC-0001 (milestone, epic, user story, task) and the gate issue on which
//! a person approves a brief or a council report, each built on [`Common`](super::Common).
//!
//! These artifacts live in the tracker, or as `plan/items/<id>.yaml` before their tracker item
//! exists; both forms are validated against the same schema (SPEC-0001, Where artifacts live).
//! The schema states what one document can state: the members, their forms and the members
//! required by a status or by a verification. Links to other artifacts, parts, statuses against
//! the default branch, titles and areas of the policy are checked across the set.

use schemars::{JsonSchema, Schema};
use serde::{Deserialize, Serialize};

use super::Common;
use crate::contract::{Contract, EPIC, GATE, MILESTONE, STORY, TASK, rules};
use crate::value::{
    BriefOrCouncilId, CouncilId, CriterionId, Date, EpicId, GateId, IssueNumber, MilestoneId,
    MilestoneNumber, PolicyName, RequirementId, SpecId, StoryId, TaskId, Text, TrackerHandle,
};

crate::contract_enum! {
    /// Status of a milestone, an epic, a user story or a task (SPEC-0001 R5); `draft` is the
    /// initial status. `blocked` is never one: it is a state of the issue cycle (SPEC-0002).
    pub enum PlanItemStatus {
        /// Proposed, not approved.
        Draft = "draft",
        /// Approved by a person; work may start.
        Approved = "approved",
        /// Work has started: a task of it is claimed.
        Implementing = "implementing",
        /// Delivered; terminal.
        Done = "done",
        /// Replaced by the items in `superseded_by`; terminal.
        Superseded = "superseded",
    }
}

impl PlanItemStatus {
    /// The statuses in which the tracker item exists, so its number is known.
    pub const STARTED: [PlanItemStatus; 2] = [PlanItemStatus::Implementing, PlanItemStatus::Done];
}

crate::contract_enum! {
    /// Status of a gate issue (SPEC-0001 R5); `draft` is its initial status.
    pub enum GateStatus {
        /// Open: the subject waits for the approval of a person.
        Draft = "draft",
        /// A person approved the subject on the issue, as recorded in the ledger; terminal.
        Done = "done",
        /// Replaced by the gates in `superseded_by`, or its subject was superseded; terminal.
        Superseded = "superseded",
    }
}

crate::contract_enum! {
    /// A verification technique of a task or of an acceptance criterion (SPEC-0009 R7).
    pub enum Verification {
        /// Test-driven development: a test written before the code.
        Tdd = "tdd",
        /// Behaviour-driven development: a scenario in the language of the user.
        Bdd = "bdd",
        /// A test of the whole system, through its interface.
        EndToEnd = "end_to_end",
        /// A property test over generated inputs.
        Property = "property",
        /// A contract test against a published schema or case.
        Contract = "contract",
        /// A measurement against a stated threshold.
        Benchmark = "benchmark",
        /// A judgement against a rubric, which is then required (SPEC-0009 R8).
        Judge = "judge",
        /// No verification.
        None = "none",
    }
}

crate::positive_integer! {
    /// Complexity score of a task, from 1 to 10 (SPEC-0013 R3).
    pub struct Complexity <= 10;
}

/// A gate issue (SPEC-0001, Gate issue): the tracker issue on which a person approves a brief or
/// a council report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(transform = gate_rules)]
pub struct Gate {
    /// The members of every artifact.
    #[serde(flatten)]
    pub common: Common<GateId, GateStatus>,
    /// The brief or council report submitted for approval.
    pub subject: BriefOrCouncilId,
    /// Tracker number of the issue, once the issue exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue: Option<IssueNumber>,
    /// Person who approved, as recorded in the ledger; required when `status` is `done`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approved_by: Option<TrackerHandle>,
}

/// A milestone of the roadmap (SPEC-0001, Milestone), stored in the description of its tracker
/// milestone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(transform = milestone_rules)]
pub struct Milestone {
    /// The members of every artifact.
    #[serde(flatten)]
    pub common: Common<MilestoneId, PlanItemStatus>,
    /// Observable condition that closes the milestone (release gate).
    pub gate: Text,
    /// Council report the milestone was generated from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub council_report: Option<CouncilId>,
    /// Planned date.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due: Option<Date>,
    /// Number of the tracker milestone, a sequence apart from issues; required when `status`
    /// is `implementing` or `done`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracker_number: Option<MilestoneNumber>,
}

/// An epic of a milestone (SPEC-0001, Epic), a tracker issue with the label `epic`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(transform = epic_rules)]
pub struct Epic {
    /// The members of every artifact.
    #[serde(flatten)]
    pub common: Common<EpicId, PlanItemStatus>,
    /// Milestone the epic belongs to.
    pub milestone: MilestoneId,
    /// What is true when the epic is done.
    pub outcome: Text,
    /// Area of the project policy, without the `area:` prefix; decides what can run in
    /// parallel.
    pub area: PolicyName,
    /// Tracker number of the issue; required when `status` is `implementing` or `done`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue: Option<IssueNumber>,
}

/// An acceptance criterion of a user story.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StoryCriterion {
    /// Identifier within the story, `AC<k>`, cited as `STORY-<seq> AC<k>`.
    pub id: CriterionId,
    /// The situation before the action.
    pub given: Text,
    /// The action.
    pub when: Text,
    /// The outcome expected.
    pub then: Text,
}

/// A user story of an epic (SPEC-0001, User story), a tracker sub-issue of its epic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(transform = story_rules)]
pub struct Story {
    /// The members of every artifact.
    #[serde(flatten)]
    pub common: Common<StoryId, PlanItemStatus>,
    /// Parent epic.
    pub epic: EpicId,
    /// Milestone of the story; must equal the milestone of its epic.
    pub milestone: MilestoneId,
    /// Role of the user.
    pub as_a: Text,
    /// Capability wanted.
    pub i_want: Text,
    /// Benefit.
    pub so_that: Text,
    /// Acceptance criteria; at least one.
    #[schemars(length(min = 1))]
    pub acceptance_criteria: Vec<StoryCriterion>,
    /// Specifications that turn the story into requirements, at least one; required when
    /// `status` is `approved`, `implementing` or `done`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1))]
    pub specification: Option<Vec<SpecId>>,
    /// Tracker number of the issue; required when `status` is `implementing` or `done`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue: Option<IssueNumber>,
}

/// A reference of a task to a living specification and to requirements of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SpecReference {
    /// The specification cited.
    pub spec: SpecId,
    /// Requirements of that specification the task implements; `[]` means none in particular.
    pub requirements: Vec<RequirementId>,
}

/// A task of an epic (SPEC-0001, Task), a tracker sub-issue of its epic closed by one pull
/// request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(transform = task_rules)]
pub struct Task {
    /// The members of every artifact.
    #[serde(flatten)]
    pub common: Common<TaskId, PlanItemStatus>,
    /// Parent epic.
    pub epic: EpicId,
    /// Milestone of the task; must equal the milestone of its epic.
    pub milestone: MilestoneId,
    /// Story the task serves; its epic must be `epic`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub story: Option<StoryId>,
    /// Specifications and requirements the task implements; `[]` means none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub specification: Option<Vec<SpecReference>>,
    /// Verification techniques; at least one.
    #[schemars(length(min = 1))]
    pub verification: Vec<Verification>,
    /// Rubric of the judge; required when `verification` contains `judge`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rubric: Option<Text>,
    /// Area of the project policy, without the `area:` prefix; decides what can run in
    /// parallel.
    pub area: PolicyName,
    /// Blocking tasks; `[]` means none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<TaskId>>,
    /// Complexity score, from 1 to 10.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub complexity: Option<Complexity>,
    /// Task this subtask was expanded from; that task is superseded by this one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expanded_from: Option<TaskId>,
    /// Whether the task can run in parallel with other tasks of the epic.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel: Option<bool>,
    /// Tracker number of the issue, cited by branches and pull requests; required when
    /// `status` is `implementing` or `done`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue: Option<IssueNumber>,
}

/// The rules of a gate the derive does not write: `approved_by` when `status` is `done`.
fn gate_rules(schema: &mut Schema) {
    rules::artifact(schema, GATE.name, GATE.version, GateStatus::Superseded.id());
    rules::require_if_equals(schema, "status", GateStatus::Done.id(), &["approved_by"]);
}

/// The rules every plan item has: those of an artifact, and its tracker number, the member
/// `number`, once its status is one of [`PlanItemStatus::STARTED`].
fn plan_item_rules(schema: &mut Schema, contract: &Contract, number: &str) {
    rules::artifact(
        schema,
        contract.name,
        contract.version,
        PlanItemStatus::Superseded.id(),
    );
    let started = PlanItemStatus::STARTED.map(PlanItemStatus::id);
    rules::require_if_in(schema, "status", &started, &[number]);
}

fn milestone_rules(schema: &mut Schema) {
    plan_item_rules(schema, &MILESTONE, "tracker_number");
}

fn epic_rules(schema: &mut Schema) {
    plan_item_rules(schema, &EPIC, "issue");
}

/// The rules of a story: those of a plan item, and `specification` from `approved` on.
fn story_rules(schema: &mut Schema) {
    plan_item_rules(schema, &STORY, "issue");
    let specified = [
        PlanItemStatus::Approved,
        PlanItemStatus::Implementing,
        PlanItemStatus::Done,
    ]
    .map(PlanItemStatus::id);
    rules::require_if_in(schema, "status", &specified, &["specification"]);
}

/// The rules of a task: those of a plan item, and `rubric` when `verification` contains
/// `judge` (SPEC-0009 R8).
fn task_rules(schema: &mut Schema) {
    plan_item_rules(schema, &TASK, "issue");
    rules::require_if_contains(
        schema,
        "verification",
        Verification::Judge.id(),
        &["rubric"],
    );
}

#[cfg(test)]
mod tests {
    use super::{Epic, Gate, Milestone, PlanItemStatus, Story, Task, Verification};
    use crate::contract::{Direction, schema_for};
    use serde::Serialize;
    use serde::de::DeserializeOwned;
    use serde_json::{Value, json};

    /// `document` reads into `T` and writes itself again.
    fn round_trip<T: Serialize + DeserializeOwned>(document: &Value) -> T {
        let read: T = serde_json::from_value(document.clone()).unwrap();
        assert_eq!(&serde_json::to_value(&read).unwrap(), document);
        read
    }

    #[test]
    fn every_plan_item_reads_and_writes_itself() {
        let gate: Gate = round_trip(&json!({
            "kind": "gate", "schema_version": 1, "id": "GATE-0001", "title": "Approve the brief",
            "status": "done", "subject": "BRIEF-0001", "issue": 2, "approved_by": "@owner"
        }));
        assert_eq!(gate.subject.as_str(), "BRIEF-0001");
        let milestone: Milestone = round_trip(&json!({
            "kind": "milestone", "schema_version": 1, "id": "M1", "title": "Time entries",
            "status": "implementing", "gate": "Entries are recorded.", "due": "2027-03-31",
            "tracker_number": 1
        }));
        assert_eq!(milestone.common.status, PlanItemStatus::Implementing);
        let _: Epic = round_trip(&json!({
            "kind": "epic", "schema_version": 1, "id": "EPIC-0001", "title": "Entries",
            "status": "draft", "milestone": "M1", "outcome": "Entries exist.", "area": "core"
        }));
        let _: Story = round_trip(&json!({
            "kind": "story", "schema_version": 1, "id": "STORY-0001", "title": "Record",
            "status": "approved", "epic": "EPIC-0001", "milestone": "M1", "as_a": "user",
            "i_want": "to record", "so_that": "I keep time",
            "acceptance_criteria": [{"id": "AC1", "given": "a", "when": "b", "then": "c"}],
            "specification": ["SPEC-0001"]
        }));
        let task: Task = round_trip(&json!({
            "kind": "task", "schema_version": 1, "id": "TASK-0001", "title": "Record",
            "status": "approved", "epic": "EPIC-0001", "milestone": "M1",
            "specification": [{"spec": "SPEC-0001", "requirements": ["R1"]}],
            "verification": ["end_to_end", "judge"], "rubric": "Score 1 to 5.", "area": "core",
            "depends_on": [], "complexity": 10, "parallel": false
        }));
        assert_eq!(
            task.verification,
            [Verification::EndToEnd, Verification::Judge]
        );
        assert!(serde_json::from_value::<super::Complexity>(json!(11)).is_err());
    }

    #[test]
    fn the_schema_of_a_task_holds_its_constants_and_its_conditional_requirements() {
        let schema = schema_for::<Task>(Direction::Input).unwrap();
        let properties = &schema["properties"];
        assert_eq!(properties["kind"]["const"], "task");
        assert_eq!(properties["schema_version"]["const"], 1);
        assert_eq!(properties["id"]["format"], "task-id");
        assert_eq!(properties["complexity"]["maximum"], 10);
        assert_eq!(properties["complexity"]["minimum"], 1);
        assert_eq!(schema["additionalProperties"], false);
        // The rules come from the helpers of `contract::rules`, tested there: here, which
        // member each requires and on which condition.
        let rules: Vec<(Value, Value)> = schema["allOf"]
            .as_array()
            .unwrap()
            .iter()
            .map(|rule| {
                (
                    rule["if"]["properties"].clone(),
                    rule["then"]["required"].clone(),
                )
            })
            .collect();
        assert_eq!(
            rules,
            [
                (
                    json!({"status": {"const": "superseded"}}),
                    json!(["superseded_by"])
                ),
                (
                    json!({"status": {"enum": ["implementing", "done"]}}),
                    json!(["issue"])
                ),
                (
                    json!({"verification": {"contains": {"const": "judge"}}}),
                    json!(["rubric"])
                ),
            ]
        );
    }

    #[test]
    fn the_gate_and_the_story_require_their_members_by_status() {
        let gate = schema_for::<Gate>(Direction::Input).unwrap();
        assert_eq!(
            gate["properties"]["subject"]["format"],
            "brief-or-council-id"
        );
        assert_eq!(
            gate["allOf"][1],
            json!({
                "if": {"properties": {"status": {"const": "done"}}, "required": ["status"]},
                "then": {"required": ["approved_by"]}
            })
        );
        let story = schema_for::<Story>(Direction::Input).unwrap();
        assert_eq!(
            story["allOf"][2],
            json!({
                "if": {
                    "properties": {"status": {"enum": ["approved", "implementing", "done"]}},
                    "required": ["status"]
                },
                "then": {"required": ["specification"]}
            })
        );
        let milestone = schema_for::<Milestone>(Direction::Input).unwrap();
        assert_eq!(
            milestone["allOf"][1]["then"],
            json!({"required": ["tracker_number"]})
        );
        assert_eq!(milestone["properties"]["id"]["format"], "milestone-id");
    }
}
