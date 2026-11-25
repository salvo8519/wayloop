//! The links between artifacts and the hierarchy of plan items (SPEC-0001 R3, R4, R9, R15;
//! plan, stage 4).
//!
//! Every link resolves by identifier through the map of carriers of the identifier stage
//! ([`Carriers`]), never by path (ADR 0008): a living specification moved to another folder
//! and renamed is still found. Only an artifact with no error of its own, of its schema or of
//! the normalisation of its Markdown, has its links checked, so that a broken document causes
//! no wave of link errors; every artifact of the index is a target, whatever its other errors
//! (decided by the maintainer on 2026-11-25).
//!
//! - A link to an identifier no artifact of the set carries is `link_missing` with the reason
//!   `missing` (R3, R9): a link to an item that is only in the tracker too, until the tracker
//!   adapter of SPEC-0005 sends tracker items (plan, Canonical input).
//! - A link to a superseded artifact is `link_superseded` with the reason `state`, its
//!   replacements, the `superseded_by` of the target, in `expected`, and in `fix` the one
//!   replacement when there is exactly one (R4; decided by the maintainer on 2026-11-25).
//!   `superseded_by` itself must name artifacts that are not superseded. `expanded_from` and
//!   `depends_on` of a task accept a superseded target: they record the history of the work, a
//!   subtask names the task it was expanded from, which it supersedes, and a dependency on a
//!   superseded task is refused by the check of parts (R18).
//! - A requirement cited by a task or a change delta, or named by an acceptance criterion of a
//!   living specification, is a requirement of that specification (R3, R9); a removed
//!   requirement is not in force, which gives `link_superseded` with no replacement.
//! - An artifact in a terminal status, `done` or `superseded`, is not changed again: each of
//!   its broken links is the warning `link_broken_terminal`, with the reason, `expected` and
//!   `fix` of the error it replaces (R3). A hand-over record has no status and is never
//!   changed: it is read as terminal.
//! - A task or a story in a non-terminal status whose milestone differs from that of its epic,
//!   or a task whose epic differs from that of its story, is `hierarchy_mismatch` with the
//!   reason `conflict` at the member it declares, the member of the parent in `pointers` and
//!   the value reached through the parent in `expected` (R15).
//! - A citation of an ADR in `related` of a living specification, the word `ADR` followed by a
//!   number, and each bare number that follows it in the same entry or in the entries right
//!   after it, as in `ADR 0002, 0003`, names an ADR; with no ADR of that number among `adrs`, it
//!   is the warning `adr_citation_missing` with the reason `missing` at the entry that holds it
//!   (Q5; decided by the maintainer on 2026-11-25). `ADR 0014` and `ADR 14` cite the same ADR.
//!   It runs only when `adrs` is given.
//!
//! No error of a link or of the hierarchy carries `related` (decided by the maintainer on
//! 2026-11-25): the pointer and the message name the target.
//!
//! The members of each type are read in the byte order of their keys, as the schema pass
//! reports them, from the table [`rules`]; the elements of a list by index; at one member, the
//! error of the link before that of the hierarchy.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};
use wayloop_schemas::artifact::{ArtifactKind, OperationKind, RequirementStatus};
use wayloop_schemas::validate::Status;

use super::codes;
use super::identifiers::{Carriers, Indexed};
use super::members::{
    ACCEPTANCE_CRITERIA, BRIEF, COUNCIL_REPORT, DEPENDS_ON, EPIC, EXPANDED_FROM, ID, MILESTONE, OP,
    OPERATIONS, RELATED, REQUIREMENT, REQUIREMENTS, SPEC, SPECIFICATION, STATUS, STORIES, STORY,
    SUBJECT, SUPERSEDED_BY, TASK, TASKS, elements, text, texts,
};
use crate::{Error, Pointer, Reason};

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
    /// The target may be in any status: the link records the history of the work.
    Any,
}

