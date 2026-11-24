//! The inputs of SPEC-0001 (Inputs) in the area `inputs` of the contracts: the pull request and
//! the commit message. They are not artifacts: they have no `kind`, no identifier and no status,
//! and the `input_kind` of their entry selects their schema (plan of SPEC-0001, Canonical input
//! of `wayloop validate`). Like every input, they carry the `schema_version` of their schema
//! (decided by the maintainer on 2026-11-24), checked and migrated as an artifact is (R16).
//! They carry no `language`: adapters assemble them from the tracker or from git, which give
//! none, and their prose is in the language of the project (R6). So they have no shared
//! `Header`, and `schema_version` is a member of each type, on purpose (decided by the
//! maintainer on 2026-11-25). The engine reads them, so their schemas are closed.
//!
//! The schema states the shape only. The grammar of a branch name and of a commit header (R11)
//! and the issue links of a pull request (R10) are checked across the set, with the policy of
//! the project.

use schemars::{JsonSchema, Schema};
use serde::{Deserialize, Serialize};

use crate::contract::{COMMIT, PULL_REQUEST, rules};
use crate::value::{SchemaVersion, Text};

/// A pull request (SPEC-0001 R10), as the adapter of the tracker reads it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(transform = pull_request_rules)]
pub struct PullRequest {
    /// Version of the schema of the input the document was written against (SPEC-0001 R16).
    pub schema_version: SchemaVersion,
    /// Pull request title, a commit header under SPEC-0001 R11.
    pub title: Text,
    /// Pull request body, where the issue is linked; may be empty.
    pub body: String,
    /// Head branch.
    pub branch: Text,
    /// Labels, in the spelling of the tracker, each a non-empty text (decided by the maintainer
    /// on 2026-11-25); `[]` means none.
    pub labels: Vec<Text>,
}

/// A commit message (SPEC-0001 R11), as the adapter of git reads it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(transform = commit_rules)]
pub struct Commit {
    /// Version of the schema of the input the document was written against (SPEC-0001 R16).
    pub schema_version: SchemaVersion,
    /// First line of the message.
    pub header: Text,
    /// Lines after the blank line, trailers included; absent when the message has one line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Text>,
}

/// The rules of a pull request the derive does not write: its `schema_version`.
fn pull_request_rules(schema: &mut Schema) {
    rules::version(schema, &PULL_REQUEST);
}

/// The rules of a commit the derive does not write: its `schema_version`.
fn commit_rules(schema: &mut Schema) {
    rules::version(schema, &COMMIT);
}

#[cfg(test)]
mod tests {
    use super::{Commit, PullRequest};
    use crate::contract::{Direction, schema_for};
    use crate::examples;
    use serde_json::Value;

    fn required(schema: &Value) -> Vec<&str> {
        schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect()
    }

    #[test]
    fn a_pull_request_reads_and_writes_itself() {
        let given = examples::pull_request();
        let read: PullRequest = serde_json::from_value(given.clone()).unwrap();
        assert_eq!(read.branch.as_str(), "feat/12-record-time-entry");
        assert_eq!(serde_json::to_value(&read).unwrap(), given);
    }

    #[test]
    fn a_commit_reads_and_writes_itself_with_and_without_a_body() {
        let given = examples::commit();
        let mut one_line = given.clone();
        one_line.as_object_mut().unwrap().remove("body");
        for given in [given, one_line] {
            let read: Commit = serde_json::from_value(given.clone()).unwrap();
            assert_eq!(serde_json::to_value(&read).unwrap(), given);
        }
    }

    #[test]
    fn the_schema_of_a_pull_request_requires_its_members_and_is_closed() {
        let schema = schema_for::<PullRequest>(Direction::Input).unwrap();
        assert_eq!(
            required(&schema),
            ["schema_version", "title", "body", "branch", "labels"]
        );
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(schema["properties"]["schema_version"]["const"], 1);
        assert_eq!(schema["properties"]["schema_version"]["type"], "integer");
        assert_eq!(schema["properties"]["labels"]["type"], "array");
        assert_eq!(schema["properties"]["labels"]["items"]["type"], "string");
        // An input has no kind, identifier or status.
        for member in ["kind", "id", "status"] {
            assert_eq!(schema["properties"].get(member), None, "{member}");
        }
    }

    #[test]
    fn the_title_the_branch_and_each_label_are_not_empty_and_the_body_may_be() {
        // Decided by the maintainer on 2026-11-24, for labels on 2026-11-25 (SPEC-0001,
        // Inputs). The list of labels may still be empty.
        let schema = schema_for::<PullRequest>(Direction::Input).unwrap();
        let properties = &schema["properties"];
        assert_eq!(properties["title"]["minLength"], 1);
        assert_eq!(properties["branch"]["minLength"], 1);
        assert_eq!(properties["body"].get("minLength"), None);
        assert_eq!(properties["labels"]["items"]["minLength"], 1);
        assert_eq!(properties["labels"].get("minItems"), None);
    }

    #[test]
    fn the_schema_of_a_commit_requires_its_version_and_header_and_is_closed() {
        let schema = schema_for::<Commit>(Direction::Input).unwrap();
        assert_eq!(required(&schema), ["schema_version", "header"]);
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(schema["properties"]["schema_version"]["const"], 1);
        assert_eq!(schema["properties"]["header"]["minLength"], 1);
        assert_eq!(schema["properties"]["body"]["type"], "string");
        assert_eq!(schema["properties"]["body"]["minLength"], 1);
    }
}
