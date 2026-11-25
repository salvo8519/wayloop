//! The names of the members of the documents of the artifact model that the checks of the core
//! read or write, as the contract types of `wayloop-schemas` spell them: each is defined once
//! for the core (AGENTS.md, one source for every value). The members every artifact has,
//! `kind`, `schema_version`, `id`, `title`, `status` and `superseded_by`, are those of
//! `wayloop_schemas::contract::rules`; the ones the readers of this module use are re-exported
//! here, and no name is defined in both places. The readers of members, [`text`], [`texts`],
//! [`elements`] and [`texts_in`], serve every check that reads a document the schema accepted.

use serde_json::{Map, Value};
pub(super) use wayloop_schemas::contract::rules::{ID, STATUS, SUPERSEDED_BY};

// Members of artifacts.
pub(super) const ACCEPTANCE_CRITERIA: &str = "acceptance_criteria";
pub(super) const BRIEF: &str = "brief";
pub(super) const CAPABILITY: &str = "capability";
pub(super) const COMMIT_SCOPE: &str = "commit_scope";
pub(super) const COUNCIL_REPORT: &str = "council_report";
pub(super) const DEPENDS_ON: &str = "depends_on";
pub(super) const EPIC: &str = "epic";
pub(super) const EXPANDED_FROM: &str = "expanded_from";
pub(super) const EXTRA_SECTIONS: &str = "extra_sections";
pub(super) const INTERFACES: &str = "interfaces";
pub(super) const LANGUAGE: &str = "language";
pub(super) const MILESTONE: &str = "milestone";
pub(super) const OPEN_QUESTIONS: &str = "open_questions";
pub(super) const OPERATIONS: &str = "operations";
pub(super) const OWNER: &str = "owner";
pub(super) const PROBLEM: &str = "problem";
pub(super) const RELATED: &str = "related";
pub(super) const REQUIREMENTS: &str = "requirements";
pub(super) const SCOPE_IN: &str = "scope_in";
pub(super) const SCOPE_OUT: &str = "scope_out";
pub(super) const SPECIFICATION: &str = "specification";
pub(super) const STORIES: &str = "stories";
pub(super) const STORY: &str = "story";
pub(super) const SUBJECT: &str = "subject";
pub(super) const TASK: &str = "task";
pub(super) const TASKS: &str = "tasks";
pub(super) const USER_STORIES: &str = "user_stories";

// Members of the parts of artifacts: requirements, acceptance criteria, open questions, extra
// sections, operations of a change delta and references of a task.
pub(super) const HEADING: &str = "heading";
pub(super) const NAME: &str = "name";
pub(super) const OP: &str = "op";
pub(super) const REQUIREMENT: &str = "requirement";
pub(super) const RESOLVED: &str = "resolved";
pub(super) const RUBRIC: &str = "rubric";
pub(super) const SPEC: &str = "spec";
pub(super) const TEXT: &str = "text";
pub(super) const VERIFICATION: &str = "verification";

/// The text at `key` of `document`.
pub(super) fn text<'d>(document: &'d Map<String, Value>, key: &str) -> Option<&'d str> {
    document.get(key)?.as_str()
}

/// The elements of the list at `key` of `document`, each with its index.
pub(super) fn elements<'d>(
    document: &'d Map<String, Value>,
    key: &str,
) -> impl Iterator<Item = (usize, &'d Value)> {
    document
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .enumerate()
}

/// The texts of the list at `key` of `document`, each with its index.
pub(super) fn texts<'d>(
    document: &'d Map<String, Value>,
    key: &str,
) -> impl Iterator<Item = (usize, &'d str)> {
    elements(document, key).filter_map(|(i, v)| Some((i, v.as_str()?)))
}

/// The texts of the list at `key` of `value`, when it is an object, each with its index: the
/// reader of a list inside a part of a document, such as the requirements of a reference.
pub(super) fn texts_in<'d>(value: &'d Value, key: &str) -> impl Iterator<Item = (usize, &'d str)> {
    value
        .as_object()
        .into_iter()
        .flat_map(move |members| texts(members, key))
}
