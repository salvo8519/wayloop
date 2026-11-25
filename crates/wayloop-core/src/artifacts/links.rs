//! The links between artifacts and the hierarchy of plan items (SPEC-0001 R3, R4, R9, R15;
//! plan, stage 4).
//!
//! Every link resolves by identifier through the index of the identifier stage
//! ([`Indexed`]), never by path (ADR 0008): a living specification moved to another folder and
//! renamed is still found. The links of an artifact are read only when it passed its schema,
//! so that a broken shape does not cause a wave of link errors; a target resolves whatever its
//! other errors, as long as its `id` raised none.
//!
//! - A link to an identifier no artifact of the set carries is `link_missing` with the reason
//!   `missing` (R3, R9): a link to an item that is only in the tracker too, until the tracker
//!   adapter of SPEC-0005 sends tracker items (plan, Canonical input).
//! - A link to a superseded artifact is `link_superseded` with the reason `state` and, in
//!   `fix`, the identifiers of its `superseded_by` (R4). Three links name a superseded target
//!   by design and accept one: `superseded_by` itself, `expanded_from` of a task, whose task is
//!   superseded by its subtasks, and `depends_on` of a task, whose superseded targets are
//!   refused by the check of parts (R18).
//! - A requirement cited by a task or a change delta, or named by an acceptance criterion of a
//!   living specification, is one of the requirements of that specification (R3, R9); a
//!   removed requirement is not in force, which gives the reason `state` with no fix.
//! - An artifact in a terminal status, `done` or `superseded`, is not changed again: each of
//!   its broken links is the warning `link_broken_terminal`, with the reason of the error it
//!   replaces (R3). A hand-over record has no status and is never changed: it is read as
//!   terminal.
//! - A task or a story in a non-terminal status whose milestone differs from that of its epic,
//!   or a task whose epic differs from that of its story, is `hierarchy_mismatch` with the
//!   reason `conflict` at the member it declares, the member of the parent in `pointers` and
//!   the value reached through the parent in `expected` (R15).
//!
//! - A citation `ADR <n>` in `related` of a living specification, `ADR` followed by a space and
//!   ASCII digits, with no ADR of that number among `adrs`, is the warning
//!   `adr_citation_missing` with the reason `missing` at the entry that holds it (Q5; decided by
//!   the maintainer on 2026-11-25); `ADR 0014` and `ADR 14` cite the same ADR. It runs only when
//!   `adrs` is given.
//!
//! No error of a link or of the hierarchy carries `related` (decided by the maintainer on
//! 2026-11-25): the pointer and the message name the target.
//!
//! Errors of one artifact come in the order of the schema pass: its members in the byte order
//! of their keys, the elements of a list by index; at one member, the error of the link before
//! that of the hierarchy.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};
use wayloop_schemas::artifact::{ArtifactKind, OperationKind, RequirementStatus};
use wayloop_schemas::contract::rules::{ID, KIND, STATUS, SUPERSEDED_BY};
use wayloop_schemas::validate::Status;

use super::codes;
use super::identifiers::Indexed;
use crate::{Error, Pointer, Reason};

// Members of the documents that hold links, as the contract types of `wayloop-schemas` spell
// them.
const ACCEPTANCE_CRITERIA: &str = "acceptance_criteria";
const BRIEF: &str = "brief";
const COUNCIL_REPORT: &str = "council_report";
const DEPENDS_ON: &str = "depends_on";
const EPIC: &str = "epic";
const EXPANDED_FROM: &str = "expanded_from";
const MILESTONE: &str = "milestone";
const OP: &str = "op";
const OPERATIONS: &str = "operations";
const RELATED: &str = "related";
const REQUIREMENT: &str = "requirement";
const REQUIREMENTS: &str = "requirements";
const SPEC: &str = "spec";
const SPECIFICATION: &str = "specification";
const STORIES: &str = "stories";
const STORY: &str = "story";
const SUBJECT: &str = "subject";
const TASK: &str = "task";
const TASKS: &str = "tasks";

/// What the links of the set gave: errors and warnings, each in the order of the index, then
/// of the members of its artifact.
#[derive(Debug, Default)]
pub(super) struct Found {
    pub(super) errors: Vec<Error>,
    pub(super) warnings: Vec<Error>,
}

/// Whether a link accepts a superseded target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Accepts {
    /// The target must not be superseded (R4).
    Active,
    /// The target may be in any status: the link names a superseded artifact by design.
    Any,
}

