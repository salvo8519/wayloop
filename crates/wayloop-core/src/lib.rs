//! Wayloop core: the deterministic engine behind the spec-driven agentic loop.
//!
//! This crate holds all decision logic and performs no I/O. Adapters (CLI, Python and
//! TypeScript SDKs, MCP server) translate requests into calls to this crate and never
//! decide anything themselves (ADR 0001). The single entry point of ADR 0028 (command,
//! canonical input and context in, envelope out) belongs to this crate, at its root, and
//! every channel calls it; it is not written yet (#221). `wayloop-runtime` executes the work
//! this crate decides (ADR 0006).
//!
//! The crate may depend only on the crates of its reviewed allowlist (`wayloop-schemas`), checked
//! by `scripts/check_core_io.py`; I/O of the standard library in its source is refused by the
//! Clippy lints forbidden below, with the paths listed in its `clippy.toml`.
//!
//! The crate is a placeholder until milestone M1: it fixes the vocabulary of the issue cycle
//! so that the first specification has a stable starting point.

// I/O is refused by the compiler: these lints read the clippy.toml of this crate, and forbid,
// unlike deny, cannot be lifted by an inner allow. `scripts/check_core_io.py` keeps them here.
#![forbid(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::dbg_macro,
    clippy::disallowed_methods,
    clippy::disallowed_types
)]

mod error;

pub use error::{Error, Reason};

/// Version of the engine, taken from the workspace manifest.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Steps of the issue cycle, in order. The full state machine, with its transitions,
/// guards and approval states, is specified in milestone M1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum IssueStep {
    /// The issue is assigned to one agent session and labelled as in progress.
    Claim,
    /// The agent re-reads the issue, its story, its specification and the code involved.
    Reanalysis,
    /// A second agent looks for gaps, edge cases and contradictions.
    CounterReview,
    /// The task checklist is generated from the specification and published on the issue.
    Checklist,
    /// Tests where they pay, then the code.
    Build,
    /// Deterministic checks decide whether the step succeeded.
    Verify,
    /// Review proportional to risk.
    Review,
    /// The pull request is open and linked to the issue.
    PullRequest,
    /// Approval state: only a person merges.
    Merge,
}

impl IssueStep {
    /// All steps in their order.
    pub const ALL: [IssueStep; 9] = [
        IssueStep::Claim,
        IssueStep::Reanalysis,
        IssueStep::CounterReview,
        IssueStep::Checklist,
        IssueStep::Build,
        IssueStep::Verify,
        IssueStep::Review,
        IssueStep::PullRequest,
        IssueStep::Merge,
    ];

    /// Stable identifier used in the ledger and in the tracker.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            IssueStep::Claim => "claim",
            IssueStep::Reanalysis => "reanalysis",
            IssueStep::CounterReview => "counter-review",
            IssueStep::Checklist => "checklist",
            IssueStep::Build => "build",
            IssueStep::Verify => "verify",
            IssueStep::Review => "review",
            IssueStep::PullRequest => "pull-request",
            IssueStep::Merge => "merge",
        }
    }

    /// Whether the loop must stop and wait for a person at this step.
    #[must_use]
    pub const fn is_approval(self) -> bool {
        matches!(self, IssueStep::Merge)
    }
}

#[cfg(test)]
mod tests {
    use super::IssueStep;

    #[test]
    fn identifiers_are_unique_and_ordered() {
        let ids: Vec<&str> = IssueStep::ALL.iter().map(|s| s.id()).collect();
        let mut unique = ids.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), ids.len());
        assert!(IssueStep::ALL.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn only_merge_is_an_approval_state() {
        let approvals: Vec<IssueStep> = IssueStep::ALL
            .into_iter()
            .filter(|s| s.is_approval())
            .collect();
        assert_eq!(approvals, vec![IssueStep::Merge]);
    }
}
