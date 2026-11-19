//! The envelope of every command result (ADR 0009 C1, with its amendment of 2026-11-19) and
//! the exit codes of C7.
//!
//! The types are contract types: they live in this crate, the core fills them and re-exports
//! them. The envelope is
//! `{command, data | error, diagnostics {warnings, not_done, checks}, meta {engine_version,
//! envelope_version, command_version, input_hash, dry_run, defaults_applied}}`. A result that is
//! not refused has `data` and no `error`; a refused one has `error`, the [`ErrorReport`], and no
//! `data` (SPEC-0004 R18). Lists are always written, `[]` when empty (C2). Nothing in the
//! envelope depends on the machine or the moment: it has no field for a clock time, a host name
//! or an absolute path (SPEC-0004 R15), and the values the engine writes in it come from the
//! input, the command and the engine version only.

use core::num::NonZeroU32;

use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};
use serde_json::Value;

use crate::{Error, ErrorReport, Pointer};

/// Version of the shape of the envelope, written in `meta.envelope_version`. It changes only
/// with a breaking change of the envelope itself (ADR 0009 C5).
pub const ENVELOPE_VERSION: NonZeroU32 = NonZeroU32::MIN;

/// Exit codes of the command line (ADR 0009 C7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ExitCode {
    /// 0: the command succeeded.
    Success,
    /// 1: the input was refused.
    InputError,
    /// 2: the command line was not understood.
    UsageError,
    /// 3: the environment failed: a file, the network or an output stream.
    EnvironmentError,
    /// 4: the loop stopped at a gate and waits for a person.
    StoppedAtGate,
    /// 5: a computation or internal error of the engine.
    InternalError,
}

impl ExitCode {
    /// All exit codes, in the order of C7.
    pub const ALL: [ExitCode; 6] = [
        ExitCode::Success,
        ExitCode::InputError,
        ExitCode::UsageError,
        ExitCode::EnvironmentError,
        ExitCode::StoppedAtGate,
        ExitCode::InternalError,
    ];

    /// The number the process exits with.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            ExitCode::Success => 0,
            ExitCode::InputError => 1,
            ExitCode::UsageError => 2,
            ExitCode::EnvironmentError => 3,
            ExitCode::StoppedAtGate => 4,
            ExitCode::InternalError => 5,
        }
    }
}

/// The SHA-256 of the canonical JSON (RFC 8785) of the input, in lower-case hexadecimal:
/// `meta.input_hash` (ADR 0009 C1). It serialises as its string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct InputHash(String);

impl InputHash {
    /// The hash written from its 32 bytes.
    #[must_use]
    pub fn from_digest(digest: &[u8; 32]) -> Self {
        let mut text = String::with_capacity(64);
        for byte in digest {
            for nibble in [byte >> 4, byte & 0x0f] {
                // A nibble is below 16, so it always has a digit, in lower case.
                text.extend(char::from_digit(u32::from(nibble), 16));
            }
        }
        Self(text)
    }

    /// The 64 lower-case hexadecimal digits.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A default the engine applied on behalf of the caller (ADR 0009 C2): an entry of
/// `meta.defaults_applied`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DefaultApplied {
    pointer: Pointer,
    value: Value,
    reason: String,
}

impl DefaultApplied {
    /// The default `value` applied at `pointer` of the input, with the `reason` for a person.
    #[must_use]
    pub fn new(pointer: Pointer, value: impl Into<Value>, reason: impl Into<String>) -> Self {
        Self {
            pointer,
            value: value.into(),
            reason: reason.into(),
        }
    }

    /// Where in the input the default applies.
    #[must_use]
    pub const fn pointer(&self) -> &Pointer {
        &self.pointer
    }

    /// The value applied.
    #[must_use]
    pub const fn value(&self) -> &Value {
        &self.value
    }

