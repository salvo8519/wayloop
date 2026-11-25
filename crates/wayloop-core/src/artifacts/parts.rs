//! The parts of artifacts and the operations of change deltas (SPEC-0001 R18, R20; plan, stage
//! 5), and the identifier of the next requirement of a living specification, which the merge of
//! a change delta gives (SPEC-0009).
//!
//! The parts of an artifact are read from its contract form, so only an artifact that passed its
//! schema has them checked. The rules inside one document hold whatever the other documents:
//!
//! - A part identifier given twice in its list, the decisions and the questions of a council
//!   report, the requirements, the acceptance criteria and the open questions of a living
//!   specification, the acceptance criteria of a user story, is `part_invalid` with the reason
//!   `duplicate` at the `id` of each carrier after the first, with that of the first and its
//!   own in `pointers` (R18).
//! - A dissent that names a decision the report does not have is `part_invalid` with the reason
//!   `missing` at its `decision`, with `/decisions` in `pointers`; a recommendation or an
//!   answer that is not among the options of its question is `part_invalid` with the reason
//!   `enum`, each option once in `expected` and the options in `pointers` (R18).
//! - Two operations of a change delta on one requirement are `delta_conflict` with the reason
//!   `conflict` at each operation after the first, with the first and itself in `pointers`
//!   (R20). A member that the `op` of an operation refuses, `requirement` on `added`, `text` on
//!   `removed` and `renamed`, `name` on `removed`, is `artifact_schema` with the reason
//!   `unknown_field` at that member and the `op` in `pointers` (decided by the maintainer on
//!   2026-11-24); a requirement refused there names no requirement, so it is no conflict.
//! - A task or an acceptance criterion of a living specification whose `verification` holds
//!   `none` beside a technique is `artifact_schema` with the reason `enum` at each `none`, the
//!   techniques in `expected` and the first of them in `pointers` (decided by the maintainer on
//!   2026-11-23 for tasks and on 2026-11-25 for acceptance criteria).
//!
//! The dependencies of a task read other documents, so only a task with no error of its own,
//! in a status that is not terminal, has them checked, and only its dependencies are edges of
//! the graph; every task of the index is a target, as long as its `id` raised no error (decided
//! by the maintainer on 2026-11-25). A dependency on the task itself, with `/id` in `pointers`,
//! on a superseded task, with its replacements as for a link (R4, [`Indexed::replaced`]), or on
//! a task that leads back to it, which closes a cycle, is `part_invalid` with the reason
//! `conflict` at the dependency (R18). Every dependency on a cycle is refused, whichever closed
//! it, with no `related`. The cycles are found once over the whole set, without recursion, in
//! time linear in the tasks and their dependencies.
//!
//! Only a dependency on a superseded task with several replacements names another document in
//! `related`, the list of the target. The errors of a document come in the parts stage, after
//! the links, also those of the code `artifact_schema`; within it in the byte order of its
//! members, the elements of a list by index, at one element its own error before those of its
//! members.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};
use wayloop_schemas::Format;
use wayloop_schemas::artifact::{ArtifactKind, OperationKind, Verification};
use wayloop_schemas::validate::Status;
use wayloop_schemas::value::RequirementId;

use super::codes;
use super::duplicates;
use super::identifiers::{Carriers, Indexed, first, id_pointer};
use super::members::{
    ACCEPTANCE_CRITERIA, ANSWER, DECISION, DECISIONS, DEPENDS_ON, DISSENT, ID, NAME, OP,
    OPEN_QUESTIONS, OPERATIONS, OPTIONS, QUESTIONS, RECOMMENDATION, REQUIREMENT, REQUIREMENTS,
    TEXT, VERIFICATION, elements, texts, texts_in,
};
use crate::schema::format;
use crate::{Error, Pointer, Reason};

/// The prefix of a requirement identifier, `R`, from the one table of formats.
const REQUIREMENT_PREFIX: &str = match format::part_prefix(Format::RequirementId) {
    Some(prefix) => prefix,
    None => "",
};
// The format of a requirement identifier is a part with a prefix, checked when the crate builds.
const _: () = assert!(!REQUIREMENT_PREFIX.is_empty());

