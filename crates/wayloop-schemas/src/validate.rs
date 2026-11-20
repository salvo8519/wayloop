//! The command `validate`, version 1 (SPEC-0001; plan of SPEC-0001, Canonical input of
//! `wayloop validate`): its input, closed, and the data of its result, open (ADR 0009 C5).
//!
//! The input schema cannot state every rule of the input: the core also refuses, with the code
//! `command_input`, a member of an entry that belongs to another format, an `origin` with no
//! member or more than one, `lines` or the format `unreadable` without a file origin, a key of
//! `lines` that is not an RFC 6901 pointer and a line that is not an integer from 1. It checks
//! them on the JSON value before it reads the input into these types.

use std::collections::BTreeMap;

use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::artifact::ArtifactKind;
use crate::contract::rules;
use crate::value::{
    AdrNumber, IssueNumber, LanguageTag, Line, MilestoneNumber, ProjectPath, Text,
    free_object_schema,
};

crate::contract_enum! {
    /// What an entry of `documents` holds; it chooses which other members are given.
    pub enum EntryFormat {
        /// A JSON value in the normalised form of its type, in `document`, with `lines` for a
        /// file.
        Document = "document",
        /// The text of a living specification (`spec.md`), in `text`.
        Markdown = "markdown",
        /// A file that did not parse, with what the parser reported in `error`.
        Unreadable = "unreadable",
    }
}

crate::contract_enum! {
    /// Type of an input (SPEC-0001, Inputs), which has no `kind` member.
    pub enum InputKind {
        /// A pull request: title, body, branch and labels.
        PullRequest = "pull_request",
        /// A commit message: header and body.
        Commit = "commit",
        /// A hand-over submitted by an agent, before the engine fills its own fields.
        HandoverInput = "handover_input",
    }
}

crate::contract_enum! {
    /// Status of an artifact (SPEC-0001 R5), as any type allows it.
    pub enum Status {
        /// Written, not approved.
        Draft = "draft",
        /// Approved by a person.
        Approved = "approved",
        /// A specification written by the survey of an existing project, not approved.
        Surveyed = "surveyed",
        /// Work on it has started.
        Implementing = "implementing",
        /// Delivered; terminal.
        Done = "done",
        /// Replaced by other artifacts; terminal.
        Superseded = "superseded",
    }
}

/// Where a document was read: exactly one of `file`, `issue`, `milestone`, which the core
/// checks. The member present tells the form, with no tag, as for `source` of an error: a choice
/// without a kind, against the letter of ADR 0009 C4, so that the input and the error report
/// write a place the same way (plan of SPEC-0001).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub struct Origin {
    /// A repository file, by its path relative to the root of the project.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<ProjectPath>,
    /// A tracker issue, by its number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue: Option<IssueNumber>,
    /// A tracker milestone, by its number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub milestone: Option<MilestoneNumber>,
}

/// What the parser of a file reported when it could not read it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ParseError {
    /// Line of the fault.
    pub line: Line,
    /// Message of the parser, for a person.
    pub message: String,
}

/// A document in the normalised form of its type, a JSON object checked by the schema of its
/// `kind`, not by the schema of the command.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FreeDocument(pub Map<String, Value>);

impl JsonSchema for FreeDocument {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "FreeDocument".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        free_object_schema(
            "A document in the normalised form of its type, checked by the schema of its kind.",
        )
    }
}

/// The line, from 1, of each value of a document read from a file, keyed by its RFC 6901
/// pointer into the document; the core checks the keys and the lines.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Lines(pub BTreeMap<String, Line>);

impl JsonSchema for Lines {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Lines".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        free_object_schema(
            "The line, from 1, of each value of the document, keyed by its RFC 6901 pointer into \
             the document.",
        )
    }
}

/// One stored artifact or input. `format` chooses which other members are given: `document`
/// (with `lines` for a file), `markdown` (with `text`), `unreadable` (with `error`, and a
/// `file` origin: only a file can fail to parse).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(transform = entry_rules)]
pub struct DocumentEntry {
    /// Where the document was read; it becomes `source` in every error. Absent for a document
    /// sent directly.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<Origin>,
    /// What the entry holds.
    pub format: EntryFormat,
    /// With the format `document`: the document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document: Option<FreeDocument>,
    /// With the format `document` and a file origin: where each value is in the file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines: Option<Lines>,
    /// With the format `markdown`: the content of the file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// With the format `unreadable`: what the parser reported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ParseError>,
    /// With the format `document`, for an input; absent for an artifact.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_kind: Option<InputKind>,
}

