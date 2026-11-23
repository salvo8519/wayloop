//! The artifacts of SPEC-0001: the members every artifact has, written once in [`Common`],
//! and each artifact type built on it (ADR 0030 G1): the brief here, the gate issue and the
//! plan items (milestone, epic, user story, task) in `plan`.
//!
//! An artifact is read by the engine, so its schema is an input: closed, an unknown member is
//! refused with `unknown_field` (SPEC-0001 R6). Keys, enum values and status words are
//! English; prose is in the language of the document. A member set to null is not given
//! (ADR 0009 C2). The constants `kind` and `schema_version` of a type come from its
//! registration in [`CONTRACTS`](crate::contract::CONTRACTS), written into its schema by
//! [`rules::artifact`].

use schemars::{JsonSchema, Schema};
use serde::{Deserialize, Serialize};

use crate::contract::{BRIEF, rules};
use crate::value::{BriefId, LanguageTag, SchemaVersion, Text, TrackerHandle};

mod plan;

pub use plan::{
    Complexity, Epic, Gate, GateStatus, Milestone, PlanItemStatus, SpecReference, Story,
    StoryCriterion, Task, Verification,
};

crate::contract_enum! {
    /// Type of an artifact (SPEC-0001, Common rules): the value of `kind`, which selects the
    /// schema of the document.
    pub enum ArtifactKind {
        /// A short statement of an idea or a scope of work, input of the council.
        Brief = "brief",
        /// The report of the council on a brief.
        CouncilReport = "council_report",
        /// The tracker issue on which a person approves a brief or a council report.
        Gate = "gate",
        /// A milestone of the roadmap.
        Milestone = "milestone",
        /// An epic of a milestone.
        Epic = "epic",
        /// A user story of an epic.
        Story = "story",
        /// A living specification of one capability.
        Specification = "specification",
        /// A change delta of a living specification.
        Change = "change",
        /// A task of an epic, closed by one pull request.
        Task = "task",
        /// The hand-over record of a session.
        Handover = "handover",
    }
}

crate::contract_enum! {
    /// Status of a brief (SPEC-0001 R5); `draft` is its initial status.
    pub enum BriefStatus {
        /// Written, not approved.
        Draft = "draft",
        /// Approved by a person on its gate issue; it goes to the council.
        Approved = "approved",
        /// Replaced by the briefs in `superseded_by`; terminal.
        Superseded = "superseded",
    }
}

crate::contract_enum! {
    /// What a brief was written from.
    pub enum BriefOrigin {
        /// A short list of needs.
        LightRequirements = "light_requirements",
        /// A scope of work agreed with a client.
        ScopeOfWork = "scope_of_work",
    }
}

/// The members every artifact has (SPEC-0001, Common rules), with the identifier type `Id`
/// and the status enum `S` of its kind. It is flattened into each artifact type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Common<Id, S> {
    /// Artifact type; selects the schema.
    pub kind: ArtifactKind,
    /// Version of the schema of the type the document was written against (SPEC-0001 R16).
    pub schema_version: SchemaVersion,
    /// Stable identifier, in the canonical form of the type; assigned by the engine and never
    /// reused (SPEC-0001 R2, R14).
    pub id: Id,
    /// Outcome or name, in sentence case without a final full stop; 1 to 120 characters
    /// (Unicode code points).
    #[schemars(length(min = 1, max = 120))]
    pub title: String,
    /// Current status (SPEC-0001 R5), not the issue-cycle state of SPEC-0002.
    pub status: S,
    /// Language of the prose, a BCP 47 tag; when absent, the language of the project.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<LanguageTag>,
    /// Identifiers of the artifacts of the same type that replace this one, at least one;
    /// required when `status` is `superseded`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1))]
    pub superseded_by: Option<Vec<Id>>,
}

/// A brief (SPEC-0001, Brief): a short statement of an idea or a scope of work, input of the
/// council. Stored as `plan/briefs/BRIEF-<seq>.yaml` in this normalised form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(transform = brief_rules)]
pub struct Brief {
    /// The members of every artifact.
    #[serde(flatten)]
    pub common: Common<BriefId, BriefStatus>,
    /// What the brief was written from.
    pub origin: BriefOrigin,
    /// Person accountable for the brief.
    pub owner: TrackerHandle,
    /// Who needs this and why.
    pub problem: Text,
    /// Outcomes the project must reach; at least one.
    #[schemars(length(min = 1))]
    pub goals: Vec<Text>,
    /// Who uses the result; at least one.
    #[schemars(length(min = 1))]
    pub users: Vec<Text>,
    /// What is included; at least one entry.
    #[schemars(length(min = 1))]
    pub scope_in: Vec<Text>,
    /// What is excluded; `[]` means nothing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope_out: Option<Vec<Text>>,
    /// Budget, deadlines, technology, regulation; `[]` means none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constraints: Option<Vec<Text>>,
    /// Questions the council must answer; `[]` means none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_questions: Option<Vec<Text>>,
}

/// The rules of an artifact that the derive does not write, for a brief.
fn brief_rules(schema: &mut Schema) {
    rules::artifact(
        schema,
        BRIEF.name,
        BRIEF.version,
        BriefStatus::Superseded.id(),
    );
}

#[cfg(test)]
mod tests {
    use super::{ArtifactKind, Brief, BriefStatus};
    use crate::contract::{Direction, schema_for};
    use serde_json::json;

    fn brief() -> serde_json::Value {
        json!({
            "kind": "brief",
            "schema_version": 1,
            "id": "BRIEF-0001",
            "title": "Freelancers record their working time from the terminal",
            "status": "approved",
            "origin": "light_requirements",
            "owner": "@example-owner",
            "problem": "Freelancers lose billable hours.",
            "goals": ["Record a time entry with one command"],
            "users": ["Freelance developers"],
            "scope_in": ["Time entries"]
        })
    }

    #[test]
    fn a_brief_reads_and_writes_itself() {
        let read: Brief = serde_json::from_value(brief()).unwrap();
        assert_eq!(read.common.kind, ArtifactKind::Brief);
        assert_eq!(read.common.status, BriefStatus::Approved);
        assert_eq!(read.common.id.as_str(), "BRIEF-0001");
        assert_eq!(serde_json::to_value(&read).unwrap(), brief());
    }

    #[test]
    fn the_schema_of_a_brief_holds_its_constants_and_the_superseded_rule() {
        let schema = schema_for::<Brief>(Direction::Input).unwrap();
        let properties = &schema["properties"];
        assert_eq!(properties["kind"]["const"], "brief");
        assert_eq!(properties["schema_version"]["const"], 1);
        assert_eq!(properties["schema_version"]["type"], "integer");
        assert_eq!(properties["title"]["maxLength"], 120);
        assert_eq!(properties["id"]["format"], "brief-id");
        assert_eq!(properties["superseded_by"]["items"]["format"], "brief-id");
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(
            schema["allOf"],
            json!([{
                "if": {"properties": {"status": {"const": "superseded"}}, "required": ["status"]},
                "then": {"required": ["superseded_by"]}
            }])
        );
        let required: Vec<&str> = schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        for member in [
            "kind",
            "schema_version",
            "id",
            "title",
            "status",
            "owner",
            "scope_in",
        ] {
            assert!(required.contains(&member), "{member}");
        }
        assert!(!required.contains(&"language"));
    }
}
