//! The identifier stage across the set (SPEC-0001 R2, R14; plan, stage 3).
//!
//! The form of each `id` is checked with the schema of its document, by the named formats of
//! the interpreter (`identifier_format`). This stage reads the index of the artifacts that
//! passed the stages of their own document, so that a broken document causes no error here:
//!
//! - `identifier_duplicate`: two artifacts or more that carry one identifier are each refused
//!   with the reason `duplicate` at `/id`, and each error names the others in `related`.
//! - `identifier_location`: an artifact read from a file of a location whose name, or whose
//!   folder number for a living specification, names another identifier gives a warning with
//!   the reason `conflict` at `/id` and `fix` its identifier. The identity is the `id`; the
//!   name only helps a person find the file. A document sent directly gives none.

use std::collections::BTreeMap;

use super::codes;
use super::input::{Entry, Place};
use super::locations::named_identifier;
use crate::{Error, Pointer, Reason, Related};

/// An artifact of the index: a document that passed the stages of its own, by its index in
/// the input, with its identifier and its entry.
#[derive(Debug, Clone, Copy)]
pub(super) struct Indexed<'a> {
    pub(super) document: u64,
    pub(super) id: &'a str,
    pub(super) entry: &'a Entry,
}

/// The pointer of the identifier of an artifact.
fn id_pointer() -> Pointer {
    Pointer::root().join("id")
}

/// The error or warning `error` of the artifact `artifact`, at `/id`, with its source.
fn of(artifact: &Indexed<'_>, error: Error) -> Error {
    let error = error.with_document(artifact.document);
    match artifact.entry.source(&id_pointer()) {
        Some(source) => error.with_location(source),
        None => error,
    }
}

/// The errors of identifiers carried by more than one artifact of `index`, one per artifact,
/// in the order of the index.
pub(super) fn duplicates(index: &[Indexed<'_>]) -> Vec<Error> {
    let mut carriers: BTreeMap<&str, Vec<&Indexed<'_>>> = BTreeMap::new();
    for artifact in index {
        carriers.entry(artifact.id).or_default().push(artifact);
    }
    index
        .iter()
        .filter_map(|artifact| {
            let others: Vec<Related> = carriers
                .get(artifact.id)?
                .iter()
                .filter(|other| other.document != artifact.document)
                .map(|other| {
                    let source = other.entry.source(&id_pointer());
                    Related::new(other.document, id_pointer(), source)
                })
                .collect();
            if others.is_empty() {
                return None;
            }
            let listed: Vec<String> = others.iter().map(|o| o.document().to_string()).collect();
            let documents = if listed.len() == 1 {
                "document"
            } else {
                "documents"
            };
            let message = format!(
                "The identifier {} is also carried by the {documents} {} of the input; an \
                 identifier is never reused.",
                artifact.id,
                listed.join(", ")
            );
            let error = Error::new(
                codes::IDENTIFIER_DUPLICATE,
                message,
                id_pointer(),
                Reason::Duplicate,
            )
            .with_related(others);
            Some(of(artifact, error))
        })
        .collect()
}

/// The warnings of the artifacts of `index` read from a file whose path names another
/// identifier, in the order of the index.
pub(super) fn locations(index: &[Indexed<'_>]) -> Vec<Error> {
    index
        .iter()
        .filter_map(|artifact| {
            let Some(Place::File(file)) = &artifact.entry.origin else {
                return None;
            };
            let named = named_identifier(file.as_str())?;
            if named == artifact.id {
                return None;
            }
            let message = format!(
                "The path {file} names {named}, not the identifier {}; rename the file or its \
                 folder after the identifier.",
                artifact.id
            );
            let warning = Error::new(
                codes::IDENTIFIER_LOCATION,
                message,
                id_pointer(),
                Reason::Conflict,
            )
            .with_fix(artifact.id);
            Some(of(artifact, warning))
        })
        .collect()
}
