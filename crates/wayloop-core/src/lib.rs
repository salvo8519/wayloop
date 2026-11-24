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
//! The crate decides on the steps of the issue cycle ([`IssueStep`], a contract type of
//! `wayloop-schemas` re-exported here, since the hand-over records it; [`is_approval`]), and
//! holds the parts of the envelope of ADR 0009 that the entry point needs: the canonical JSON
//! of RFC 8785 ([`canonical`]), a hand-written SHA-256 ([`sha256`]) and the hash of the input
//! with the meta of a result ([`envelope`]). The envelope types themselves are contract types
//! of `wayloop-schemas`, re-exported here; [`Envelope`] is generic over the data of a command.

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
    ErrorReport, ExitCode, INPUT_UNREADABLE, INTERNAL_ERROR, InputHash, IssueStep, Meta, NotDone,
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
            reported(*report),
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

/// Whether `report` is a failure of the engine: its first error has the reason `internal`.
fn is_failure(report: &ErrorReport) -> bool {
    report.first().reason() == Reason::Internal
}

/// The outcome `report` tells: a failure of the engine when it is one, a refusal of the input
/// otherwise; a report that mixes them is itself a failure.
fn reported(report: ErrorReport) -> Outcome<Value> {
    let outcome = if is_failure(&report) {
        Outcome::failed(report)
    } else {
        Outcome::refused(report)
    };
    outcome.unwrap_or_else(|| {
        Outcome::internal("The engine wrote a report that mixes refusals and failures.")
    })
}

/// The version of `command` the context requests, with the default applied when it requests
/// none, or the refusal of a version the engine does not run; a failure of the engine when the
/// default cannot be written, never a default dropped.
fn requested(
    command: Command,
    context: Context,
) -> Result<(NonZeroU32, Vec<DefaultApplied>), Box<ErrorReport>> {
    let current = command.version();
    let Some(version) = context.command_version() else {
        let Some(default) =
            DefaultApplied::new(Pointer::root(), current.get(), CURRENT_COMMAND_VERSION)
        else {
            let message = "The default version of the command is written as null.";
            return Err(Box::new(ErrorReport::single(Error::internal(message))));
        };
        let message = format!("No version of the command was requested: version {current} runs.");
        return Ok((current, vec![default.with_message(message)]));
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
                // A failure of the engine stands alone; a refused version comes first.
                Err(stop) if is_failure(&stop) => (reported(*stop), command.version(), Vec::new()),
                Err(refused) => {
                    let errors = refused.errors().chain(report.errors()).cloned();
                    let both = ErrorReport::from_errors(errors).unwrap_or(*refused);
                    (reported(both), command.version(), Vec::new())
                }
                Ok((version, defaults)) => (reported(*report), version, defaults),
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

/// Whether the loop must stop and wait for a person at `step`.
#[must_use]
pub const fn is_approval(step: IssueStep) -> bool {
    matches!(step, IssueStep::Merge)
}

#[cfg(test)]
mod tests {
    use super::{
        COMMAND_INPUT, Error, ErrorReport, INTERNAL_ERROR, IssueStep, Pointer, Reason, is_approval,
        reported,
    };

    #[test]
    fn a_report_tells_its_outcome_and_a_mixed_one_is_a_failure() {
        let refusal = Error::new(COMMAND_INPUT, "Refused.", Pointer::root(), Reason::Type);
        let failure = Error::internal("Broken.");
        let refused = reported(ErrorReport::single(refusal.clone()));
        assert_eq!(refused.exit_code().code(), 1);
        let failed = reported(ErrorReport::single(failure.clone()));
        assert_eq!(failed.exit_code().code(), 5);
        assert_eq!(failed.report().unwrap().first().message(), "Broken.");
        for mixed in [[refusal.clone(), failure.clone()], [failure, refusal]] {
            let outcome = reported(ErrorReport::from_errors(mixed).unwrap());
            let first = outcome.report().unwrap().first().clone();
            assert_eq!(
                (first.code(), first.reason()),
                (&INTERNAL_ERROR, Reason::Internal)
            );
            assert!(first.message().contains("mixes"), "{first:?}");
        }
    }

    #[test]
    fn only_merge_is_an_approval_state() {
        let approvals: Vec<IssueStep> = IssueStep::ALL
            .into_iter()
            .filter(|s| is_approval(*s))
            .collect();
        assert_eq!(approvals, vec![IssueStep::Merge]);
    }
}