/// The errors and warnings of the links of every artifact of `index` that passed its schema.
pub(super) fn check(index: &[Indexed<'_>]) -> Found {
    let mut by_id: BTreeMap<&str, &Indexed<'_>> = BTreeMap::new();
    for artifact in index {
        // Two artifacts with one identifier are refused by the identifier stage; a link to that
        // identifier resolves to the first of them.
        by_id.entry(artifact.id.as_str()).or_insert(artifact);
    }
    let mut found = Found::default();
    for holder in index {
        let Some(document) = &holder.read else {
            continue;
        };
        let kind = document.get(KIND).and_then(Value::as_str);
        let Some(kind) = kind.and_then(ArtifactKind::from_id) else {
            continue;
        };
        let mut links = Links {
            by_id: &by_id,
            holder,
            document,
            terminal: terminal(kind, holder.status()),
            found: &mut found,
        };
        links.of(kind);
    }
    found
}

/// The warnings of the citations of ADRs in `related` of every living specification of `index`
/// that passed its schema, with no ADR of that number among `adrs`.
pub(super) fn citations(index: &[Indexed<'_>], adrs: &BTreeSet<u64>) -> Vec<Error> {
    let mut warnings = Vec::new();
    for artifact in index {
        let Some(document) = &artifact.read else {
            continue;
        };
        if text(document, KIND) != Some(ArtifactKind::Specification.id()) {
            continue;
        }
        for (i, entry) in texts(document, RELATED) {
            for cited in cited_adrs(entry) {
                if cited.parse::<u64>().is_ok_and(|n| adrs.contains(&n)) {
                    continue;
                }
                let message = format!(
                    "{} cites ADR {cited}, which is not an ADR of the project.",
                    artifact.id
                );
                let at = Pointer::root().join(RELATED).join_index(i);
                let warning = Error::new(codes::ADR_CITATION_MISSING, message, at, Reason::Missing);
                warnings.push(artifact.of(warning));
            }
        }
    }
    warnings
}

/// The numbers, as written, of the citations `ADR <n>` in `entry`: the word `ADR`, a space and
/// ASCII digits, neither run joined to a letter or a digit.
fn cited_adrs(entry: &str) -> Vec<&str> {
    const WORD: &str = "ADR ";
    let joined = |c: char| c.is_alphanumeric() || c == '_';
    entry
        .match_indices(WORD)
        .filter(|(start, _)| {
            let before = entry.get(..*start).and_then(|b| b.chars().next_back());
            !before.is_some_and(joined)
        })
        .filter_map(|(start, _)| {
            let rest = entry.get(start.saturating_add(WORD.len())..)?;
            let end = rest
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(rest.len());
            let digits = rest.get(..end)?;
            let after = rest.get(end..).and_then(|a| a.chars().next());
            (!digits.is_empty() && !after.is_some_and(joined)).then_some(digits)
        })
        .collect()
}

/// Whether an artifact of `kind` in `status` is not changed again: `done` and `superseded` are
/// terminal for every type, and a hand-over record, which has no status, is never changed.
fn terminal(kind: ArtifactKind, status: Option<Status>) -> bool {
    kind == ArtifactKind::Handover || matches!(status, Some(Status::Done | Status::Superseded))
}

/// The text at `key` of `document`.
fn text<'d>(document: &'d Map<String, Value>, key: &str) -> Option<&'d str> {
    document.get(key)?.as_str()
}

/// The elements of the list at `key` of `document`, each with its index.
fn elements<'d>(
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
fn texts<'d>(
    document: &'d Map<String, Value>,
    key: &str,
) -> impl Iterator<Item = (usize, &'d str)> {
    elements(document, key).filter_map(|(i, v)| Some((i, v.as_str()?)))
}

/// The status of the requirement `id` of the living specification `specification`, `active`
/// when it gives none; `None` when it has no such requirement.
fn requirement_status(specification: &Map<String, Value>, id: &str) -> Option<RequirementStatus> {
    let (_, requirement) = elements(specification, REQUIREMENTS)
        .find(|(_, r)| r.get(ID).and_then(Value::as_str) == Some(id))?;
    let status = requirement.get(STATUS).and_then(Value::as_str);
    status.map_or(Some(RequirementStatus::Active), RequirementStatus::from_id)
}

/// The links of one artifact, `holder`, read from `document`, its contract form.
struct Links<'s, 'i, 'a> {
    by_id: &'s BTreeMap<&'i str, &'i Indexed<'a>>,
    holder: &'i Indexed<'a>,
    document: &'i Map<String, Value>,
    terminal: bool,
    found: &'s mut Found,
}

