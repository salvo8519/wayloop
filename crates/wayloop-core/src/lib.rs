//! Wayloop core: the deterministic engine behind the spec-driven agentic loop.
//!
//! This crate holds all decision logic and performs no I/O. Adapters (CLI, Python and
//! TypeScript SDKs, MCP server) translate requests into calls to this crate and never
//! decide anything themselves (ADR 0001). The single entry point of ADR 0028 (command,
//! canonical input and context in, envelope out) belongs to this crate, at its root, and
//! every channel calls it; it is not written yet (#221). `wayloop-runtime` executes the work
//! this crate decides (ADR 0006).
//!
//! The crate may depend only on the crates of its reviewed allowlist (`wayloop-schemas`, and
//! `serde`, `serde_json` and `schemars` with the crates they reach), checked by
//! `scripts/check_core_io.py`; I/O of the standard library in its source is refused by the
//! Clippy lints forbidden below, with the paths listed in its `clippy.toml`.
//!
//! The crate fixes the vocabulary of the issue cycle, and holds the parts of the envelope of
//! ADR 0009 that the entry point needs: the canonical JSON of RFC 8785 ([`canonical`]), a
//! hand-written SHA-256 ([`sha256`]) and the hash of the input with the meta of a result
//! ([`envelope`]). The envelope types themselves are contract types of `wayloop-schemas`,
//! re-exported here; [`Envelope`] is generic over the data of a command.

// I/O is refused by the compiler: these lints read the clippy.toml of this crate, and forbid,
// unlike deny, cannot be lifted by an inner allow. `scripts/check_core_io.py` keeps them here.
#![forbid(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::dbg_macro,
    clippy::disallowed_methods,
    clippy::disallowed_types
)]

pub mod canonical;
pub mod envelope;
pub mod schema;
pub mod sha256;
#[cfg(test)]
mod test_support;

pub use wayloop_schemas::{
    Check, Code, DefaultApplied, DefaultReason, Diagnostics, ENVELOPE_VERSION, Envelope, Error,
    ErrorReport, ExitCode, InputHash, Meta, NotDone, NotDoneReason, Outcome, Pointer, Reason,
    Source,
};

/// Version of the engine, taken from the workspace manifest.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

wayloop_schemas::closed_enum! {
    /// Steps of the issue cycle, in order. The full state machine, with its transitions,
    /// guards and approval states, is specified in milestone M1. The identifier of a step is
    /// its spelling in the contract: in the ledger, in hand-overs and in the JSON output of
    /// every channel, a `snake_case` value as every enum value of the contract (ADR 0009 C4,
    /// SPEC-0001), with one spelling everywhere. Other spellings, such as the kebab-case labels
    /// of a tracker, are produced by the adapters from it; the core never spells them.
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

impl IssueStep {
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

    /// Whether `id` matches `^[a-z][a-z0-9_]*$`, the spelling of contract enum values
    /// (ADR 0009 C4).
    fn is_snake_case(id: &str) -> bool {
        let mut chars = id.chars();
        chars.next().is_some_and(|c| c.is_ascii_lowercase())
            && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    }

    #[test]
    fn identifiers_are_snake_case_contract_values() {
        for step in IssueStep::ALL {
            assert!(is_snake_case(step.id()), "{:?} -> {}", step, step.id());
        }
        assert_eq!(IssueStep::CounterReview.id(), "counter_review");
        assert_eq!(IssueStep::PullRequest.id(), "pull_request");
    }

    #[test]
    fn the_snake_case_check_refuses_other_spellings() {
        for id in [
            "",
            "_claim",
            "1claim",
            "Claim",
            "counter-review",
            "pull request",
            "\u{e9}tape",
        ] {
            assert!(!is_snake_case(id), "{id}");
        }
        assert!(is_snake_case("a1_b"));
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
