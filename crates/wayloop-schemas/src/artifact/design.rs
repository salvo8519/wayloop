//! The artifacts of SPEC-0001 that design the work before it is planned: the council report on
//! a brief, the living specification of a capability and the change delta of a living
//! specification, each built on [`Common`](super::Common).
//!
//! They live in the repository: a council report is the file `plan/council/COUNCIL-<seq>.yaml`,
//! a living specification the file `specs/<NNNN>-<capability>/spec.md`, whose Markdown is
//! normalised into the document described here, and a change delta a folder of its change
//! (SPEC-0001, Where artifacts live). The schema states what one document can state: the
//! members, their forms and the members required by a status, a verification or an operation.
//! Links to other artifacts, unique parts, a dissent that names a decision of the report, a
//! recommendation or an answer among the options, requirements that are active and one
//! operation per requirement are checked across the set. A member that the `op` of an
//! operation refuses cannot be stated by the schema: it is refused by the check of parts.

use schemars::{JsonSchema, Schema};
use serde::{Deserialize, Serialize};

use super::plan::rubric_when_judged;
use super::{Common, Verification};
use crate::contract::rules;
use crate::contract::{CHANGE, COUNCIL_REPORT, SPECIFICATION};
use crate::value::{
    BriefId, Capability, ChangeId, CouncilId, CriterionId, DecisionId, IssueNumber, MilestoneId,
    PolicyName, QuestionId, RequirementId, RoleName, SpecId, StoryId, TaskId, Text, TrackerHandle,
};

crate::contract_enum! {
    /// Status of a council report (SPEC-0001 R5); `draft` is its initial status.
    pub enum CouncilStatus {
        /// Written by the moderator, not approved.
        Draft = "draft",
        /// Approved by a person on its gate issue, every question answered; the roadmap may be
        /// generated from it.
        Approved = "approved",
        /// Replaced by the council reports in `superseded_by`; terminal.
        Superseded = "superseded",
    }
}

crate::contract_enum! {
    /// How the council ran (SPEC-0012 R6).
    pub enum CouncilMode {
        /// Every role in a single context.
        OneMind = "one_mind",
        /// One executor per role, each reading only the brief in the first round.
        Independent = "independent",
    }
}

crate::contract_enum! {
    /// Status of a living specification (SPEC-0001 R5); `draft` and `surveyed` are its initial
    /// statuses.
    pub enum SpecificationStatus {
        /// Written, not approved.
        Draft = "draft",
        /// Approved by a person; tasks may cite it.
        Approved = "approved",
        /// Written by the survey of an existing project (SPEC-0011 R4), not approved.
        Surveyed = "surveyed",
        /// A task that cites it is claimed.
        Implementing = "implementing",
        /// Delivered, as a person declares; terminal.
        Done = "done",
        /// Replaced by the specifications in `superseded_by`; terminal.
        Superseded = "superseded",
    }
}

crate::contract_enum! {
    /// Status of a requirement within its living specification.
    pub enum RequirementStatus {
        /// In force.
        Active = "active",
        /// Removed by a change delta; kept so that its identifier is never used again.
        Removed = "removed",
    }
}

crate::contract_enum! {
    /// Status of a change delta (SPEC-0001 R5); `draft` is its initial status.
    pub enum ChangeStatus {
        /// Proposed, not approved.
        Draft = "draft",
        /// Approved by a person; its tasks may start.
        Approved = "approved",
        /// Its tasks are under way.
        Implementing = "implementing",
        /// Merged into its living specification and archived; terminal.
        Done = "done",
        /// Replaced by the change deltas in `superseded_by`; terminal.
        Superseded = "superseded",
    }
}

crate::contract_enum! {
    /// What an operation of a change delta does to a requirement (ADR 0014, SPEC-0009 R2).
    pub enum OperationKind {
        /// Adds a requirement, which gets its identifier when the delta is merged.
        Added = "added",
        /// Changes the text of a requirement, and its name when one is given.
        Modified = "modified",
        /// Removes a requirement, which stays recorded with the status `removed`.
        Removed = "removed",
        /// Changes the name of a requirement only.
        Renamed = "renamed",
    }
}

