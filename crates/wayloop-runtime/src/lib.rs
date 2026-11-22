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
//! to their reviewed allowlists of dependencies, which enforces that direction.
//!
//! Its first component is [`project`]: it reads the stored artifacts of a project, at the
//! locations the core names, into the canonical input of `validate` (SPEC-0001, plan D1),
//! parsing YAML with the rules of the plan.

#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "the runtime performs I/O: the lists of clippy.toml bind the pure crates only"
)]

pub mod project;
mod refusal;
#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub mod test_support;
mod yaml;
