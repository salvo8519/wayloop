//! The vocabulary of the issue cycle that reaches a contract: the steps, which the hand-over of
//! SPEC-0001 records and the state machine of SPEC-0002 builds on. A contract type, so it lives
//! here (ADR 0030 G1); the decisions on the steps belong to `wayloop-core`, which re-exports it.

crate::contract_enum! {
    /// Step of the issue cycle, in order. The identifier of a step is its spelling in the
    /// contract: in the ledger, in hand-overs and in the JSON output of every channel, a
    /// `snake_case` value as every enum value of the contract (ADR 0009 C4, SPEC-0001), with one
    /// spelling everywhere. Other spellings, such as the kebab-case labels of a tracker, are
    /// produced by the adapters from it; the core never spells them.
    pub enum IssueStep {
        /// The issue is assigned to one agent session and labelled as in progress.
        Claim = "claim",
        /// The agent re-reads the issue, its story, its specification and the code involved.
        Reanalysis = "reanalysis",
        /// A second agent looks for gaps, edge cases and contradictions.
        CounterReview = "counter_review",
        /// The task checklist is generated from the specification and published on the issue.
        Checklist = "checklist",
        /// Tests where they pay, then the code.
        Build = "build",
        /// Deterministic checks decide whether the step succeeded.
        Verify = "verify",
        /// Review proportional to risk.
        Review = "review",
        /// The pull request is open and linked to the issue.
        PullRequest = "pull_request",
        /// Approval state: only a person merges.
        Merge = "merge",
    }
}