impl OperationKind {
    /// The operations on an existing requirement, which name it in `requirement`.
    pub const ON_A_REQUIREMENT: [OperationKind; 3] = [
        OperationKind::Modified,
        OperationKind::Removed,
        OperationKind::Renamed,
    ];

    /// The operations that give the text of the requirement.
    pub const WITH_TEXT: [OperationKind; 2] = [OperationKind::Added, OperationKind::Modified];
}

/// A decision of a council report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Decision {
    /// Identifier within the report, `D<k>`, cited as `COUNCIL-<seq> D<k>`.
    pub id: DecisionId,
    /// What is decided.
    pub statement: Text,
    /// Why.
    pub reason: Text,
}

/// An option the council considered and rejected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RejectedAlternative {
    /// The option.
    pub option: Text,
    /// Why it was rejected.
    pub reason: Text,
}

/// The position of a role that disagrees with a decision of the report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Dissent {
    /// The role that disagrees.
    pub role: RoleName,
    /// Its position.
    pub position: Text,
    /// The decision of this report it disagrees with, `D<k>`.
    pub decision: DecisionId,
}

/// A risk and its mitigation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Risk {
    /// The risk.
    pub description: Text,
    /// How it is mitigated.
    pub mitigation: Text,
}

/// An unresolved conflict of the council, for the maintainer to decide.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CouncilQuestion {
    /// Identifier within the report, `Q<k>`, cited as `COUNCIL-<seq> Q<k>`.
    pub id: QuestionId,
    /// The question.
    pub question: Text,
    /// The possible answers; at least two.
    #[schemars(length(min = 2))]
    pub options: Vec<Text>,
    /// The answer the council recommends, one of `options`.
    pub recommendation: Text,
    /// The answer of the maintainer, one of `options`; absent until decided.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answer: Option<Text>,
}

/// A council report (SPEC-0001, Council report): the moderator report on a brief (ADR 0016,
/// SPEC-0012 R4). Stored as `plan/council/COUNCIL-<seq>.yaml` in this normalised form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(transform = council_report_rules)]
pub struct CouncilReport {
    /// The members of every artifact.
    #[serde(flatten)]
    pub common: Common<CouncilId, CouncilStatus>,
    /// The brief the council answered.
    pub brief: BriefId,
    /// How the council ran.
    pub mode: CouncilMode,
    /// The roles that gave an opinion; at least one.
    #[schemars(length(min = 1))]
    pub roles: Vec<RoleName>,
    /// The decisions; at least one.
    #[schemars(length(min = 1))]
    pub decisions: Vec<Decision>,
    /// Options considered and rejected; `[]` means none.
    pub rejected_alternatives: Vec<RejectedAlternative>,
    /// Positions that disagree with a decision; `[]` means none.
    pub dissent: Vec<Dissent>,
    /// Risks and their mitigation; `[]` means none.
    pub risks: Vec<Risk>,
    /// Unresolved conflicts for the maintainer; `[]` means none.
    pub questions: Vec<CouncilQuestion>,
}

/// An EARS requirement of a living specification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Requirement {
    /// Identifier within the specification, `R<k>`, never used again.
    pub id: RequirementId,
    /// Short name, changed by a rename.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<Text>,
    /// The requirement, in EARS form.
    pub text: Text,
    /// Whether the requirement is in force; absent means `active`, which is not a default
    /// applied by the engine.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<RequirementStatus>,
}

/// An acceptance criterion of a living specification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(transform = rubric_when_judged)]
pub struct SpecCriterion {
    /// Identifier within the specification, `AC<k>`, cited as `SPEC-<seq> AC<k>`.
    pub id: CriterionId,
    /// The active requirements of this specification it verifies; at least one.
    #[schemars(length(min = 1))]
    pub requirements: Vec<RequirementId>,
    /// The criterion.
    pub text: Text,
    /// Verification techniques; at least one. The order of the values and a value repeated
    /// carry no meaning.
    #[schemars(length(min = 1))]
    pub verification: Vec<Verification>,
    /// Rubric of the judge; required when `verification` contains `judge`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rubric: Option<Text>,
}