/// One rule of the links of a type.
#[derive(Debug, Clone, Copy)]
enum Rule {
    /// The member holds one identifier.
    One(&'static str, Accepts),
    /// The member holds a list of identifiers.
    Many(&'static str, Accepts),
    /// The member equals the same member of the artifact that `via` links to (R15).
    Parent {
        member: &'static str,
        via: &'static str,
    },
    /// The requirements of each acceptance criterion are requirements of the specification
    /// itself.
    Criteria,
    /// The requirement of each operation on an existing requirement is a requirement of the
    /// specification of the delta.
    Operations,
    /// Each reference of a task: the requirements it cites, then its specification.
    References,
}

/// The rules of the links of `kind`, in the byte order of the members they read.
const fn rules(kind: ArtifactKind) -> &'static [Rule] {
    use Accepts::{Active, Any};
    use Rule::{Criteria, Many, One, Operations, Parent, References};
    match kind {
        ArtifactKind::Brief => &[Many(SUPERSEDED_BY, Active)],
        ArtifactKind::CouncilReport => &[One(BRIEF, Active), Many(SUPERSEDED_BY, Active)],
        ArtifactKind::Gate => &[One(SUBJECT, Active), Many(SUPERSEDED_BY, Active)],
        ArtifactKind::Milestone => &[One(COUNCIL_REPORT, Active), Many(SUPERSEDED_BY, Active)],
        ArtifactKind::Epic => &[One(MILESTONE, Active), Many(SUPERSEDED_BY, Active)],
        ArtifactKind::Story => &[
            One(EPIC, Active),
            One(MILESTONE, Active),
            Parent {
                member: MILESTONE,
                via: EPIC,
            },
            Many(SPECIFICATION, Active),
            Many(SUPERSEDED_BY, Active),
        ],
        ArtifactKind::Specification => &[
            Criteria,
            One(MILESTONE, Active),
            Many(STORIES, Active),
            Many(SUPERSEDED_BY, Active),
        ],
        ArtifactKind::Change => &[
            Operations,
            One(SPECIFICATION, Active),
            Many(SUPERSEDED_BY, Active),
            Many(TASKS, Active),
        ],
        ArtifactKind::Task => &[
            Many(DEPENDS_ON, Any),
            One(EPIC, Active),
            Parent {
                member: EPIC,
                via: STORY,
            },
            One(EXPANDED_FROM, Any),
            One(MILESTONE, Active),
            Parent {
                member: MILESTONE,
                via: EPIC,
            },
            References,
            One(STORY, Active),
            Many(SUPERSEDED_BY, Active),
        ],
        ArtifactKind::Handover => &[One(TASK, Any)],
    }
}

/// The requirements of a living specification by their identifier, with their status, absent
/// meaning `active`.
type Requirements<'d> = BTreeMap<&'d str, RequirementStatus>;

/// The requirements of every living specification of `index` that passed its schema, by the
/// index of its document: built once, so that each citation is one lookup.
fn requirements<'i>(index: &'i [Indexed<'_>]) -> BTreeMap<u64, Requirements<'i>> {
    index
        .iter()
        .filter(|artifact| artifact.kind == ArtifactKind::Specification)
        .filter_map(|artifact| {
            let read = artifact.read.as_ref()?;
            let listed = elements(read, REQUIREMENTS).filter_map(|(_, requirement)| {
                let id = requirement.get(ID)?.as_str()?;
                let status = requirement.get(STATUS).and_then(Value::as_str);
                let status =
                    status.map_or(Some(RequirementStatus::Active), RequirementStatus::from_id)?;
                Some((id, status))
            });
            Some((artifact.document, listed.collect()))
        })
        .collect()
}

/// The errors and warnings of the links of every artifact of `index` with no error of its own;
/// `carriers` is the map of the identifiers of `index`.
pub(super) fn check(index: &[Indexed<'_>], carriers: &Carriers<'_, '_>) -> Found {
    let requirements = requirements(index);
    let mut found = Found::default();
    for holder in index.iter().filter(|artifact| artifact.clean) {
        let Some(document) = &holder.read else {
            continue;
        };
        let mut links = Links {
            carriers,
            requirements: &requirements,
            holder,
            document,
            terminal: terminal(holder.kind, holder.status()),
            found: &mut found,
        };
        for rule in rules(holder.kind) {
            links.apply(*rule);
        }
    }
    found
}

/// Whether an artifact of `kind` in `status` is not changed again: `done` and `superseded` are
/// terminal for every type, and a hand-over record, which has no status, is never changed.
fn terminal(kind: ArtifactKind, status: Option<Status>) -> bool {
    kind == ArtifactKind::Handover || matches!(status, Some(Status::Done | Status::Superseded))
}

/// The links of one artifact, `holder`, read from `document`, its contract form.
struct Links<'s, 'i, 'a> {
    carriers: &'s Carriers<'i, 'a>,
    requirements: &'s BTreeMap<u64, Requirements<'i>>,
    holder: &'i Indexed<'a>,
    document: &'i Map<String, Value>,
    terminal: bool,
    found: &'s mut Found,
}

impl<'i, 'a> Links<'_, 'i, 'a> {
    /// Applies one rule of the type of the artifact.
    fn apply(&mut self, rule: Rule) {
        let root = Pointer::root();
        let document = self.document;
        match rule {
            Rule::One(key, accepts) => {
                if let Some(target) = text(document, key) {
                    self.link(root.join(key), target, accepts);
                }
            }
            Rule::Many(key, accepts) => {
                for (i, target) in texts(document, key) {
                    self.link(root.join(key).join_index(i), target, accepts);
                }
            }
            Rule::Parent { member, via } => self.hierarchy(member, via),
            Rule::Criteria => {
                let own = self.holder;
                for (i, criterion) in elements(document, ACCEPTANCE_CRITERIA) {
                    let at = root.join(ACCEPTANCE_CRITERIA).join_index(i);
                    self.cited(&at, criterion, own);
                }
            }
            Rule::Operations => {
                let specification = text(document, SPECIFICATION).and_then(|s| self.find(s));
                for (i, operation) in elements(document, OPERATIONS) {
                    let at = root.join(OPERATIONS).join_index(i);
                    self.operation(&at, operation, specification);
                }
            }
            Rule::References => {
                for (i, reference) in elements(document, SPECIFICATION) {
                    let Some(cited) = reference.get(SPEC).and_then(Value::as_str) else {
                        continue;
                    };
                    let at = root.join(SPECIFICATION).join_index(i);
                    if let Some(specification) = self.find(cited) {
                        self.cited(&at, reference, specification);
                    }
                    self.link(at.join(SPEC), cited, Accepts::Active);
                }
            }
        }
    }

    /// The artifact of the set that carries `id`, the first when several do.
    fn find(&self, id: &str) -> Option<&'i Indexed<'a>> {
        self.carriers.get(id)?.first().copied()
    }

    /// Checks the link at `at` to `target`.
    fn link(&mut self, at: Pointer, target: &str, accepts: Accepts) {
        let holder = &self.holder.id;
        let Some(found) = self.find(target) else {
            let message = format!("{holder} links {target}, which no artifact of the set carries.");
            self.broken(at, Reason::Missing, message, &[]);
            return;
        };
        if accepts == Accepts::Any || found.status() != Some(Status::Superseded) {
            return;
        }
        let replacements = found.superseded_by();
        let message = if replacements.is_empty() {
            format!("{holder} links {target}, which is superseded.")
        } else {
            format!(
                "{holder} links {target}, which is superseded by {}.",
                replacements.join(", ")
            )
        };
        self.broken(at, Reason::State, message, &replacements);
    }

    /// Checks each requirement listed at `requirements` of `part`, at `at`, against the living
    /// specification `specification`, when it passed its schema.
    fn cited(&mut self, at: &Pointer, part: &Value, specification: &Indexed<'_>) {
        let Some(requirements) = self.requirements.get(&specification.document) else {
            return;
        };
        let listed = part.get(REQUIREMENTS).and_then(Value::as_array);
        for (j, requirement) in listed
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .enumerate()
        {
            if let Some(requirement) = requirement.as_str() {
                let at = at.join(REQUIREMENTS).join_index(j);
                self.requirement(at, requirements, &specification.id, requirement);
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
        if let Some(requirements) = self.requirements.get(&target.document) {
            self.requirement(at.join(REQUIREMENT), requirements, &target.id, requirement);
        }
    }

    /// Checks that `requirement`, at `at`, is an active requirement among `requirements`, those
    /// of the specification `cited`.
    fn requirement(
        &mut self,
        at: Pointer,
        requirements: &Requirements<'_>,
        cited: &str,
        requirement: &str,
    ) {
        let holder = &self.holder.id;
        match requirements.get(requirement) {
            None => {
                let message =
                    format!("{holder} cites {cited} {requirement}, which {cited} does not have.");
                self.broken(at, Reason::Missing, message, &[]);
            }
            Some(RequirementStatus::Removed) => {
                let message = format!("{holder} cites {cited} {requirement}, which is removed.");
                self.broken(at, Reason::State, message, &[]);
            }
            Some(RequirementStatus::Active) => {}
        }
    }

    /// Records a broken link at `at`: an error of the reason `reason`, or the warning
    /// `link_broken_terminal` when the artifact is in a terminal status (R3). `replacements`,
    /// those of a superseded target, are the values accepted at `at`, and the fix when there is
    /// exactly one.
    fn broken(&mut self, at: Pointer, reason: Reason, message: String, replacements: &[&str]) {
        let code = match (self.terminal, reason) {
            (true, _) => codes::LINK_BROKEN_TERMINAL,
            (false, Reason::Missing) => codes::LINK_MISSING,
            (false, _) => codes::LINK_SUPERSEDED,
        };
        let mut error = Error::new(code, message, at, reason);
        if !replacements.is_empty() {
            error = error.with_expected(replacements.iter().copied());
        }
        if let [replacement] = replacements {
            error = error.with_fix(*replacement);
        }
        let error = self.holder.of(error);
        if self.terminal {
            self.found.warnings.push(error);
        } else {
            self.found.errors.push(error);
        }
    }

    /// Checks that `member` of a non-terminal artifact equals the same member of the artifact
    /// its member `via` links to, when it resolves and passed its schema (R15).
    fn hierarchy(&mut self, member: &str, via: &str) {
        if self.terminal {
            return;
        }
        let parent = text(self.document, via).and_then(|id| self.find(id));
        let declared = text(self.document, member);
        let reached = parent
            .and_then(|p| p.read.as_ref())
            .and_then(|p| text(p, member));
        let (Some(declared), Some(reached), Some(parent)) = (declared, reached, parent) else {
            return;
        };
        if declared == reached {
            return;
        }
        let message = format!(
            "{} declares the {member} {declared}, but its {via} {} belongs to {reached}.",
            self.holder.id, parent.id
        );
        let root = Pointer::root();
        let error = Error::new(
            codes::HIERARCHY_MISMATCH,
            message,
            root.join(member),
            Reason::Conflict,
        )
        .with_pointers([root.join(via)])
        .with_expected([reached]);
        self.found.errors.push(self.holder.of(error));
    }
}

/// The warnings of the citations of ADRs in `related` of every living specification of `index`
/// with no error of its own, with no ADR of that number among `adrs`.
pub(super) fn citations(index: &[Indexed<'_>], adrs: &BTreeSet<u64>) -> Vec<Error> {
    let mut warnings = Vec::new();
    let specifications = index
        .iter()
        .filter(|a| a.kind == ArtifactKind::Specification && a.clean);
    for artifact in specifications {
        let Some(document) = &artifact.read else {
            continue;
        };
        let mut continued = false;
        for (i, entry) in texts(document, RELATED) {
            let (cited, run) = cited_adrs(entry, continued);
            continued = run;
            for number in cited {
                if number.parse::<u64>().is_ok_and(|n| adrs.contains(&n)) {
                    continue;
                }
                let message = format!(
                    "{} cites ADR {number}, which is not an ADR of the project.",
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

/// The numbers of ADRs that `entry` cites, as written, and whether it ends in a citation, so
/// that an entry that follows with a bare number cites an ADR too. `continued` says whether the
/// entry before ended in one. Words are separated by spaces, commas and semicolons; a
/// citation is the word `ADR`, then each word of ASCII digits that follows it.
fn cited_adrs(entry: &str, continued: bool) -> (Vec<&str>, bool) {
    const WORD: &str = "ADR";
    let separator = |c: char| c.is_whitespace() || c == ',' || c == ';';
    let mut cited = Vec::new();
    let mut citing = continued;
    for word in entry.split(separator).filter(|w| !w.is_empty()) {
        let word = word.trim_matches(|c| matches!(c, '(' | ')' | '[' | ']' | '.'));
        if word == WORD {
            citing = true;
        } else if citing && !word.is_empty() && word.bytes().all(|b| b.is_ascii_digit()) {
            cited.push(word);
        } else {
            citing = false;
        }
    }
    (cited, citing)
}

#[cfg(test)]
mod tests;
