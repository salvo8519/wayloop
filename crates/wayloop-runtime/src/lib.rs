//! Wayloop runtime: executes the work that the core decides (ADR 0006).
//!
//! The core is synchronous and pure: given a state and an event, it returns the next state and
//! the work to do. This crate holds everything around it that waits on the outside world: the
//! queue of ready work derived from the state, the workers that run it in parallel up to a
//! configured limit, each issue in its own git worktree, the grouped tracker calls, and the
//! single writer that validates every result and appends it to the ledger in order.
//!
//! The runtime performs I/O and decides nothing. A choice it needs, such as which work is
//! ready or whether a result is valid, is asked of `wayloop-core`.
//!
//! Dependency direction: the runtime may depend on `wayloop-core` and `wayloop-schemas`;
//! neither of them may depend on the runtime. `scripts/check_core_io.py` holds the pure crates
//! to their reviewed allowlists of dependencies, which enforces that direction. The crate holds
//! no code until the first component of the runtime is specified.

#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "the runtime performs I/O: the lists of clippy.toml bind the pure crates only"
)]
