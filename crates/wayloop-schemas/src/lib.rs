//! Wayloop schemas: the Rust types of the contracts (ADR 0009, ADR 0028).
//!
//! The JSON schemas under `schemas/` are the only contract of the engine. This crate holds the
//! types that mirror them, generated from the schemas or checked against them, so that the
//! core, the runtime and every adapter read and write the same shapes.
//!
//! The crate is pure: it performs no I/O and holds no decision logic, because `wayloop-core`
//! is built on it. It depends on no other crate, checked by `scripts/check_core_io.py`; I/O of
//! the standard library in its source is refused by the Clippy lints forbidden below, with the
//! paths listed in its `clippy.toml`. The crate holds no type until the first command schema
//! is written.

// I/O is refused by the compiler: these lints read the clippy.toml of this crate, and forbid,
// unlike deny, cannot be lifted by an inner allow. `scripts/check_core_io.py` keeps them here.
#![forbid(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::dbg_macro,
    clippy::disallowed_methods,
    clippy::disallowed_types
)]