    /// Why the default applies, for a person.
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

/// What the result of a command is: its data, a refusal of the input, or an internal error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The command ran: `data` holds its result. Exit code 0.
    Success(Value),
    /// The input was refused: `error` holds every error found in one pass. Exit code 1.
    Refused(ErrorReport),
    /// The engine could not compute the result: `error` holds the report. Exit code 5.
    ///
    /// It is written as a refusal is, in `error`; the code of its errors tells the two apart.
    Failed(ErrorReport),
}

/// The field `diagnostics`: what the engine reports beside the result.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct Diagnostics {
    warnings: Vec<Error>,
    not_done: Vec<String>,
    checks: Vec<String>,
}

impl Diagnostics {
    /// Diagnostics with the `warnings` found (each in the shape of an error), the checks
    /// `not_done` and the identifiers of the `checks` run, each list in the order given.
    #[must_use]
    pub fn new(warnings: Vec<Error>, not_done: Vec<String>, checks: Vec<String>) -> Self {
        Self {
            warnings,
            not_done,
            checks,
        }
    }

    /// Warnings, in the shape of an error; the input is not refused for them.
    #[must_use]
    pub fn warnings(&self) -> &[Error] {
        &self.warnings
    }

    /// Checks the engine did not run, with the work left.
    #[must_use]
    pub fn not_done(&self) -> &[String] {
        &self.not_done
    }

    /// Identifiers of the checks the engine ran.
    #[must_use]
    pub fn checks(&self) -> &[String] {
        &self.checks
    }
}

/// The field `meta`: how the result was produced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Meta {
    engine_version: String,
    envelope_version: NonZeroU32,
    command_version: NonZeroU32,
    input_hash: InputHash,
    dry_run: bool,
    defaults_applied: Vec<DefaultApplied>,
}

impl Meta {
    /// The meta of a result of the engine `engine_version`, for version `command_version` of
    /// the command, on the input of hash `input_hash`, with the defaults applied in order. The
    /// envelope version is [`ENVELOPE_VERSION`].
    #[must_use]
    pub fn new(
        engine_version: impl Into<String>,
        command_version: NonZeroU32,
        input_hash: InputHash,
        dry_run: bool,
        defaults_applied: Vec<DefaultApplied>,
    ) -> Self {
        Self {
            engine_version: engine_version.into(),
            envelope_version: ENVELOPE_VERSION,
            command_version,
            input_hash,
            dry_run,
            defaults_applied,
        }
    }

    /// Version of the engine that produced the result.
    #[must_use]
    pub fn engine_version(&self) -> &str {
        &self.engine_version
    }

    /// Version of the shape of the envelope.
    #[must_use]
    pub const fn envelope_version(&self) -> NonZeroU32 {
        self.envelope_version
    }

    /// Version of the input and output schemas of the command.
    #[must_use]
    pub const fn command_version(&self) -> NonZeroU32 {
        self.command_version
    }

    /// Hash of the canonical input.
    #[must_use]
    pub const fn input_hash(&self) -> &InputHash {
        &self.input_hash
    }

    /// Whether the command only reported the changes it would make.
    #[must_use]
    pub const fn dry_run(&self) -> bool {
        self.dry_run
    }

    /// Every default applied, in order.
    #[must_use]
    pub fn defaults_applied(&self) -> &[DefaultApplied] {
        &self.defaults_applied
    }
}

/// The result of one command, on every channel (ADR 0009 C1, ADR 0028).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope {
    command: String,
    outcome: Outcome,
    diagnostics: Diagnostics,
    meta: Meta,
}

impl Envelope {
    /// The envelope of `command`, its identifier, with its outcome, diagnostics and meta.
    #[must_use]
    pub fn new(
        command: impl Into<String>,
        outcome: Outcome,
        diagnostics: Diagnostics,
        meta: Meta,
    ) -> Self {
        Self {
            command: command.into(),
            outcome,
            diagnostics,
            meta,
        }
    }

