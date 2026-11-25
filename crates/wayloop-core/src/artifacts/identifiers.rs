//! The identifier stage across the set (SPEC-0001 R2, R14; plan, stage 3).
//!
//! The form of each `id` is checked with the schema of its document, by the named formats of
//! the interpreter (`identifier_format`). This stage reads the index of the artifacts of a
//! known kind whose `id` raised no error, whatever their other errors, so that duplicates are
//! found in the same pass as every other error (ADR 0009 C3) and a broken identifier causes no
//! error here:
//!
//! - `identifier_duplicate`: two artifacts or more that carry one identifier are each refused
//!   with the reason `duplicate` at `/id`, and each error names the others in `related`.
//! - `identifier_location`: an artifact read from a file of a location whose name, or whose
//!   folder number for a living specification, names another identifier gives a warning with
//!   the reason `conflict` at `/id` and `fix` its identifier. The identity is the `id`; the
//!   name only helps a person find the file. A document sent directly gives none.
//!
//! The same index, and the map of [`carriers`] built from it once, is the one the links
//! resolve through (`links`, stage 4): an identifier is looked up there, never a path (ADR
//! 0008).

use core::cell::OnceCell;
use std::collections::BTreeMap;

use serde_json::{Map, Value};
use wayloop_schemas::artifact::ArtifactKind;
use wayloop_schemas::validate::Status;

use super::codes;
use super::duplicates::{self, shared_in};
use super::input::{Body, Entry, Place};
use super::locations::named_identifier;
use super::members::{self, ID, STATUS, SUPERSEDED_BY};
use crate::{Error, Pointer, Reason, Related, Source};

/// An artifact of the index: a document of a known kind whose `id` raised no error, by its
/// index in the input, with its kind, its identifier, its entry, the document read into its
/// contract type when it passed its schema (ADR 0030 G1), and whether it raised no error of its
/// own.
#[derive(Debug, Clone)]
pub(super) struct Indexed<'a> {
    pub(super) document: u64,
    pub(super) kind: ArtifactKind,
    pub(super) id: String,
    pub(super) entry: &'a Entry,
    pub(super) read: Option<Map<String, Value>>,
    /// No error of its own, of the schema or of the normalisation of its Markdown: only such an
    /// artifact has its links checked, while every artifact of the index is a target (decided by
    /// the maintainer on 2026-11-25).
    pub(super) clean: bool,
    /// The replacements the artifact offers, read once from its `superseded_by` by
    /// [`Indexed::replaced`].
    pub(super) offered: OnceCell<Vec<String>>,
}

/// The artifacts of the index by the identifier they carry, each list in the order of the
/// index: built once and read by the duplicates and the links.
pub(super) type Carriers<'i, 'a> = BTreeMap<&'i str, Vec<&'i Indexed<'a>>>;

/// The artifacts of `index` by the identifier they carry.
pub(super) fn carriers<'i, 'a>(index: &'i [Indexed<'a>]) -> Carriers<'i, 'a> {
    duplicates::carriers(index, |a| a.id.as_str())
}

/// The artifact of the set that carries `id`, the first of the index when several do: the one
/// target of every link and dependency that names `id`.
pub(super) fn first<'i, 'a>(carriers: &Carriers<'i, 'a>, id: &str) -> Option<&'i Indexed<'a>> {
    carriers.get(id)?.first().copied()
}

/// The pointer of the identifier of an artifact.
pub(super) fn id_pointer() -> Pointer {
    Pointer::root().join(ID)
}

impl Indexed<'_> {
    /// Where the identifier of the artifact is stored; `None` for a document sent directly.
    fn source(&self) -> Option<Source> {
        self.entry.source(&id_pointer())
    }

    /// `error`, of the artifact, with its document and the place where the value at its
    /// pointer is stored.
    pub(super) fn of(&self, error: Error) -> Error {
        self.entry.locate(error.with_document(self.document))
    }

    /// The document as read into its contract type, or as given when it did not pass its
    /// schema.
    fn document(&self) -> Option<&Map<String, Value>> {
        match (&self.read, &self.entry.body) {
            (Some(read), _) => Some(read),
            (None, Body::Document { document, .. }) => Some(document),
            (None, _) => None,
        }
    }

    /// The status of the artifact; `None` when it gives none that any type allows.
    pub(super) fn status(&self) -> Option<Status> {
        let status = self.document()?.get(STATUS)?.as_str()?;
        Status::from_id(status)
    }

    /// At most three distinct identifiers of the `superseded_by` of the artifact, other than its
    /// own, in their order, read once: enough to tell, for any holder, whether no replacement,
    /// one or several remain once the holder is left out.
    fn offered(&self) -> &[String] {
        self.offered.get_or_init(|| {
            let mut offered: Vec<String> = Vec::new();
            let listed = self
                .document()
                .into_iter()
                .flat_map(|d| members::texts(d, SUPERSEDED_BY).map(|(_, id)| id));
            for id in listed {
                if offered.len() == 3 {
                    break;
                }
                if id != self.id && !offered.iter().any(|o| o == id) {
                    offered.push(id.to_owned());
                }
            }
            offered
        })
    }

    /// `error`, of a link or a dependency of `holder` that names this artifact, superseded, with
    /// the replacements it offers (R4). The replacements never include the artifact itself,
    /// which is superseded, nor `holder`, which never names itself. No error copies a list that
    /// grows with the input (decided by the maintainer on 2026-11-25): with exactly one
    /// replacement, it is `expected` and `fix`; with several, `related` names where the list
    /// is, the `superseded_by` of this artifact; with none, the error has neither.
    pub(super) fn replaced(&self, holder: &str, error: Error) -> Error {
        let mut offered = self.offered().iter().filter(|id| *id != holder);
        match (offered.next(), offered.next()) {
            (None, _) => error,
            (Some(one), None) => error.with_expected([one.as_str()]).with_fix(one.as_str()),
            (Some(_), Some(_)) => {
                let at = Pointer::root().join(SUPERSEDED_BY);
                let source = self.entry.source(&at);
                error.with_related([Related::new(self.document, at, source)])
            }
        }
    }

    /// Whether the artifact is not changed again: `done` and `superseded` are terminal for every
    /// type, and a hand-over record, which has no status, is never changed (R3).
    pub(super) fn terminal(&self) -> bool {
        self.kind == ArtifactKind::Handover
            || matches!(self.status(), Some(Status::Done | Status::Superseded))
    }
}

/// The errors of identifiers carried by more than one artifact of `index`, whose map is
/// `carriers`, one per artifact, in the order of the index.
pub(super) fn duplicates<'i>(index: &'i [Indexed<'_>], carriers: &Carriers<'i, '_>) -> Vec<Error> {
    let related = |a: &Indexed<'_>| Related::new(a.document, id_pointer(), a.source());
    shared_in(index, carriers, |a| a.id.as_str(), related)
        .into_iter()
        .map(|(artifact, other)| {
            let message = format!(
                "The identifier {} is also carried by the document {} of the input; an \
                 identifier is never reused.",
                artifact.id,
                other.document()
            );
            let error = Error::new(
                codes::IDENTIFIER_DUPLICATE,
                message,
                id_pointer(),
                Reason::Duplicate,
            )
            .with_related([other]);
            artifact.of(error)
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
            .with_fix(artifact.id.as_str());
            Some(artifact.of(warning))
        })
        .collect()
}