/// The identifier that the next requirement added to a living specification gets when a change
/// delta is merged (SPEC-0001 R20; SPEC-0009): `R<k>`, with `k` one more than the highest
/// number among `requirements`, the identifiers of every requirement of the specification,
/// removed ones included, so that an identifier is never used again; `R1` when there is none.
/// Two deltas open at the same time therefore never claim the same number, since each gets its
/// identifier only when it is merged.
///
/// The numbers are compared and incremented as decimal text, so no number is too large. An
/// identifier that is not in the form `R<k>`, `k` from 1 without a leading zero, is not
/// counted: the schema of a living specification refuses it, so a specification that passed
/// validation has none.
#[must_use]
pub fn next_requirement<'r>(
    requirements: impl IntoIterator<Item = &'r RequirementId>,
) -> RequirementId {
    let prefix = REQUIREMENT_PREFIX;
    let highest = requirements
        .into_iter()
        .map(RequirementId::as_str)
        .filter(|id| format::accepts(Format::RequirementId, id))
        .filter_map(|id| id.strip_prefix(prefix))
        .max_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)))
        .unwrap_or("0");
    RequirementId::new(format!("{prefix}{}", incremented(highest)))
}

/// The decimal number `digits`, ASCII digits without a leading zero or `0`, plus one.
fn incremented(digits: &str) -> String {
    let mut bytes: Vec<u8> = digits.bytes().collect();
    let mut carry = true;
    for byte in bytes.iter_mut().rev() {
        if *byte == b'9' {
            *byte = b'0';
        } else {
            *byte += 1;
            carry = false;
            break;
        }
    }
    if carry {
        bytes.insert(0, b'1');
    }
    bytes.into_iter().map(char::from).collect()
}

/// The errors of the parts of every artifact of `index` that passed its schema, in the order of
/// the index, then of the members of its artifact; `carriers` is the map of the identifiers of
/// `index`.
pub(super) fn check(index: &[Indexed<'_>], carriers: &Carriers<'_, '_>) -> Vec<Error> {
    let cyclic = cyclic(index, carriers);
    let mut errors = Vec::new();
    for artifact in index {
        let Some(document) = &artifact.read else {
            continue;
        };
        let mut parts = Parts {
            artifact,
            document,
            errors: &mut errors,
        };
        match artifact.kind {
            ArtifactKind::CouncilReport => parts.council_report(),
            ArtifactKind::Specification => {
                parts.criteria();
                parts.unique(OPEN_QUESTIONS);
                parts.unique(REQUIREMENTS);
            }
            ArtifactKind::Story => parts.unique(ACCEPTANCE_CRITERIA),
            ArtifactKind::Change => parts.operations(),
            ArtifactKind::Task => {
                parts.dependencies(carriers, &cyclic);
                parts.verification(&Pointer::root(), document);
            }
            ArtifactKind::Brief
            | ArtifactKind::Gate
            | ArtifactKind::Milestone
            | ArtifactKind::Epic
            | ArtifactKind::Handover => {}
        }
    }
    errors
}

/// Whether the dependencies of `task` are checked: it has no error of its own and is not in a
/// terminal status, since a task done or superseded is not changed again (R3).
fn checked(task: &Indexed<'_>) -> bool {
    task.clean && !task.terminal()
}

/// The elements of `items`, each an index and a key, whose key an earlier element carries, by
/// their index, each with the index of the first element that carries it and the key.
fn repeated<'k>(items: &[(usize, &'k str)]) -> BTreeMap<usize, (usize, &'k str)> {
    let mut later = BTreeMap::new();
    for carriers in duplicates::carriers(items, |item| item.1).values() {
        if let [first, others @ ..] = carriers.as_slice() {
            later.extend(others.iter().map(|other| (other.0, (first.0, other.1))));
        }
    }
    later
}

/// The identifiers of the parts in the list `list` of `document`, each with its index.
fn identifiers<'d>(document: &'d Map<String, Value>, list: &str) -> Vec<(usize, &'d str)> {
    elements(document, list)
        .filter_map(|(i, part)| Some((i, part.get(ID)?.as_str()?)))
        .collect()
}

/// The parts of one artifact, read from `document`, its contract form.
struct Parts<'s, 'i, 'a> {
    artifact: &'i Indexed<'a>,
    document: &'i Map<String, Value>,
    errors: &'s mut Vec<Error>,
}

impl<'i> Parts<'_, 'i, '_> {
    /// Records an error of the artifact.
    fn push(&mut self, error: Error) {
        self.errors.push(self.artifact.of(error));
    }

    /// Checks that no part of the list `list` repeats the identifier of an earlier one.
    fn unique(&mut self, list: &str) {
        for (later, (first, id)) in repeated(&identifiers(self.document, list)) {
            self.duplicate(list, later, first, id);
        }
    }

    /// Records the identifier `id` of the part `later` of `list`, already carried by `first`.
    fn duplicate(&mut self, list: &str, later: usize, first: usize, id: &str) {
        let at = |i: usize| Pointer::root().join(list).join_index(i).join(ID);
        let message = format!(
            "{} gives the part {id} twice in {list}; a part identifier is unique within its \
             list.",
            self.artifact.id
        );
        let error = Error::new(codes::PART_INVALID, message, at(later), Reason::Duplicate)
            .with_pointers([at(first), at(later)]);
        self.push(error);
    }

