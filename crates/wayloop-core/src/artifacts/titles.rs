//! The title stage across the set (SPEC-0001 R7, R8; plan, Naming rules).
//!
//! This stage reads the epics and tasks whose `title` is a string that raised no error, whatever
//! their other errors, so that the titles are checked in the same pass as every other error
//! (ADR 0009 C3):
//!
//! - `title_format`: the form of each title, by the grammar of [`crate::naming::titles`].
//! - `title_duplicate`: two titles or more equal after normalisation, in any status except
//!   `superseded`, epics and tasks compared with each other, are each refused with the reason
//!   `duplicate` at `/title`, and each error names one other in `related`: each later carrier
//!   names the first, the first names the second (decided by the maintainer on 2026-11-25).

use serde_json::Value;
use wayloop_schemas::artifact::{ArtifactKind, PlanItemStatus};
use wayloop_schemas::contract::rules::{TITLE, TITLE_MAX_CHARS};
use wayloop_schemas::validate::DocumentSummary;
use wayloop_schemas::value::PolicyName;

use super::codes;
use super::duplicates::shared;
use super::input::{Body, Entry};
use crate::naming::titles::{self, Prefixes};
use crate::{Error, Reason, Related};

/// The kinds whose titles are checked (R7, R8).
const TITLED: [ArtifactKind; 2] = [ArtifactKind::Epic, ArtifactKind::Task];

/// An epic or a task of the set, by its index in the input, with its title and its entry.
#[derive(Debug, Clone)]
pub(super) struct Titled<'a> {
    document: u64,
    title: &'a str,
    /// The normalised title (R8).
    key: String,
    superseded: bool,
    entry: &'a Entry,
}

impl Titled<'_> {
    /// `error`, at `/title` of the artifact, with its document and the place it is stored.
    fn of(&self, error: Error) -> Error {
        self.entry.locate(error.with_document(self.document))
    }

    /// The artifact as another error names it.
    fn related(&self) -> Related {
        Related::new(
            self.document,
            titles::pointer(),
            self.entry.source(&titles::pointer()),
        )
    }
}

/// The epic or task of `entry`, the document `document` summarised by `summary`, when its title
/// is a string of at most `TITLE_MAX_CHARS` characters; `None` for any other document. A longer
/// title is an error of the schema, or of a document whose schema did not run (an unknown
/// version), and is never read here, so the cost of the rules stays bounded.
pub(super) fn titled<'a>(
    document: u64,
    entry: &'a Entry,
    summary: &DocumentSummary,
) -> Option<Titled<'a>> {
    if !is_titled(summary) {
        return None;
    }
    let Body::Document { document: body, .. } = &entry.body else {
        return None;
    };
    let title = body.get(TITLE).and_then(Value::as_str)?;
    if title.chars().nth(TITLE_MAX_CHARS).is_some() {
        return None;
    }
    let superseded = summary.status.as_deref() == Some(PlanItemStatus::Superseded.id());
    Some(Titled {
        document,
        key: titles::key(title),
        title,
        superseded,
        entry,
    })
}

/// Whether the set summarised by `documents` holds an epic or a task, whose checks read the
/// policy (R21).
pub(super) fn reads_policy(documents: &[DocumentSummary]) -> bool {
    documents.iter().any(is_titled)
}

/// Whether the document summarised by `summary` is an epic or a task.
fn is_titled(summary: &DocumentSummary) -> bool {
    TITLED.iter().any(|kind| summary.kind == Some(*kind))
}

/// Every error of the titles of `index`, whose type prefixes are `types`: the errors of form in
/// the order of the index, then the duplicates in the same order.
pub(super) fn errors(index: &[Titled<'_>], types: &[PolicyName]) -> Vec<Error> {
    let prefixes = Prefixes::new(types);
    let mut errors: Vec<Error> = index
        .iter()
        .flat_map(|t| {
            titles::format(t.title, &prefixes)
                .into_iter()
                .map(|e| t.of(e))
        })
        .collect();
    let compared: Vec<&Titled<'_>> = index.iter().filter(|t| !t.superseded).collect();
    let duplicates = shared(&compared, |t| t.key.as_str(), |t| t.related());
    errors.extend(duplicates.into_iter().map(|(artifact, other)| {
        let message = format!(
            "The title is equal to the title of the document {} of the input after \
             normalisation; two epics or tasks never share a title.",
            other.document()
        );
        let error = Error::new(
            codes::TITLE_DUPLICATE,
            message,
            titles::pointer(),
            Reason::Duplicate,
        )
        .with_related([other]);
        artifact.of(error)
    }));
    errors
}
