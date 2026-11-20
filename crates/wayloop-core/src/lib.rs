//! Wayloop core: the deterministic engine behind the spec-driven agentic loop.
//!
//! This crate holds all decision logic and performs no I/O. Adapters (CLI, Python and
//! TypeScript SDKs, MCP server) translate requests into calls to this crate and never
//! decide anything themselves (ADR 0001). The single entry point of ADR 0028 (command,
//! canonical input and context in, envelope out) is [`run`], at the root of this crate, and
//! every channel calls it, or [`run_json`] for an input sent as JSON text. Its one command is
//! [`Command::Validate`], which validates artifacts against their schemas ([`artifacts`]).
//! `wayloop-runtime` executes the work this crate decides (ADR 0006).
//!
//! The crate may depend only on the crates of its reviewed allowlist (`wayloop-schemas`, and
//! `serde`, `serde_json` and `schemars` with the crates they reach), checked by
//! `scripts/check_core_io.py`; I/O of the standard library in its source is refused by the
//! Clippy lints forbidden below, with the paths listed in the `clippy.toml` of the workspace.
//!
//! The crate fixes the vocabulary of the issue cycle, and holds the parts of the envelope of
//! ADR 0009 that the entry point needs: the canonical JSON of RFC 8785 ([`canonical`]), a
//! hand-written SHA-256 ([`sha256`]) and the hash of the input with the meta of a result
//! ([`envelope`]). The envelope types themselves are contract types of `wayloop-schemas`,
//! re-exported here; [`Envelope`] is generic over the data of a command.

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

pub mod artifacts;
pub mod canonical;
mod command;
pub mod envelope;
pub mod schema;
pub mod sha256;
#[cfg(test)]
mod test_support;

use core::num::NonZeroU32;

use serde::Serialize;
use serde_json::Value;

pub use command::{COMMAND_INPUT, CURRENT_COMMAND_VERSION, Command, Context, parse_json};
pub use envelope::envelope_json;
pub use wayloop_schemas::{
    Check, Code, DefaultApplied, DefaultReason, Diagnostics, ENVELOPE_VERSION, Envelope, Error,
    ErrorReport, ExitCode, INPUT_UNREADABLE, INTERNAL_ERROR, InputHash, Meta, NotDone,
    NotDoneReason, Outcome, Pointer, Reason, Source, USAGE,
};

/// The entry point of the engine (ADR 0028, SPEC-0004 R14): runs `command` on its canonical
/// `input` with the settings of the channel in `context`, and gives the envelope of ADR 0009
/// C1, with the data of the command as a JSON value, in the shape of its output contract. It
/// never panics: a refused input and a failure of the engine are outcomes of the envelope, whose
/// [`Envelope::exit_code`] the command line exits with. `meta.input_hash` is the SHA-256 of the
/// canonical JSON of `input` without its members set to null.
///
/// The version of the command comes from the context (ADR 0028, amended on 2026-11-20): when
/// the channel requests none, the current version runs and is listed in `meta.defaults_applied`
/// with the pointer `""`, since the version is a setting of the context and has no place in
/// the input; a version the engine does not run is refused with [`COMMAND_INPUT`], reason
/// `enum`, the known versions in `expected` and the pointer `""`, and its envelope reports the
/// current version in `meta.command_version`: the version whose shape the envelope has.
#[must_use]
pub fn run(command: Command, input: &Value, context: &Context) -> Envelope<Value> {
    let (outcome, diagnostics, version, defaults_applied) = match requested(command, *context) {
        Err(report) => (
            refusal(*report),
            Diagnostics::default(),
            command.version(),
            Vec::new(),
        ),
        Ok((version, defaults)) => {
            let (outcome, diagnostics) = match command {
                Command::Validate => {
                    let (outcome, diagnostics) = artifacts::run(input);
                    (as_value(outcome), diagnostics)
                }
            };
            (outcome, diagnostics, version, defaults)
        }
    };
    let meta = envelope::meta(version, input, context.dry_run(), defaults_applied);
    Envelope::new(command.id(), outcome, diagnostics, meta)
}

/// `outcome` with its data written as a JSON value: the one place where the data of a command
/// leaves its type.
fn as_value<D: Serialize>(outcome: Outcome<D>) -> Outcome<Value> {
    let written = outcome.data().map(serde_json::to_value);
    match written {
        None => outcome.map(|_| Value::Null),
        Some(Ok(value)) => Outcome::success(value)
            .unwrap_or_else(|| Outcome::internal("The data of the result is written as null.")),
        Some(Err(e)) => {
            Outcome::internal(format!("The data of the result cannot be written: {e}."))
        }
    }
}

/// The refusal of `report`, whose errors are refusals of the input.
fn refusal(report: ErrorReport) -> Outcome<Value> {
    Outcome::refused(report)
        .unwrap_or_else(|| Outcome::internal("The engine wrote a refusal that holds a failure."))
}