    /// The decisions, the dissent and the questions of a council report, in this order.
    fn council_report(&mut self) {
        let document: &'i Map<String, Value> = self.document;
        self.unique(DECISIONS);
        let decisions: BTreeSet<&str> = identifiers(document, DECISIONS)
            .into_iter()
            .map(|(_, id)| id)
            .collect();
        let root = Pointer::root();
        for (i, dissent) in elements(document, DISSENT) {
            let Some(named) = dissent.get(DECISION).and_then(Value::as_str) else {
                continue;
            };
            if decisions.contains(named) {
                continue;
            }
            let message = format!(
                "{} records a dissent on {named}, which is not a decision of the report.",
                self.artifact.id
            );
            let at = root.join(DISSENT).join_index(i).join(DECISION);
            let error = Error::new(codes::PART_INVALID, message, at, Reason::Missing)
                .with_pointers([root.join(DECISIONS)]);
            self.push(error);
        }
        let repeated = repeated(&identifiers(document, QUESTIONS));
        for (i, question) in elements(document, QUESTIONS) {
            self.option(i, question, ANSWER);
            if let Some((first, id)) = repeated.get(&i) {
                self.duplicate(QUESTIONS, i, *first, id);
            }
            self.option(i, question, RECOMMENDATION);
        }
    }

    /// Checks that `member` of the question `i`, when given, is one of its options.
    fn option(&mut self, i: usize, question: &Value, member: &str) {
        let Some(given) = question.get(member).and_then(Value::as_str) else {
            return;
        };
        let mut seen = BTreeSet::new();
        let options: Vec<&str> = texts_in(question, OPTIONS)
            .map(|(_, option)| option)
            .filter(|option| seen.insert(*option))
            .collect();
        if seen.contains(given) {
            return;
        }
        let at = Pointer::root().join(QUESTIONS).join_index(i);
        let message = format!(
            "The {member} {given:?} of {} is not one of the options of its question.",
            self.artifact.id
        );
        let error = Error::new(codes::PART_INVALID, message, at.join(member), Reason::Enum)
            .with_expected(options)
            .with_pointers([at.join(OPTIONS)]);
        self.push(error);
    }

    /// The operations of a change delta: at most one per requirement (R20), and no member its
    /// `op` refuses.
    fn operations(&mut self) {
        let document: &'i Map<String, Value> = self.document;
        let on_a_requirement: Vec<(usize, &str)> = elements(document, OPERATIONS)
            .filter_map(|(i, operation)| {
                let op = operation_kind(operation)?;
                let requirement = operation.get(REQUIREMENT)?.as_str()?;
                (!refused(op, REQUIREMENT)).then_some((i, requirement))
            })
            .collect();
        let conflicts = repeated(&on_a_requirement);
        for (i, operation) in elements(document, OPERATIONS) {
            let at = Pointer::root().join(OPERATIONS).join_index(i);
            if let Some((first, requirement)) = conflicts.get(&i) {
                let message = format!(
                    "{} has more than one operation on {requirement}; a change of its text and \
                     its name is one operation modified.",
                    self.artifact.id
                );
                let first = Pointer::root().join(OPERATIONS).join_index(*first);
                let error =
                    Error::new(codes::DELTA_CONFLICT, message, at.clone(), Reason::Conflict)
                        .with_pointers([first, at.clone()]);
                self.push(error);
            }
            let Some(op) = operation_kind(operation) else {
                continue;
            };
            for member in OPERATION_MEMBERS {
                if refused(op, member) && operation.get(member).is_some() {
                    let message = format!("The member {member} is refused on an operation {op}.");
                    let error = Error::new(
                        codes::ARTIFACT_SCHEMA,
                        message,
                        at.join(member),
                        Reason::UnknownField,
                    )
                    .with_pointers([at.join(OP)]);
                    self.push(error);
                }
            }
        }
    }