/// An open question of a living specification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct OpenQuestion {
    /// Identifier within the specification, `Q<k>`, when the question has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<QuestionId>,
    /// The question, and its answer once resolved.
    pub text: Text,
    /// Whether the question is resolved.
    pub resolved: bool,
}

/// A section of a living specification outside its template, kept as prose and not validated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ExtraSection {
    /// The heading of the section, as written; not empty.
    pub heading: Text,
    /// The content of the section, as written; absent when the section has no content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<Text>,
}

/// A living specification (SPEC-0001, Living specification): the current behaviour of one
/// capability (ADR 0014), in the normalised form of its file `specs/<NNNN>-<capability>/spec.md`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(transform = specification_rules)]
pub struct Specification {
    /// The members of every artifact.
    #[serde(flatten)]
    pub common: Common<SpecId, SpecificationStatus>,
    /// The capability described, the slug of the folder.
    pub capability: Capability,
    /// Person who approves changes.
    pub owner: TrackerHandle,
    /// Milestone of the first delivery.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub milestone: Option<MilestoneId>,
    /// Scope of the project policy used by the commits that implement it.
    pub commit_scope: PolicyName,
    /// Identifiers of ADRs, issues and specifications, as written, for example `ADR 0014`;
    /// `[]` means none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related: Option<Vec<Text>>,
    /// Who needs this and why.
    pub problem: Text,
    /// What is specified; at least one entry.
    #[schemars(length(min = 1))]
    pub scope_in: Vec<Text>,
    /// What is left to other specifications; `[]` means nothing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope_out: Option<Vec<Text>>,
    /// Stories covered, by identifier; `[]` means none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stories: Option<Vec<StoryId>>,
    /// Stories written as prose, before or without a story artifact; `[]` means none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_stories: Option<Vec<Text>>,
    /// The requirements, removed ones included; at least one.
    #[schemars(length(min = 1))]
    pub requirements: Vec<Requirement>,
    /// The acceptance criteria; at least one.
    #[schemars(length(min = 1))]
    pub acceptance_criteria: Vec<SpecCriterion>,
    /// Commands, schemas and events, with their versions; `[]` means none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interfaces: Option<Vec<Text>>,
    /// Open and resolved questions; `[]` means none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_questions: Option<Vec<OpenQuestion>>,
    /// Sections outside the template; `[]` means none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_sections: Option<Vec<ExtraSection>>,
}

/// An operation of a change delta on one requirement of its living specification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(transform = operation_rules)]
pub struct Operation {
    /// What the operation does.
    pub op: OperationKind,
    /// The existing requirement, `R<k>`; required for `modified`, `removed` and `renamed`,
    /// refused for `added`: an added requirement gets its identifier when the delta is merged.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requirement: Option<RequirementId>,
    /// The name of the requirement; required for `renamed`, optional for `added` and
    /// `modified`, refused for `removed`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<Text>,
    /// The text of the requirement; required for `added` and `modified`, refused for `removed`
    /// and `renamed`: a rename with a new text is one `modified` operation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<Text>,
}

/// A change delta (SPEC-0001, Change delta): a reviewable change of the requirements of a
/// living specification (ADR 0014, SPEC-0009 R2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(transform = change_rules)]
pub struct Change {
    /// The members of every artifact.
    #[serde(flatten)]
    pub common: Common<ChangeId, ChangeStatus>,
    /// The living specification the delta changes.
    pub specification: SpecId,
    /// Why the change is made.
    pub proposal: Text,
    /// How the change is made, when the proposal does not say it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub design: Option<Text>,
    /// The operations, at most one per requirement; at least one.
    #[schemars(length(min = 1))]
    pub operations: Vec<Operation>,
    /// Tasks that implement the change; `[]` means none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tasks: Option<Vec<TaskId>>,
    /// Tracker issue that requested the change.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue: Option<IssueNumber>,
}

