//! Error codes of the artifact model (SPEC-0001, Errors) and of the command `validate`.
//!
//! A new meaning gets a new code; codes are never renamed or reused (ADR 0009 C8). Every code
//! of the Errors table of SPEC-0001 is listed here, also those whose check is not built yet,
//! so that one place holds them all. The codes a version of `validate` reports are documented
//! in `docs/en/reference/validate.md`.

use crate::Code;

/// A document breaks the schema of its type (SPEC-0001 R1, R6, R12); also a document without
/// a known `kind` and a file that did not parse.
pub const ARTIFACT_SCHEMA: Code = Code::new("artifact_schema");
/// A link of an artifact in a non-terminal status points to nothing (R3, R4, R9).
pub const LINK_MISSING: Code = Code::new("link_missing");
/// A link of an artifact in a non-terminal status points to a superseded artifact (R4).
pub const LINK_SUPERSEDED: Code = Code::new("link_superseded");
/// A broken link of an artifact in a terminal status: a warning (R3).
pub const LINK_BROKEN_TERMINAL: Code = Code::new("link_broken_terminal");
/// An epic or task title with a type prefix, a bracket tag or a final full stop (R7).
pub const TITLE_FORMAT: Code = Code::new("title_format");
/// Two epic or task titles equal after normalisation (R8).
pub const TITLE_DUPLICATE: Code = Code::new("title_duplicate");
/// A pull request that does not link exactly one planned issue (R10).
pub const PULL_REQUEST_LINK: Code = Code::new("pull_request_link");
/// A branch name outside the naming grammar (R11).
pub const BRANCH_NAME: Code = Code::new("branch_name");
/// A commit header outside the naming grammar (R11).
pub const COMMIT_HEADER: Code = Code::new("commit_header");
/// A status change that the artifact type does not allow (R13).
pub const TRANSITION_REFUSED: Code = Code::new("transition_refused");
/// An identifier not in the canonical form of its type (R14).
pub const IDENTIFIER_FORMAT: Code = Code::new("identifier_format");
/// Two artifacts with one identifier (R14, R23).
pub const IDENTIFIER_DUPLICATE: Code = Code::new("identifier_duplicate");
/// A warning: the file name or the folder number of a stored artifact names another identifier
/// than its `id` (R14; decided by the maintainer on 2026-11-25).
pub const IDENTIFIER_LOCATION: Code = Code::new("identifier_location");
/// A milestone or an epic that differs from the one reached through the parent (R15).
pub const HIERARCHY_MISMATCH: Code = Code::new("hierarchy_mismatch");
/// A `schema_version` that the type does not know (R16).
pub const SCHEMA_VERSION_UNKNOWN: Code = Code::new("schema_version_unknown");
/// A status in the file that disagrees with the approval recorded in the ledger (R17).
pub const APPROVAL_CONFLICT: Code = Code::new("approval_conflict");
/// A part of an artifact that is duplicated, dangling or outside its options (R18).
pub const PART_INVALID: Code = Code::new("part_invalid");
/// An artifact created in a status that is not initial for its type (R19).
pub const INITIAL_STATUS: Code = Code::new("initial_status");
/// Two operations of a change delta on one requirement (R20).
pub const DELTA_CONFLICT: Code = Code::new("delta_conflict");
/// A hand-over input that breaks its schema or sets an engine field (R22).
pub const HANDOVER_INPUT: Code = Code::new("handover_input");

/// A warning: the documents were discovered in a project where no folder of a location exists,
/// for example because the command did not run at the root of the project (R26).
pub const ARTIFACT_LOCATION_MISSING: Code = Code::new("artifact_location_missing");

/// Every code of SPEC-0001, in the order of its Errors table.
pub const ALL: [Code; 21] = [
    ARTIFACT_SCHEMA,
    LINK_MISSING,
    LINK_SUPERSEDED,
    LINK_BROKEN_TERMINAL,
    TITLE_FORMAT,
    TITLE_DUPLICATE,
    PULL_REQUEST_LINK,
    BRANCH_NAME,
    COMMIT_HEADER,
    TRANSITION_REFUSED,
    IDENTIFIER_FORMAT,
    IDENTIFIER_DUPLICATE,
    IDENTIFIER_LOCATION,
    HIERARCHY_MISMATCH,
    SCHEMA_VERSION_UNKNOWN,
    APPROVAL_CONFLICT,
    PART_INVALID,
    INITIAL_STATUS,
    DELTA_CONFLICT,
    HANDOVER_INPUT,
    ARTIFACT_LOCATION_MISSING,
];

#[cfg(test)]
mod tests {
    use super::ALL;
    use std::collections::BTreeSet;

    #[test]
    fn the_codes_are_those_of_the_errors_table_of_spec_0001_once_each() {
        let all = ALL;
        let ids: Vec<&str> = all.iter().map(wayloop_schemas::Code::id).collect();
        assert_eq!(
            ids,
            [
                "artifact_schema",
                "link_missing",
                "link_superseded",
                "link_broken_terminal",
                "title_format",
                "title_duplicate",
                "pull_request_link",
                "branch_name",
                "commit_header",
                "transition_refused",
                "identifier_format",
                "identifier_duplicate",
                "identifier_location",
                "hierarchy_mismatch",
                "schema_version_unknown",
                "approval_conflict",
                "part_invalid",
                "initial_status",
                "delta_conflict",
                "handover_input",
                "artifact_location_missing",
            ]
        );
        assert_eq!(ids.iter().collect::<BTreeSet<_>>().len(), ids.len());
    }
}