/// The member each format requires.
fn entry_rules(schema: &mut Schema) {
    for (format, member) in [
        (EntryFormat::Document, "document"),
        (EntryFormat::Markdown, "text"),
        (EntryFormat::Unreadable, "error"),
    ] {
        rules::require_if_equals(schema, "format", format.id(), &[member]);
    }
}

/// The status of one artifact of the set on the default branch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BaseStatus {
    /// Identifier of the artifact.
    #[schemars(length(min = 1))]
    pub id: String,
    /// Its status on the default branch.
    pub status: Status,
}

/// The subset of the project policy the artifact model reads (SPEC-0021 R1): the types and
/// scopes of commits and the areas of epics and tasks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Policy {
    /// Types of commits, branches and pull request titles.
    pub types: Vec<Text>,
    /// Scopes of commits and pull request titles.
    pub scopes: Vec<Text>,
    /// Areas of epics and tasks, without the `area:` prefix.
    pub areas: Vec<Text>,
}

/// The canonical input of `wayloop validate`: a set of stored artifacts and inputs, validated
/// together in one pass. A member set to null is not given (ADR 0009 C2). Checks whose part of
/// the engine is not built yet are listed in `diagnostics.not_done` of the result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ValidateInput {
    /// One entry per stored artifact or input, in the order the caller sorted them; `[]` means
    /// none. Errors of documents are reported in this order.
    pub documents: Vec<DocumentEntry>,
    /// Statuses on the default branch of the artifacts of this set, for the checks of
    /// transitions and initial statuses (SPEC-0001 R13, R19). Absent: those checks are not
    /// done.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base: Option<Vec<BaseStatus>>,
    /// The subset of the project policy. Absent: the default policy applies.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<Policy>,
    /// Numbers of the ADRs of the project, for the warning on a citation `ADR <NNNN>` with no
    /// ADR of that number. Absent: citations are not checked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adrs: Option<Vec<AdrNumber>>,
    /// Language of the project: the language of the prose of a document without its own
    /// `language`. When it is absent and a check needs it, the default is listed in
    /// `meta.defaults_applied`; no check of this version reads it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<LanguageTag>,
}

/// What the engine read of one document. A member is absent when the document does not give
/// it, or gives it with another type.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub struct DocumentSummary {
    /// Artifact type, from the member `kind` of the document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<ArtifactKind>,
    /// Input type, from `input_kind` of the entry.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_kind: Option<InputKind>,
    /// Identifier of the artifact.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Status of the artifact, after migration to the current version of its schema.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Where the document was read, as given in the input.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<Origin>,
}

/// The data of a result of `validate` that accepted the set.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub struct ValidateData {
    /// One summary per entry of the input `documents`, in the same order; `[]` when the input
    /// has none.
    pub documents: Vec<DocumentSummary>,
}

#[cfg(test)]
mod tests {
    use super::{DocumentEntry, EntryFormat, ValidateData, ValidateInput};
    use crate::contract::{Direction, schema_for};
    use serde_json::json;

    #[test]
    fn the_input_is_closed_and_its_documents_and_lines_are_free_objects() {
        let schema = schema_for::<ValidateInput>(Direction::Input).unwrap();
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(schema["required"], json!(["documents"]));
        let entry = &schema["$defs"]["DocumentEntry"];
        assert_eq!(entry["additionalProperties"], false);
        for free in ["FreeDocument", "Lines"] {
            let free = &schema["$defs"][free];
            assert_eq!(free["type"], "object");
            assert!(free.get("additionalProperties").is_none(), "{free}");
        }
        assert_eq!(
            entry["allOf"][2],
            json!({
                "if": {"properties": {"format": {"const": "unreadable"}}, "required": ["format"]},
                "then": {"required": ["error"]}
            })
        );
        let origin = &schema["$defs"]["Origin"];
        assert_eq!(origin["properties"]["file"]["format"], "project-path");
        assert_eq!(origin["properties"]["issue"]["minimum"], 1);
    }

    #[test]
    fn the_data_is_open() {
        let schema = schema_for::<ValidateData>(Direction::Output).unwrap();
        assert!(schema.get("additionalProperties").is_none());
    }

    #[test]
    fn an_entry_reads_its_members_by_value() {
        let entry: DocumentEntry = serde_json::from_value(json!({
            "format": "document",
            "document": {"kind": "brief"},
            "origin": {"file": "plan/briefs/BRIEF-0001.yaml"},
            "lines": {"": 1.0, "/kind": 2}
        }))
        .unwrap();
        assert_eq!(entry.format, EntryFormat::Document);
        assert_eq!(entry.lines.unwrap().0["/kind"].get().get(), 2);
        assert_eq!(entry.document.unwrap().0["kind"], "brief");
    }
}