fn council_report_rules(schema: &mut Schema) {
    rules::artifact(schema, &COUNCIL_REPORT, CouncilStatus::Superseded.id());
}

fn specification_rules(schema: &mut Schema) {
    rules::artifact(schema, &SPECIFICATION, SpecificationStatus::Superseded.id());
}

fn change_rules(schema: &mut Schema) {
    rules::artifact(schema, &CHANGE, ChangeStatus::Superseded.id());
}

/// The rules of an operation: the members its `op` requires.
fn operation_rules(schema: &mut Schema) {
    let on_a_requirement = OperationKind::ON_A_REQUIREMENT.map(OperationKind::id);
    rules::require_if_in(schema, "op", &on_a_requirement, &["requirement"]);
    let with_text = OperationKind::WITH_TEXT.map(OperationKind::id);
    rules::require_if_in(schema, "op", &with_text, &["text"]);
    rules::require_if_equals(schema, "op", OperationKind::Renamed.id(), &["name"]);
}

#[cfg(test)]
mod tests {
    use super::{Change, CouncilReport, Specification};
    use crate::contract::{Direction, schema_for};
    use serde_json::{Value, json};

    /// The `if` and the `then` of each rule of `schema`.
    fn rules(schema: &Value) -> Vec<(Value, Value)> {
        schema["allOf"]
            .as_array()
            .unwrap()
            .iter()
            .map(|rule| {
                (
                    rule["if"]["properties"].clone(),
                    rule["then"]["required"].clone(),
                )
            })
            .collect()
    }

    #[test]
    fn the_schemas_hold_their_constants_and_the_superseded_rule() {
        let superseded = (
            json!({"status": {"const": "superseded"}}),
            json!(["superseded_by"]),
        );
        for (schema, kind, id) in [
            (
                schema_for::<CouncilReport>(Direction::Input).unwrap(),
                "council_report",
                "council-id",
            ),
            (
                schema_for::<Specification>(Direction::Input).unwrap(),
                "specification",
                "spec-id",
            ),
            (
                schema_for::<Change>(Direction::Input).unwrap(),
                "change",
                "change-id",
            ),
        ] {
            assert_eq!(schema["properties"]["kind"]["const"], kind);
            assert_eq!(schema["properties"]["schema_version"]["const"], 1);
            assert_eq!(schema["properties"]["id"]["format"], id);
            assert_eq!(schema["additionalProperties"], false);
            assert_eq!(rules(&schema), core::slice::from_ref(&superseded), "{kind}");
        }
    }

    #[test]
    fn a_criterion_and_an_operation_require_their_members_by_condition() {
        // The rules come from the helpers of `contract::rules`, tested there: here, which
        // member each requires and on which condition.
        let specification = schema_for::<Specification>(Direction::Input).unwrap();
        assert_eq!(
            specification["properties"]["capability"]["format"],
            "capability"
        );
        assert_eq!(
            rules(&specification["$defs"]["SpecCriterion"]),
            [(
                json!({"verification": {"type": "array", "contains": {"const": "judge"}}}),
                json!(["rubric"])
            )]
        );
        let change = schema_for::<Change>(Direction::Input).unwrap();
        assert_eq!(
            rules(&change["$defs"]["Operation"]),
            [
                (
                    json!({"op": {"enum": ["modified", "removed", "renamed"]}}),
                    json!(["requirement"])
                ),
                (
                    json!({"op": {"enum": ["added", "modified"]}}),
                    json!(["text"])
                ),
                (json!({"op": {"const": "renamed"}}), json!(["name"])),
            ]
        );
        let council = schema_for::<CouncilReport>(Direction::Input).unwrap();
        assert_eq!(
            council["$defs"]["CouncilQuestion"]["properties"]["options"]["minItems"],
            2
        );
    }
}
