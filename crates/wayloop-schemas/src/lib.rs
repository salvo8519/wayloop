//! Wayloop schemas: the Rust types of the contracts (ADR 0009, ADR 0028).
//!
//! The JSON schemas under `schemas/` are the only contract of the engine. This crate holds the
//! types that mirror them, generated from the schemas or checked against them, so that the
//! core, the runtime and every adapter read and write the same shapes.
//!
//! The crate is pure: it performs no I/O and holds no decision logic, because `wayloop-core`
//! is built on it. `scripts/check_core_io.py` refuses an I/O crate among its dependencies and
//! I/O of the standard library in its source. The crate holds no type until the first command
//! schema is written.