    /// Identifier of the command.
    #[must_use]
    pub fn command(&self) -> &str {
        &self.command
    }

    /// The data, the refusal or the internal error.
    #[must_use]
    pub const fn outcome(&self) -> &Outcome {
        &self.outcome
    }

    /// What the engine reports beside the result.
    #[must_use]
    pub const fn diagnostics(&self) -> &Diagnostics {
        &self.diagnostics
    }

    /// How the result was produced.
    #[must_use]
    pub const fn meta(&self) -> &Meta {
        &self.meta
    }

    /// The exit code of the outcome (ADR 0009 C7): 0 for a success, 1 for a refused input, 5
    /// for an internal error. The command line exits with it and never maps outcomes itself.
    #[must_use]
    pub const fn exit_code(&self) -> ExitCode {
        match self.outcome {
            Outcome::Success(_) => ExitCode::Success,
            Outcome::Refused(_) => ExitCode::InputError,
            Outcome::Failed(_) => ExitCode::InternalError,
        }
    }
}

impl Serialize for Envelope {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(4))?;
        map.serialize_entry("command", &self.command)?;
        match &self.outcome {
            Outcome::Success(data) => map.serialize_entry("data", data)?,
            Outcome::Refused(report) | Outcome::Failed(report) => {
                map.serialize_entry("error", report)?;
            }
        }
        map.serialize_entry("diagnostics", &self.diagnostics)?;
        map.serialize_entry("meta", &self.meta)?;
        map.end()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DefaultApplied, Diagnostics, ENVELOPE_VERSION, Envelope, ExitCode, InputHash, Meta, Outcome,
    };
    use crate::{Code, Error, ErrorReport, Pointer, Reason};
    use core::num::NonZeroU32;
    use serde_json::{Value, json};

    const INVALID_INPUT: Code = Code::new("invalid_input");

    fn at(pointer: &str) -> Pointer {
        Pointer::parse(pointer).unwrap()
    }

    fn hash() -> InputHash {
        InputHash::from_digest(&[0xab; 32])
    }

    fn meta() -> Meta {
        Meta::new("0.0.0", NonZeroU32::new(2).unwrap(), hash(), false, vec![])
    }

    fn report() -> ErrorReport {
        ErrorReport::from_errors([
            Error::new(
                INVALID_INPUT,
                "Unknown field.",
                at("/problema"),
                Reason::UnknownField,
            )
            .with_fix("problem"),
            Error::new(
                INVALID_INPUT,
                "No problem.",
                at("/problem"),
                Reason::Missing,
            ),
        ])
        .unwrap()
    }

    fn envelope(outcome: Outcome) -> Envelope {
        Envelope::new("validate", outcome, Diagnostics::default(), meta())
    }

    fn keys(value: &Value) -> Vec<&str> {
        value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect()
    }

    #[test]
    fn exit_codes_are_those_of_adr_0009_c7() {
        let codes: Vec<u8> = ExitCode::ALL.iter().map(|c| c.code()).collect();
        assert_eq!(codes, [0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn success_refusal_and_internal_error_exit_with_0_1_and_5() {
        assert_eq!(envelope(Outcome::Success(json!({}))).exit_code().code(), 0);
        assert_eq!(envelope(Outcome::Refused(report())).exit_code().code(), 1);
        assert_eq!(envelope(Outcome::Failed(report())).exit_code().code(), 5);
    }

    #[test]
    fn the_input_hash_is_lower_case_hexadecimal() {
        let mut digest = [0u8; 32];
        for (i, byte) in digest.iter_mut().enumerate() {
            *byte = u8::try_from(i * 8).unwrap();
        }
        let text = InputHash::from_digest(&digest);
        assert_eq!(
            text.as_str(),
            "0008101820283038404850586068707880889098a0a8b0b8c0c8d0d8e0e8f0f8"
        );
        assert_eq!(serde_json::to_value(&text).unwrap(), json!(text.as_str()));
    }

    #[test]
    fn a_success_has_data_and_no_error() {
        let value =
            serde_json::to_value(envelope(Outcome::Success(json!({"documents": []})))).unwrap();
        assert_eq!(keys(&value), ["command", "data", "diagnostics", "meta"]);
        assert_eq!(value["data"], json!({"documents": []}));
        assert_eq!(value["command"], json!("validate"));
    }

    #[test]
    fn a_refusal_has_the_error_report_and_no_data() {
        // SPEC-0004 R18, ADR 0009 C1 as amended on 2026-11-19.
        let value = serde_json::to_value(envelope(Outcome::Refused(report()))).unwrap();
        assert_eq!(keys(&value), ["command", "diagnostics", "error", "meta"]);
        assert_eq!(value["error"], serde_json::to_value(report()).unwrap());
        assert_eq!(value["error"]["errors"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn an_internal_error_is_written_in_error_too() {
        let value = serde_json::to_value(envelope(Outcome::Failed(report()))).unwrap();
        assert_eq!(keys(&value), ["command", "diagnostics", "error", "meta"]);
    }

    #[test]
    fn meta_holds_the_fields_of_c1_and_nothing_of_the_machine_or_the_moment() {
        let meta = Meta::new(
            "0.1.0",
            NonZeroU32::new(3).unwrap(),
            hash(),
            true,
            vec![DefaultApplied::new(
                at("/language"),
                "en",
                "The project language applies.",
            )],
        );
        assert_eq!(meta.envelope_version(), ENVELOPE_VERSION);
        assert_eq!(ENVELOPE_VERSION.get(), 1);
        let value = serde_json::to_value(&meta).unwrap();
        assert_eq!(
            value,
            json!({
                "engine_version": "0.1.0",
                "envelope_version": 1,
                "command_version": 3,
                "input_hash": hash().as_str(),
                "dry_run": true,
                "defaults_applied": [
                    {"pointer": "/language", "value": "en", "reason": "The project language applies."}
                ],
            })
        );
    }

    #[test]
    fn diagnostics_write_every_list_even_when_empty() {
        let value = serde_json::to_value(Diagnostics::default()).unwrap();
        assert_eq!(value, json!({"warnings": [], "not_done": [], "checks": []}));
        let warning = Error::new(INVALID_INPUT, "Old ADR.", at("/related/0"), Reason::State);
        let diagnostics = Diagnostics::new(
            vec![warning.clone()],
            vec!["links".into()],
            vec!["schema".into()],
        );
        assert_eq!(diagnostics.warnings(), core::slice::from_ref(&warning));
        assert_eq!(diagnostics.not_done(), ["links"]);
        assert_eq!(diagnostics.checks(), ["schema"]);
        assert_eq!(
            serde_json::to_value(&diagnostics).unwrap(),
            json!({
                "warnings": [serde_json::to_value(&warning).unwrap()],
                "not_done": ["links"],
                "checks": ["schema"],
            })
        );
    }

    #[test]
    fn the_accessors_return_what_was_given() {
        let applied = DefaultApplied::new(at("/a"), 1, "Default.");
        assert_eq!(applied.pointer(), &at("/a"));
        assert_eq!(applied.value(), &json!(1));
        assert_eq!(applied.reason(), "Default.");
        let meta = meta();
        assert_eq!(meta.engine_version(), "0.0.0");
        assert_eq!(meta.command_version().get(), 2);
        assert_eq!(meta.input_hash(), &hash());
        assert!(!meta.dry_run());
        assert!(meta.defaults_applied().is_empty());
        let envelope = envelope(Outcome::Refused(report()));
        assert_eq!(envelope.command(), "validate");
        assert_eq!(envelope.outcome(), &Outcome::Refused(report()));
        assert_eq!(envelope.diagnostics(), &Diagnostics::default());
        assert_eq!(envelope.meta(), &meta);
    }
}
