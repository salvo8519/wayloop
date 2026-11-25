//! The artifact model of SPEC-0001: the types of artifacts and inputs, the versions of their
//! schemas and the migrations between them, the codes of their errors, and the validation of
//! a set of them behind the command `validate`.
//!
//! The schemas are the snapshots under `schemas/artifacts/`, embedded in `wayloop-schemas` and
//! read with the interpreter of [`crate::schema`]. A document is checked in stages: its
//! `kind`, its `schema_version` (R16), the schema of that version (R1), the form of its `id`
//! (R14); then the identifiers (R14), the links between artifacts (R3, R4, R9, R15), the parts
//! of artifacts and the operations of change deltas (R18, R20), the titles of epics and tasks
//! (R7, R8), and the branch names and commit headers of pull requests and commits (R11) across
//! the set. A living specification sent as Markdown is first normalised
//! into its document model (R12). The other checks across documents are listed in
//! `diagnostics.not_done` until they are built.

pub mod checks;
pub mod codes;
mod duplicates;
mod identifiers;
mod input;
pub mod kinds;
mod links;
pub mod locations;
mod markdown;
mod members;
mod naming;
mod parts;
mod titles;
mod validate;

#[cfg(test)]
mod tests;

pub use parts::next_requirement;
pub(crate) use validate::run;
#[cfg(test)]
pub(crate) use validate::{Validation, run_with, validate, validate_with};
