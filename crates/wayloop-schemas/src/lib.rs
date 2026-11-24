//! Wayloop schemas: the Rust types of the contracts (ADR 0009, ADR 0028).
//!
//! The JSON schemas under `schemas/` are the contract of the engine. They are generated from
//! the types of this crate and committed as snapshots (ADR 0030), so that the core, the runtime
//! and every adapter read and write the same shapes.
//!
//! The crate is pure: it performs no I/O and holds no decision logic, because `wayloop-core`
//! is built on it. It depends only on the crates of its reviewed allowlist, checked by
//! `scripts/check_core_io.py`; I/O of the standard library in its source is refused by the
//! Clippy lints forbidden below, with the paths listed in the `clippy.toml` of the workspace.
//! It holds the error report of ADR 0009 C3 and the envelope of C1, generic over the data of a
//! command, with the exit codes of C7; the artifacts of SPEC-0001 built on one common type
//! ([`artifact`]), with the steps of the issue cycle that the hand-over records ([`IssueStep`]);
//! the inputs pull request and commit ([`input`]); the input and the data of the command
//! `validate` ([`validate`]); and the value
//! forms they share, one type each ([`value`]), with numbers read by value ([`number`]). Every
//! contract type derives its JSON Schema; [`contract`] generates it inside the subset that the
//! core interprets, with the named formats of [`Format`] (ADR 0030). Closed enums and static
//! identifiers are written once with the macros of [`ident`], text and number forms with the
//! macros of [`value`].

// I/O is refused by the compiler: these lints read the clippy.toml of the workspace, and
// forbid, unlike deny, cannot be lifted by an inner allow. `scripts/check_core_io.py` keeps
// them here.
#![forbid(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::dbg_macro,
    clippy::disallowed_methods,
    clippy::disallowed_types
)]

pub mod artifact;
pub mod contract;
mod cycle;
mod envelope;
mod error;
#[cfg(any(test, feature = "examples"))]
pub mod examples;
#[cfg(test)]
mod fixtures;
mod format;
pub mod ident;
pub mod input;
pub mod number;
pub mod validate;
pub mod value;

pub use cycle::IssueStep;
pub use envelope::{
    Check, DefaultApplied, DefaultReason, Diagnostics, ENVELOPE_VERSION, Envelope, ExitCode,
    INPUT_UNREADABLE, INTERNAL_ERROR, InputHash, Meta, NotDone, NotDoneReason, Outcome, USAGE,
};
pub use error::{Code, Error, ErrorReport, Pointer, Reason, Source};
pub use format::Format;