impl<'i, 'a> Links<'_, 'i, 'a> {
    /// Checks the links of an artifact of `kind`, in the byte order of their members.
    fn of(&mut self, kind: ArtifactKind) {
        let root = Pointer::root();
        match kind {
            ArtifactKind::Brief => {}
            ArtifactKind::CouncilReport => self.one(BRIEF, Accepts::Active),
            ArtifactKind::Gate => self.one(SUBJECT, Accepts::Active),
            ArtifactKind::Milestone => self.one(COUNCIL_REPORT, Accepts::Active),
            ArtifactKind::Epic => self.one(MILESTONE, Accepts::Active),
            ArtifactKind::Story => {
                let epic = self.one_resolved(EPIC, Accepts::Active);
                self.one(MILESTONE, Accepts::Active);
                self.hierarchy(MILESTONE, EPIC, epic);
                self.many(SPECIFICATION, Accepts::Active);
            }
            ArtifactKind::Specification => {
                let (holder, document) = (self.holder, self.document);
                for (i, criterion) in elements(document, ACCEPTANCE_CRITERIA) {
                    let at = root.join(ACCEPTANCE_CRITERIA).join_index(i);
                    self.requirements(&at, criterion, document, &holder.id);
                }
                self.one(MILESTONE, Accepts::Active);
                self.many(STORIES, Accepts::Active);
            }
            ArtifactKind::Change => {
                let document = self.document;
                let specification = text(document, SPECIFICATION).and_then(|s| self.find(s));
                for (i, operation) in elements(document, OPERATIONS) {
                    self.operation(
                        &root.join(OPERATIONS).join_index(i),
                        operation,
                        specification,
                    );
                }
                self.one(SPECIFICATION, Accepts::Active);
            }
            ArtifactKind::Task => self.task(),
            ArtifactKind::Handover => self.one(TASK, Accepts::Any),
        }
        self.many(SUPERSEDED_BY, Accepts::Any);
        if kind == ArtifactKind::Change {
            self.many(TASKS, Accepts::Active);
        }
    }

    /// The links of a task, whose members come before `superseded_by`.
    fn task(&mut self) {
        let root = Pointer::root();
        let document = self.document;
        self.many(DEPENDS_ON, Accepts::Any);
        let epic = self.one_resolved(EPIC, Accepts::Active);
        let story = text(document, STORY).and_then(|s| self.find(s));
        self.hierarchy(EPIC, STORY, story);
        self.one(EXPANDED_FROM, Accepts::Any);
        self.one(MILESTONE, Accepts::Active);
        self.hierarchy(MILESTONE, EPIC, epic);
        for (i, reference) in elements(document, SPECIFICATION) {
            let at = root.join(SPECIFICATION).join_index(i);
            let Some(cited) = reference.get(SPEC).and_then(Value::as_str) else {
                continue;
            };
            let target = self.find(cited);
            if let Some(read) = target.and_then(|t| t.read.as_ref()) {
                self.requirements(&at, reference, read, cited);
            }
            self.link(at.join(SPEC), cited, Accepts::Active);
        }
        self.one(STORY, Accepts::Active);
    }

    /// The artifact of the set that carries `id`.
    fn find(&self, id: &str) -> Option<&'i Indexed<'a>> {
        self.by_id.get(id).copied()
    }

    /// Checks the link at the member `key`, when the artifact gives it.
    fn one(&mut self, key: &str, accepts: Accepts) {
        self.one_resolved(key, accepts);
    }

    /// Checks the link at the member `key`, when the artifact gives it, and gives its target
    /// when it resolves.
    fn one_resolved(&mut self, key: &str, accepts: Accepts) -> Option<&'i Indexed<'a>> {
        let target = text(self.document, key)?;
        self.link(Pointer::root().join(key), target, accepts)
    }

    /// Checks each link of the list at the member `key`.
    fn many(&mut self, key: &str, accepts: Accepts) {
        let document = self.document;
        for (i, target) in texts(document, key) {
            self.link(Pointer::root().join(key).join_index(i), target, accepts);
        }
    }

    /// Checks the link at `at` to `target`, and gives the target when it resolves.
    fn link(&mut self, at: Pointer, target: &str, accepts: Accepts) -> Option<&'i Indexed<'a>> {
        let holder = &self.holder.id;
        let Some(found) = self.find(target) else {
            let message = format!("{holder} links {target}, which no artifact of the set carries.");
            self.broken(at, Reason::Missing, message, None);
            return None;
        };
        if accepts == Accepts::Active && found.status() == Some(Status::Superseded) {
            let replacements = found.superseded_by();
            let message = if replacements.is_empty() {
                format!("{holder} links {target}, which is superseded.")
            } else {
                format!(
                    "{holder} links {target}, which is superseded by {}.",
                    replacements.join(", ")
                )
            };
            let fix = Some(replacements).filter(|r| !r.is_empty());
            self.broken(at, Reason::State, message, fix);
        }
        Some(found)
    }

    /// Checks each requirement listed at `requirements` of `part`, at `at`, against the living
    /// specification `specification`, identified by `cited`.
    fn requirements(
        &mut self,
        at: &Pointer,
        part: &Value,
        specification: &Map<String, Value>,
        cited: &str,
    ) {
        let listed = part.get(REQUIREMENTS).and_then(Value::as_array);
        for (j, requirement) in listed
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .enumerate()
        {
            if let Some(requirement) = requirement.as_str() {
                let at = at.join(REQUIREMENTS).join_index(j);
                self.requirement(at, specification, cited, requirement);
            }
        }
    }

    /// Checks the requirement of an operation of a change delta on an existing requirement of
    /// its specification, when that specification resolves and passed its schema.
    fn operation(&mut self, at: &Pointer, operation: &Value, specification: Option<&Indexed<'_>>) {
        let op = operation.get(OP).and_then(Value::as_str);
        let on_a_requirement = op
            .and_then(OperationKind::from_id)
            .is_some_and(|op| OperationKind::ON_A_REQUIREMENT.contains(&op));
        let requirement = operation.get(REQUIREMENT).and_then(Value::as_str);
        let (Some(target), Some(requirement), true) =
            (specification, requirement, on_a_requirement)
        else {
            return;
        };
        if let Some(read) = &target.read {
            self.requirement(at.join(REQUIREMENT), read, &target.id, requirement);
        }
    }

    /// Checks that `requirement`, at `at`, is an active requirement of `specification`,
    /// identified by `cited`.
    fn requirement(
        &mut self,
        at: Pointer,
        specification: &Map<String, Value>,
        cited: &str,
        requirement: &str,
    ) {
        let holder = &self.holder.id;
        match requirement_status(specification, requirement) {
            None => {
                let message =
                    format!("{holder} cites {cited} {requirement}, which {cited} does not have.");
                self.broken(at, Reason::Missing, message, None);
            }
            Some(RequirementStatus::Removed) => {
                let message = format!("{holder} cites {cited} {requirement}, which is removed.");
                self.broken(at, Reason::State, message, None);
            }
            Some(RequirementStatus::Active) => {}
        }
    }

    /// Records a broken link at `at`: an error of the reason `reason`, or the warning
    /// `link_broken_terminal` when the artifact is in a terminal status (R3).
    fn broken(&mut self, at: Pointer, reason: Reason, message: String, fix: Option<Vec<String>>) {
        let code = match (self.terminal, reason) {
            (true, _) => codes::LINK_BROKEN_TERMINAL,
            (false, Reason::Missing) => codes::LINK_MISSING,
            (false, _) => codes::LINK_SUPERSEDED,
        };
        let mut error = Error::new(code, message, at, reason);
        if let Some(fix) = fix {
            error = error.with_fix(fix);
        }
        let error = self.holder.of(error);
        if self.terminal {
            self.found.warnings.push(error);
        } else {
            self.found.errors.push(error);
        }
    }

    /// Checks that the member `key` of a non-terminal artifact equals the same member of
    /// `parent`, the artifact its member `via` links to, when it resolves and passed its schema
    /// (R15).
    fn hierarchy(&mut self, key: &str, via: &str, parent: Option<&Indexed<'_>>) {
        if self.terminal {
            return;
        }
        let declared = text(self.document, key);
        let reached = parent
            .and_then(|p| p.read.as_ref())
            .and_then(|p| text(p, key));
        let (Some(declared), Some(reached), Some(parent)) = (declared, reached, parent) else {
            return;
        };
        if declared == reached {
            return;
        }
        let message = format!(
            "{} declares the {key} {declared}, but its {via} {} belongs to {reached}.",
            self.holder.id, parent.id
        );
        let root = Pointer::root();
        let error = Error::new(
            codes::HIERARCHY_MISMATCH,
            message,
            root.join(key),
            Reason::Conflict,
        )
        .with_pointers([root.join(via)])
        .with_expected([reached]);
        self.found.errors.push(self.holder.of(error));
    }
}

#[cfg(test)]
mod tests;
