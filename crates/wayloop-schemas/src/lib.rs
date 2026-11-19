//! Wayloop schemas: the Rust types of the contracts (ADR 0009, ADR 0028).
//!
//! The JSON schemas under `schemas/` are the only contract of the engine. This crate holds the
//! types that mirror them, generated from the schemas or checked against them, so that the
//! core, the runtime and every adapter read and write the same shapes.
//!
//! The crate is pure: it performs no I/O and holds no decision logic, because `wayloop-core`
//! is built on it. It depends only on the crates of its reviewed allowlist, checked by
//! `scripts/check_core_io.py`; I/O of the standard library in its source is refused by the
//! Clippy lints forbidden below, with the paths listed in its `clippy.toml`. It holds the error
//! report of ADR 0009 C3 and the envelope of C1 with the exit codes of C7; the types of the
//! command schemas come with the first command.

// I/O is refused by the compiler: these lints read the clippy.toml of this crate, and forbid,
// unlike deny, cannot be lifted by an inner allow. `scripts/check_core_io.py` keeps them here.
#![forbid(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::dbg_macro,
    clippy::disallowed_methods,
    clippy::disallowed_types
)]

mod envelope;
mod error;

pub use envelope::{
    DefaultApplied, Diagnostics, ENVELOPE_VERSION, Envelope, ExitCode, InputHash, Meta, Outcome,
};
pub use error::{Code, Error, ErrorReport, Pointer, Reason, Source};
