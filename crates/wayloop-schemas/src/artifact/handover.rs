//! The hand-over of SPEC-0001 (Hand-over record): the input an agent submits at the end of a
//! session, and the record the engine writes from it.
//!
//! The members the agent writes are one type, [`HandoverReport`], flattened into both: the
//! input holds them with its `schema_version` and nothing else, so an agent that sends a member
//! of the engine, for example `tokens`, is refused with `unknown_field` (SPEC-0001 R22). The
//! record adds the members the engine sets. A hand-over has no status and no title: it is
//! immutable, and a later one of the same issue takes its place in the next brief. Its links
//! (`issue`, `task`) and its branch are checked across the set.

use schemars::{JsonSchema, Schema};
use serde::{Deserialize, Serialize};

use super::ArtifactKind;
use crate::IssueStep;
use crate::contract::{HANDOVER, rules};
use crate::value::{HandoverId, IssueNumber, SchemaVersion, TaskId, Text, UtcDateTime};

crate::positive_integer! {
    /// Size of a hand-over record in tokens, from 1: the length in bytes of the canonical JSON
    /// of the record (RFC 8785) divided by four, rounded up; counted by the engine, within the
    /// budget of SPEC-0003 R12.
    pub struct Tokens;
}

/// A ruling of a session (SPEC-0003 R5): a decision the agent took, why, and what it costs if
/// it is wrong.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Ruling {
    /// The decision taken.
    pub decision: Text,
    /// Why it was taken.
    pub reason: Text,
    /// What it costs to undo if it proves wrong.
    pub cost_if_wrong: Text,
}

/// What the agent reports at the end of a session: the members of a hand-over that the agent
/// writes, the same in the input and in the record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct HandoverReport {
    /// Step reached in the issue cycle.
    pub step: IssueStep,
    /// Paths changed by the session; `[]` means none.
    pub files_touched: Vec<Text>,
    /// Work left; `[]` means none.
    pub not_done: Vec<Text>,
    /// Questions raised; `[]` means none.
    pub open_questions: Vec<Text>,
    /// Rulings of the session; `[]` means none.
    pub rulings: Vec<Ruling>,
    /// First action of the next session.
    pub next_step: Text,
}

/// A hand-over input (SPEC-0001 R22): what an agent submits at the end of a session, before the
/// engine adds the members it owns. It has no `kind`: the entry that carries it names it with
/// `input_kind`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(transform = handover_input_rules)]
pub struct HandoverInput {
    /// Version of the input schema the agent wrote against (SPEC-0001 R16).
    pub schema_version: SchemaVersion,
    /// The members the agent writes.
    #[serde(flatten)]
    pub report: HandoverReport,
}

/// A hand-over record (SPEC-0001, Hand-over record): the input of the agent with the members
/// the engine sets, as the engine records it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(transform = handover_rules)]
pub struct Handover {
    /// Artifact type; selects the schema.
    pub kind: ArtifactKind,
    /// Version of the record schema, set by the engine (SPEC-0001 R16).
    pub schema_version: SchemaVersion,
    /// Identifier, assigned by the engine when it records the hand-over.
    pub id: HandoverId,
    /// Tracker number of the issue worked, from the checkout.
    pub issue: IssueNumber,
    /// Task of the issue, when it has one; it carries the same `issue`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<TaskId>,
    /// Session identifier in the ledger.
    pub session: Text,
    /// Holder of the checkout (SPEC-0003 R10).
    pub holder: Text,
    /// Executor that ran the session, for example `claude_code`.
    pub executor: Text,
    /// Branch of the checkout, valid under the naming grammar of SPEC-0001 R11.
    pub branch: Text,
    /// The members the agent wrote.
    #[serde(flatten)]
    pub report: HandoverReport,
    /// When the engine recorded the hand-over.
    pub recorded_at: UtcDateTime,
    /// Size of the record.
    pub tokens: Tokens,
}

/// The rules of a hand-over input the derive does not write: its `schema_version`.
fn handover_input_rules(schema: &mut Schema) {
    rules::version(schema, &HANDOVER);
}

/// The rules of a hand-over record the derive does not write: its `kind` and its
/// `schema_version`.
fn handover_rules(schema: &mut Schema) {
    rules::constants(schema, &HANDOVER);
}

#[cfg(test)]
mod tests {
    use super::{Handover, HandoverInput};
    use crate::IssueStep;
    use crate::contract::{Direction, schema_for};
    use serde_json::{Value, json};

    /// The engine members of a record (SPEC-0001, Hand-over record, `Set by` engine).
    const ENGINE: [&str; 10] = [
        "kind",
        "id",
        "issue",
        "task",
        "session",
        "holder",
        "executor",
        "branch",
        "recorded_at",
        "tokens",
    ];

    fn members(schema: &Value) -> Vec<String> {
        let mut names: Vec<String> = schema["properties"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        names.sort_unstable();
        names
    }

    #[test]
    fn the_input_holds_the_members_of_the_agent_and_the_record_adds_those_of_the_engine() {
        let input = schema_for::<HandoverInput>(Direction::Input).unwrap();
        let record = schema_for::<Handover>(Direction::Input).unwrap();
        let mut expected = members(&input);
        expected.extend(ENGINE.map(str::to_owned));
        expected.sort_unstable();
        assert_eq!(members(&record), expected);
        for member in ENGINE {
            assert!(input["properties"].get(member).is_none(), "{member}");
        }
        assert_eq!(input["additionalProperties"], false);
        assert_eq!(record["additionalProperties"], false);
        assert_eq!(input["properties"]["schema_version"]["const"], 1);
        assert_eq!(record["properties"]["schema_version"]["const"], 1);
        assert_eq!(record["properties"]["kind"]["const"], "handover");
        assert_eq!(record["properties"]["id"]["format"], "handover-id");
        assert_eq!(
            record["properties"]["recorded_at"]["format"],
            "utc-date-time"
        );
        assert_eq!(record["properties"]["tokens"]["minimum"], 1);
        // Every member is required, except `task`; no status, so no rule on it.
        let required = |schema: &Value| -> Vec<String> {
            let mut names: Vec<String> = schema["required"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_owned())
                .collect();
            names.sort_unstable();
            names
        };
        assert_eq!(required(&input), members(&input));
        let mut optional = members(&record);
        optional.retain(|m| !required(&record).contains(m));
        assert_eq!(optional, ["task"]);
        assert!(record.get("allOf").is_none());
    }

    #[test]
    fn a_hand_over_reads_and_writes_itself_with_its_step_in_snake_case() {
        let input = json!({
            "schema_version": 1, "step": "pull_request", "files_touched": [],
            "not_done": [], "open_questions": [], "rulings": [], "next_step": "Merge."
        });
        let read: HandoverInput = serde_json::from_value(input.clone()).unwrap();
        assert_eq!(read.report.step, IssueStep::PullRequest);
        assert_eq!(serde_json::to_value(&read).unwrap(), input);
        let mut kebab = input;
        kebab["step"] = json!("pull-request");
        assert!(serde_json::from_value::<HandoverInput>(kebab).is_err());
    }
}