/// The version of `command` the context requests, with the default applied when it requests
/// none, or the refusal of a version the engine does not run.
fn requested(
    command: Command,
    context: Context,
) -> Result<(NonZeroU32, Vec<DefaultApplied>), Box<ErrorReport>> {
    let current = command.version();
    let Some(version) = context.command_version() else {
        let default = DefaultApplied::new(Pointer::root(), current.get(), CURRENT_COMMAND_VERSION)
            .map(|d| {
                d.with_message(format!(
                    "No version of the command was requested: version {current} runs."
                ))
            });
        return Ok((current, default.into_iter().collect()));
    };
    if command.runs(version) {
        return Ok((version, Vec::new()));
    }
    let known: Vec<u32> = (1..=current.get()).collect();
    let message = format!(
        "The command {command} has no version {version}; the versions it runs are 1 to {current}."
    );
    Err(Box::new(ErrorReport::single(
        Error::new(COMMAND_INPUT, message, Pointer::root(), Reason::Enum).with_expected(known),
    )))
}

/// [`run`] on an input sent as JSON `text`, as a channel that reads text calls it. Text that
/// is not JSON, that repeats a key in one object or that holds a number beyond a double, is
/// refused with the code [`COMMAND_INPUT`] before the command runs; `meta.input_hash` is then
/// the SHA-256 of the bytes received, since they have no canonical JSON (SPEC-0004 R11). Two
/// channels that send the same value with other bytes (line endings, a byte order mark) get the
/// same envelope only when the bytes are JSON. A version of the command the engine does not
/// run is reported in the same report, before the errors of the text.
#[must_use]
pub fn run_json(command: Command, text: &[u8], context: &Context) -> Envelope<Value> {
    match parse_json(text) {
        Ok(input) => run(command, &input, context),
        Err(report) => {
            let (outcome, version, defaults) = match requested(command, *context) {
                Err(refused) => {
                    let errors = refused.errors().chain(report.errors()).cloned();
                    let both = ErrorReport::from_errors(errors).unwrap_or(*refused);
                    (refusal(both), command.version(), Vec::new())
                }
                Ok((version, defaults)) => (refusal(*report), version, defaults),
            };
            let meta = envelope::meta_of_bytes(version, text, context.dry_run(), defaults);
            Envelope::new(command.id(), outcome, Diagnostics::default(), meta)
        }
    }
}

/// The envelope of a channel that was not used as documented, for example a command line its
/// adapter does not understand (ADR 0009 C7, exit code 2): code [`USAGE`], reason `syntax`, at
/// the root. `command` is the command the channel named, empty when it named none; `detail`
/// says what was not understood. No input was received, so `meta.input_hash` is the SHA-256 of
/// no bytes; `meta.command_version` is the current version of the command named, and 1 when it
/// names no known command, since there is then no data whose shape a version would describe.
#[must_use]
pub fn usage_error(command: &str, detail: &str) -> Envelope<Value> {
    let error = Error::new(USAGE, detail, Pointer::root(), Reason::Syntax);
    let outcome = Outcome::unusable(ErrorReport::single(error));
    stopped(command, outcome)
}

/// The envelope of an input that the channel could not read from the environment, for example
/// a file (ADR 0009 C7, exit code 3): code [`INPUT_UNREADABLE`] at the root, with `reason`
/// [`Reason::Missing`] when the input does not exist and [`Reason::State`] when it cannot be
/// read. The message, with no path, is the engine's. Any other reason is not a reason of this
/// code: the envelope is then a failure of the engine. No input was received, so
/// `meta.input_hash` is the SHA-256 of no bytes.
#[must_use]
pub fn input_unreadable(command: Command, reason: Reason) -> Envelope<Value> {
    let message = match reason {
        Reason::Missing => "The input does not exist.",
        Reason::State => "The input cannot be read.",
        other => {
            let failure = Outcome::internal(format!(
                "An unreadable input was reported with the reason {other}."
            ));
            return stopped(command.id(), Some(failure));
        }
    };
    let error = Error::new(INPUT_UNREADABLE, message, Pointer::root(), reason);
    let outcome = Outcome::environment(ErrorReport::single(error));
    stopped(command.id(), outcome)
}

/// The envelope of a command stopped before it ran, or a failure when `outcome` is none.
fn stopped(command: &str, outcome: Option<Outcome<Value>>) -> Envelope<Value> {
    let outcome =
        outcome.unwrap_or_else(|| Outcome::internal("The engine wrote a stop of another code."));
    let version = Command::from_id(command).map_or(NonZeroU32::MIN, Command::version);
    let meta = envelope::meta_of_bytes(version, b"", false, Vec::new());
    Envelope::new(command, outcome, Diagnostics::default(), meta)
}

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
