//! The artifact model of SPEC-0001: the types of artifacts and inputs, the versions of their
//! schemas and the migrations between them, the codes of their errors, and the validation of
//! a set of them behind the command `validate`.
//!
//! The schemas are the snapshots under `schemas/artifacts/`, embedded in `wayloop-schemas` and
//! read with the interpreter of [`crate::schema`]. A document is checked in stages: its
//! `kind`, its `schema_version` (R16), the schema of that version (R1), the form of its `id`
//! (R14); the checks across documents are listed in `diagnostics.not_done` until they are built.

pub mod checks;
pub mod codes;
mod input;
pub mod kinds;
mod validate;

#[cfg(test)]
mod tests;

pub(crate) use validate::run;
#[cfg(test)]
pub(crate) use validate::{Validation, run_with, validate, validate_with};
