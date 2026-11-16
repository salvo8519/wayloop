//! Wayloop core: the deterministic engine behind the spec-driven agentic loop.
//!
//! This crate holds all decision logic and performs no I/O. Adapters (CLI, Python and
//! TypeScript SDKs, MCP server) translate requests into calls to this crate and never
//! decide anything themselves (ADR 0001).
//!
//! The crate is a placeholder until milestone M1: it fixes the vocabulary of the issue cycle
//! so that the first specification has a stable starting point.

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