    /// The dependencies of a task, when they are checked: none on the task itself, on a
    /// superseded task or on a cycle, the dependencies of `cyclic`.
    fn dependencies(&mut self, carriers: &Carriers<'_, '_>, cyclic: &BTreeSet<(u64, usize)>) {
        if !checked(self.artifact) {
            return;
        }
        let artifact: &'i Indexed<'_> = self.artifact;
        let own = artifact.id.as_str();
        for (i, target) in texts(self.document, DEPENDS_ON) {
            let at = Pointer::root().join(DEPENDS_ON).join_index(i);
            let error = if target == own {
                let message = format!("{own} depends on itself.");
                conflict(message, at).with_pointers([id_pointer()])
            } else if let Some(found) =
                first(carriers, target).filter(|found| found.status() == Some(Status::Superseded))
            {
                let message = format!("{own} depends on {target}, which is superseded.");
                found.replaced(artifact, conflict(message, at))
            } else if cyclic.contains(&(artifact.document, i)) {
                let message =
                    format!("{own} depends on {target}, which leads back to {own}: a cycle.");
                conflict(message, at)
            } else {
                continue;
            };
            self.push(error);
        }
    }

    /// The acceptance criteria of a living specification, each in the byte order of its
    /// members: its identifier unique in the list, then its verification.
    fn criteria(&mut self) {
        let document: &'i Map<String, Value> = self.document;
        let repeated = repeated(&identifiers(document, ACCEPTANCE_CRITERIA));
        for (i, criterion) in elements(document, ACCEPTANCE_CRITERIA) {
            if let Some((first, id)) = repeated.get(&i) {
                self.duplicate(ACCEPTANCE_CRITERIA, i, *first, id);
            }
            if let Some(members) = criterion.as_object() {
                let at = Pointer::root().join(ACCEPTANCE_CRITERIA).join_index(i);
                self.verification(&at, members);
            }
        }
    }

    /// The verification of a task or of an acceptance criterion, the object `holder` at `at`:
    /// `none` is never given beside a technique (decided by the maintainer on 2026-11-23 and,
    /// for acceptance criteria, on 2026-11-25). The order of the values and a repeated value
    /// carry no meaning.
    fn verification(&mut self, at: &Pointer, holder: &Map<String, Value>) {
        let none = Verification::None.id();
        let values: Vec<(usize, &str)> = texts(holder, VERIFICATION).collect();
        let Some(technique) = values.iter().find(|(_, v)| *v != none).map(|v| v.0) else {
            return;
        };
        let techniques = Verification::ALL
            .into_iter()
            .filter(|v| *v != Verification::None)
            .map(Verification::id);
        let value = |i: usize| at.join(VERIFICATION).join_index(i);
        for (i, _) in values.iter().filter(|(_, v)| *v == none) {
            let message = format!(
                "{} is verified by {none} and by a technique; {none} states that no technique \
                 verifies it.",
                self.artifact.id
            );
            let error = Error::new(codes::ARTIFACT_SCHEMA, message, value(*i), Reason::Enum)
                .with_expected(techniques.clone())
                .with_pointers([value(technique)]);
            self.push(error);
        }
    }
}

/// An error `part_invalid` with the reason `conflict` at `at`.
fn conflict(message: String, at: Pointer) -> Error {
    Error::new(codes::PART_INVALID, message, at, Reason::Conflict)
}

/// The members of an operation that its `op` may refuse, in the byte order of their keys.
const OPERATION_MEMBERS: [&str; 3] = [NAME, REQUIREMENT, TEXT];

/// What `operation` does, when its `op` is known.
fn operation_kind(operation: &Value) -> Option<OperationKind> {
    OperationKind::from_id(operation.get(OP)?.as_str()?)
}

/// Whether an operation `op` refuses `member` (Change delta, decided by the maintainer on
/// 2026-11-24): `requirement` on an operation that names no existing requirement, `text` on one
/// that gives no text, `name` on `removed`.
fn refused(op: OperationKind, member: &str) -> bool {
    match member {
        REQUIREMENT => !OperationKind::ON_A_REQUIREMENT.contains(&op),
        TEXT => !OperationKind::WITH_TEXT.contains(&op),
        NAME => OperationKind::WITHOUT_NAME.contains(&op),
        _ => false,
    }
}

/// The dependencies that lie on a cycle, by the document of their task and their index in
/// `depends_on`: those between two tasks of one strongly connected component of the graph of
/// the checked dependencies. A dependency of a task on itself is refused apart.
fn cyclic(index: &[Indexed<'_>], carriers: &Carriers<'_, '_>) -> BTreeSet<(u64, usize)> {
    let tasks: Vec<&Indexed<'_>> = index
        .iter()
        .filter(|a| a.kind == ArtifactKind::Task)
        .collect();
    let node: BTreeMap<u64, usize> = tasks
        .iter()
        .enumerate()
        .map(|(n, task)| (task.document, n))
        .collect();
    // Each edge with the dependency it comes from.
    let mut edges: Vec<(usize, usize, usize)> = Vec::new();
    let mut successors: Vec<Vec<usize>> = vec![Vec::new(); tasks.len()];
    for (n, task) in tasks.iter().enumerate() {
        let (true, Some(document)) = (checked(task), &task.read) else {
            continue;
        };
        for (i, target) in texts(document, DEPENDS_ON) {
            let found = first(carriers, target);
            let Some(m) = found.and_then(|f| node.get(&f.document)).copied() else {
                continue;
            };
            if m != n {
                edges.push((n, i, m));
                if let Some(list) = successors.get_mut(n) {
                    list.push(m);
                }
            }
        }
    }
    let component = components(&successors);
    edges
        .into_iter()
        .filter(|(n, _, m)| component.get(*n) == component.get(*m))
        .filter_map(|(n, i, _)| Some((tasks.get(n)?.document, i)))
        .collect()
}

/// The strongly connected component of each node of the graph `successors`, the successors of
/// each node by its index, named by one of its nodes: the algorithm of Tarjan, with an explicit
/// stack of frames in place of recursion, so that a long path cannot exhaust the stack of the
/// thread. Each node and each edge is visited once.
fn components(successors: &[Vec<usize>]) -> Vec<usize> {
    let mut tarjan = Tarjan::new(successors.len());
    for root in 0..successors.len() {
        if tarjan.order(root).is_none() {
            tarjan.visit(successors, root);
        }
    }
    tarjan.component
}

/// The state of the algorithm of Tarjan over a graph of `count` nodes. Every index it is given
/// is below `count`; an index out of range is read as unvisited and never written.
struct Tarjan {
    /// The order in which each node was first visited; `None` before.
    order: Vec<Option<usize>>,
    /// The lowest order reached from each node through the nodes on the stack.
    low: Vec<usize>,
    /// Whether each node is on the stack of the component being built.
    on_stack: Vec<bool>,
    /// The nodes of the components not closed yet.
    stack: Vec<usize>,
    /// The component of each node, named by its first node visited.
    component: Vec<usize>,
    /// The order the next node visited gets.
    visited: usize,
}

impl Tarjan {
    fn new(count: usize) -> Self {
        Self {
            order: vec![None; count],
            low: vec![0; count],
            on_stack: vec![false; count],
            stack: Vec::new(),
            component: (0..count).collect(),
            visited: 0,
        }
    }

    fn order(&self, node: usize) -> Option<usize> {
        self.order.get(node).copied().flatten()
    }

    fn low(&self, node: usize) -> usize {
        self.low.get(node).copied().unwrap_or_default()
    }

    /// Lowers the lowest order reached from `node` to `reached`, when it is lower.
    fn lower(&mut self, node: usize, reached: usize) {
        if let Some(slot) = self.low.get_mut(node) {
            *slot = (*slot).min(reached);
        }
    }

    /// Gives `node` the next order of visit and puts it on the stack.
    fn enter(&mut self, node: usize) {
        if let Some(slot) = self.order.get_mut(node) {
            *slot = Some(self.visited);
        }
        if let Some(slot) = self.low.get_mut(node) {
            *slot = self.visited;
        }
        if let Some(flag) = self.on_stack.get_mut(node) {
            *flag = true;
        }
        self.stack.push(node);
        self.visited += 1;
    }

    /// Closes the component whose first node is `node`: every node above it on the stack.
    fn close(&mut self, node: usize) {
        while let Some(member) = self.stack.pop() {
            if let Some(flag) = self.on_stack.get_mut(member) {
                *flag = false;
            }
            if let Some(named) = self.component.get_mut(member) {
                *named = node;
            }
            if member == node {
                break;
            }
        }
    }

    /// Visits every node reached from `root`, unvisited, with one frame per node on the path:
    /// the node and the position of its next successor.
    fn visit(&mut self, successors: &[Vec<usize>], root: usize) {
        let mut frames: Vec<(usize, usize)> = vec![(root, 0)];
        self.enter(root);
        while let Some(frame) = frames.last_mut() {
            let (node, position) = *frame;
            if let Some(successor) = successors.get(node).and_then(|s| s.get(position)).copied() {
                frame.1 += 1;
                match self.order(successor) {
                    None => {
                        frames.push((successor, 0));
                        self.enter(successor);
                    }
                    Some(seen) if self.on_stack.get(successor).copied().unwrap_or_default() => {
                        self.lower(node, seen);
                    }
                    Some(_) => {}
                }
                continue;
            }
            frames.pop();
            let reached = self.low(node);
            if let Some((parent, _)) = frames.last() {
                self.lower(*parent, reached);
            }
            if self.order(node) == Some(reached) {
                self.close(node);
            }
        }
    }
}

#[cfg(test)]
mod tests;
