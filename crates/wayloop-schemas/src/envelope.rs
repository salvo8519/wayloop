//! The envelope of every command result (ADR 0009 C1, with its amendments of 2026-11-19 and
//! 2026-11-20) and the exit codes of C7.
//!
//! The types are contract types: they live in this crate, the core fills them and re-exports
//! them. The envelope is
//! `{command, data | error, diagnostics {warnings, not_done, checks}, meta {engine_version,
//! envelope_version, command_version, input_hash, dry_run, defaults_applied}}`. A result that
//! succeeded has `data` and no `error`; a refused or failed one has `error`, the
//! [`ErrorReport`], and no `data` (SPEC-0004 R18). A failure is told apart from a refusal by the
//! reason `internal` of its errors (C3, amended on 2026-11-20) and by its exit code: 5 for a
//! failure, 1 for a refusal. Lists are always written, `[]` when empty (C2). Every identifier is
//! a stable `snake_case` value, and clients accept values they do not know (C5). Nothing in the
//! envelope depends on the machine or the moment: it has no field for a clock time, a host name
//! or an absolute path (SPEC-0004 R15), and the values the engine writes in it come from the
//! input, the command and the engine version only.

use core::num::NonZeroU32;

use schemars::JsonSchema;
use serde::de::Error as _;
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use crate::{Error, ErrorReport, Pointer};

/// Version of the shape of the envelope, written in `meta.envelope_version`. It changes only
/// with a breaking change of the envelope itself (ADR 0009 C5).
pub const ENVELOPE_VERSION: NonZeroU32 = NonZeroU32::MIN;

/// Exit codes of the command line (ADR 0009 C7). An exit code is a number by contract, never
/// spelt: [`ExitCode::code`] is the one source of each number.
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct InputHash(#[schemars(length(min = 64, max = 64))] String);

/// Reads the text of a hash and refuses one that is not 64 lower-case hexadecimal digits.
impl<'de> Deserialize<'de> for InputHash {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        let hexadecimal = |b: u8| b.is_ascii_digit() || (b'a'..=b'f').contains(&b);
        if text.len() == 64 && text.bytes().all(hexadecimal) {
            Ok(Self(text))
        } else {
            Err(D::Error::custom(
                "an input hash is 64 lower-case hexadecimal digits",
            ))
        }
    }
}

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

crate::static_id! {
    /// Why the engine applied a default: a stable `snake_case` identifier, the field `reason` of an
    /// entry of `meta.defaults_applied`. Each value is a constant of the core, documented with the
    /// command that applies it; it is not a reason of C3. It serialises as its identifier.
    pub struct DefaultReason;
}

/// A default the engine applied on behalf of the caller (ADR 0009 C2): an entry of
/// `meta.defaults_applied`, `{pointer, value, reason, message}`. `message` is optional and left
/// out of the serialised form when not given. The value applied is never null: null means not
/// given (C2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DefaultApplied {
    /// RFC 6901 pointer, from the root of the input, to where the default applies.
    pointer: Pointer,
    /// The value applied.
    #[serde(deserialize_with = "not_null")]
    #[schemars(with = "Value")]
    value: Value,
    /// Stable `snake_case` identifier of why the default applies, documented with the command
    /// that applies it.
    reason: DefaultReason,
    /// Why the default applies, for a person; absent when not given.
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

/// Reads a value and refuses null, which means not given (ADR 0009 C2).
fn not_null<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Value, D::Error> {
    match Value::deserialize(deserializer)? {
        Value::Null => Err(D::Error::custom("the value applied is null")),
        value => Ok(value),
    }
}

impl DefaultApplied {
    /// The default `value` applied at `pointer` of the input, for `reason`; `None` when `value`
    /// is null, which means not given (ADR 0009 C2): a default is a value.
    #[must_use]
    pub fn new(pointer: Pointer, value: impl Into<Value>, reason: DefaultReason) -> Option<Self> {
        let value = value.into();
        (!value.is_null()).then_some(Self {
            pointer,
            value,
            reason,
            message: None,
        })
    }

    /// The entry with a `message` for a person.
    #[must_use]
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
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

    /// Why the default applies.
    #[must_use]
    pub const fn reason(&self) -> &DefaultReason {
        &self.reason
    }

    /// Why the default applies, for a person; `None` when not given.
    #[must_use]
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}

crate::static_id! {
    /// A check of the engine: a stable `snake_case` identifier, written in `diagnostics.checks`
    /// and in `diagnostics.not_done`. Each value is a constant of the core, documented with the
    /// command that runs it. It serialises as its identifier.
    pub struct Check;
}

crate::contract_enum! {
    /// Why a check was not run: the field `reason` of an entry of `diagnostics.not_done`. It
    /// serialises as its identifier; clients accept values they do not know (C5).
    pub enum NotDoneReason {
        /// The part of the engine that runs the check is not built yet.
        NotBuilt = "not_built",
        /// The input the check needs was not given, for example the base revision of a status
        /// check.
        InputAbsent = "input_absent",
    }
}

/// A check the engine did not run, and why: an entry of `diagnostics.not_done`,
/// `{check, reason}`.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub struct NotDone {
    /// Identifier of the check, documented with the command that runs it.
    check: Check,
    /// Why the check was not run.
    reason: NotDoneReason,
}

impl NotDone {
    /// The `check` not run, for `reason`.
    #[must_use]
    pub const fn new(check: Check, reason: NotDoneReason) -> Self {
        Self { check, reason }
    }

    /// The check not run.
    #[must_use]
    pub const fn check(&self) -> &Check {
        &self.check
    }

    /// Why it was not run.
    #[must_use]
    pub const fn reason(&self) -> NotDoneReason {
        self.reason
    }
}

/// What the result of a command is: its data, a refusal of the input, or an internal error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The command ran: `data` holds its result. Exit code 0.
    Success(Value),
    /// The input was refused: `error` holds every error found in one pass. Exit code 1.
    Refused(ErrorReport),
    /// The engine could not compute the result: `error` holds the report, whose errors have
    /// the reason [`Reason::Internal`](crate::Reason::Internal). Exit code 5.
    Failed(ErrorReport),
}

/// The field `diagnostics`: what the engine reports beside the result. All three lists are
/// always written, `[]` when empty.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub struct Diagnostics {
    /// Warnings, in the shape of an error; the input is not refused for them.
    warnings: Vec<Error>,
    /// Checks the engine did not run, each with its reason.
    not_done: Vec<NotDone>,
    /// Identifiers of the checks the engine ran, documented with the command that runs them.
    checks: Vec<Check>,
}

impl Diagnostics {
    /// Diagnostics with the `warnings` found (each in the shape of an error), the checks
    /// `not_done` with their reason, and the `checks` run, each list in the order given.
    #[must_use]
    pub fn new(warnings: Vec<Error>, not_done: Vec<NotDone>, checks: Vec<Check>) -> Self {
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

    /// Checks the engine did not run, each with its reason.
    #[must_use]
    pub fn not_done(&self) -> &[NotDone] {
        &self.not_done
    }

    /// Checks the engine ran.
    #[must_use]
    pub fn checks(&self) -> &[Check] {
        &self.checks
    }
}

/// The field `meta`: how the result was produced; nothing in it depends on the machine or the
/// moment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Meta {
    /// Version of the engine that produced the result.
    engine_version: String,
    /// Version of the shape of the envelope, from 1.
    envelope_version: NonZeroU32,
    /// Version of the input and output schemas of the command, from 1.
    command_version: NonZeroU32,
    /// SHA-256, in lower-case hexadecimal, of the canonical JSON (RFC 8785) of the input.
    input_hash: InputHash,
    /// Whether the command only reported the changes it would make.
    dry_run: bool,
    /// Every default the engine applied, in order (ADR 0009 C2); `[]` when none.
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
        Check, DefaultApplied, DefaultReason, Diagnostics, ENVELOPE_VERSION, Envelope, ExitCode,
        InputHash, Meta, NotDone, NotDoneReason, Outcome,
    };
    use crate::{Code, Error, ErrorReport, Pointer, Reason};
    use core::num::NonZeroU32;
    use serde_json::json;

    const INVALID_INPUT: Code = Code::new("invalid_input");
    const INTERNAL_ERROR: Code = Code::new("internal_error");
    const SCHEMA: Check = Check::new("schema");
    const LINKS: Check = Check::new("links");
    const PROJECT_LANGUAGE: DefaultReason = DefaultReason::new("project_language");

    fn at(pointer: &str) -> Pointer {
        Pointer::parse(pointer).unwrap()
    }

    fn hash() -> InputHash {
        InputHash::from_digest(&[0xab; 32])
    }

    fn meta() -> Meta {
        Meta::new("0.0.0", NonZeroU32::new(2).unwrap(), hash(), false, vec![])
    }

    fn refusal() -> ErrorReport {
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

    fn failure() -> ErrorReport {
        ErrorReport::from_errors([Error::new(
            INTERNAL_ERROR,
            "The engine could not compute the result.",
            Pointer::root(),
            Reason::Internal,
        )])
        .unwrap()
    }

    fn envelope(outcome: Outcome) -> Envelope {
        Envelope::new("validate", outcome, Diagnostics::default(), meta())
    }

    /// The text of `value` exactly as serialised: the order of the fields is checked too.
    fn text(value: &impl serde::Serialize) -> String {
        serde_json::to_string(value).unwrap()
    }

    fn empty_diagnostics_and_meta() -> String {
        format!(
            concat!(
                r#""diagnostics":{{"warnings":[],"not_done":[],"checks":[]}},"#,
                r#""meta":{{"engine_version":"0.0.0","envelope_version":1,"command_version":2,"#,
                r#""input_hash":"{}","dry_run":false,"defaults_applied":[]}}"#
            ),
            hash().as_str()
        )
    }

    #[test]
    fn exit_codes_are_those_of_adr_0009_c7() {
        let codes: Vec<u8> = ExitCode::ALL.iter().map(|c| c.code()).collect();
        assert_eq!(codes, [0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn success_refusal_and_internal_error_exit_with_0_1_and_5() {
        assert_eq!(envelope(Outcome::Success(json!({}))).exit_code().code(), 0);
        assert_eq!(envelope(Outcome::Refused(refusal())).exit_code().code(), 1);
        assert_eq!(envelope(Outcome::Failed(failure())).exit_code().code(), 5);
    }

    #[test]
    fn the_input_hash_is_lower_case_hexadecimal() {
        let mut digest = [0u8; 32];
        for (i, byte) in digest.iter_mut().enumerate() {
            *byte = u8::try_from(i * 8).unwrap();
        }
        let hash = InputHash::from_digest(&digest);
        assert_eq!(
            hash.as_str(),
            "0008101820283038404850586068707880889098a0a8b0b8c0c8d0d8e0e8f0f8"
        );
        assert_eq!(text(&hash), format!("\"{}\"", hash.as_str()));
    }

    #[test]
    fn a_success_has_data_and_no_error() {
        assert_eq!(
            text(&envelope(Outcome::Success(json!({"documents": []})))),
            format!(
                r#"{{"command":"validate","data":{{"documents":[]}},{}}}"#,
                empty_diagnostics_and_meta()
            )
        );
    }

    #[test]
    fn a_refusal_has_the_error_report_and_no_data() {
        // SPEC-0004 R18, ADR 0009 C1 as amended on 2026-11-19.
        assert_eq!(
            text(&envelope(Outcome::Refused(refusal()))),
            format!(
                r#"{{"command":"validate","error":{},{}}}"#,
                text(&refusal()),
                empty_diagnostics_and_meta()
            )
        );
    }

    #[test]
    fn an_internal_error_has_the_error_report_with_reason_internal_and_no_data() {
        // SPEC-0004 R18, ADR 0009 C3 as amended on 2026-11-20.
        let written = text(&envelope(Outcome::Failed(failure())));
        assert_eq!(
            written,
            format!(
                r#"{{"command":"validate","error":{},{}}}"#,
                text(&failure()),
                empty_diagnostics_and_meta()
            )
        );
        assert!(written.contains(r#""reason":"internal""#));
    }

    #[test]
    fn meta_holds_the_fields_of_c1_and_nothing_of_the_machine_or_the_moment() {
        let meta = Meta::new(
            "0.1.0",
            NonZeroU32::new(3).unwrap(),
            hash(),
            true,
            vec![
                DefaultApplied::new(at("/language"), "en", PROJECT_LANGUAGE)
                    .unwrap()
                    .with_message("The language of the project applies."),
                DefaultApplied::new(
                    at("/policy"),
                    json!({}),
                    DefaultReason::new("default_policy"),
                )
                .unwrap(),
            ],
        );
        assert_eq!(meta.envelope_version(), ENVELOPE_VERSION);
        assert_eq!(ENVELOPE_VERSION.get(), 1);
        assert_eq!(
            text(&meta),
            format!(
                concat!(
                    r#"{{"engine_version":"0.1.0","envelope_version":1,"command_version":3,"#,
                    r#""input_hash":"{}","dry_run":true,"defaults_applied":["#,
                    r#"{{"pointer":"/language","value":"en","reason":"project_language","#,
                    r#""message":"The language of the project applies."}},"#,
                    r#"{{"pointer":"/policy","value":{{}},"reason":"default_policy"}}]}}"#
                ),
                hash().as_str()
            )
        );
    }

    #[test]
    fn a_default_reason_is_an_identifier_and_its_message_is_optional() {
        let applied = DefaultApplied::new(at("/a"), 1, PROJECT_LANGUAGE).unwrap();
        assert_eq!(applied.pointer(), &at("/a"));
        assert_eq!(applied.value(), &json!(1));
        assert_eq!(applied.reason(), &PROJECT_LANGUAGE);
        assert_eq!(applied.reason().id(), "project_language");
        assert_eq!(PROJECT_LANGUAGE.to_string(), "project_language");
        assert_eq!(applied.message(), None);
        let applied = applied.with_message("Why.");
        assert_eq!(applied.message(), Some("Why."));
    }

    #[test]
    fn not_done_reasons_are_documented_snake_case_identifiers() {
        let ids: Vec<&str> = NotDoneReason::ALL.iter().map(|r| r.id()).collect();
        assert_eq!(ids, ["not_built", "input_absent"]);
        for reason in NotDoneReason::ALL {
            assert_eq!(text(&reason), format!("\"{}\"", reason.id()));
            assert_eq!(reason.to_string(), reason.id());
        }
    }

    #[test]
    fn diagnostics_write_every_list_even_when_empty() {
        assert_eq!(
            text(&Diagnostics::default()),
            r#"{"warnings":[],"not_done":[],"checks":[]}"#
        );
        let warning = Error::new(INVALID_INPUT, "Old ADR.", at("/related/0"), Reason::State);
        let skipped = NotDone::new(LINKS, NotDoneReason::InputAbsent);
        let diagnostics =
            Diagnostics::new(vec![warning.clone()], vec![skipped.clone()], vec![SCHEMA]);
        assert_eq!(diagnostics.warnings(), core::slice::from_ref(&warning));
        assert_eq!(diagnostics.not_done(), core::slice::from_ref(&skipped));
        assert_eq!(skipped.check(), &LINKS);
        assert_eq!(skipped.reason(), NotDoneReason::InputAbsent);
        assert_eq!(diagnostics.checks(), [SCHEMA]);
        assert_eq!(SCHEMA.id(), "schema");
        assert_eq!(SCHEMA.to_string(), "schema");
        assert_eq!(
            text(&diagnostics),
            format!(
                r#"{{"warnings":[{}],"not_done":[{{"check":"links","reason":"input_absent"}}],"checks":["schema"]}}"#,
                text(&warning)
            )
        );
    }

    #[test]
    fn the_accessors_return_what_was_given() {
        let meta = meta();
        assert_eq!(meta.engine_version(), "0.0.0");
        assert_eq!(meta.command_version().get(), 2);
        assert_eq!(meta.input_hash(), &hash());
        assert!(!meta.dry_run());
        assert_eq!(meta.defaults_applied(), []);
        // Values other than the defaults, so that an accessor returning a constant fails.
        let applied = DefaultApplied::new(at("/a"), 1, PROJECT_LANGUAGE).unwrap();
        let other = Meta::new(
            "0.0.0",
            NonZeroU32::MIN,
            hash(),
            true,
            vec![applied.clone()],
        );
        assert!(other.dry_run());
        assert_eq!(other.defaults_applied(), [applied]);
        let envelope = envelope(Outcome::Refused(refusal()));
        assert_eq!(envelope.command(), "validate");
        assert_eq!(envelope.outcome(), &Outcome::Refused(refusal()));
        assert_eq!(envelope.diagnostics(), &Diagnostics::default());
        assert_eq!(envelope.meta(), &meta);
    }

    #[test]
    fn a_default_is_a_value_never_null() {
        // ADR 0009 C2: null means not given, and the schema of an entry refuses it.
        assert_eq!(
            DefaultApplied::new(at("/a"), serde_json::Value::Null, PROJECT_LANGUAGE),
            None
        );
        let read = serde_json::from_value::<DefaultApplied>(
            json!({"pointer": "/a", "value": null, "reason": "project_language"}),
        );
        assert!(read.unwrap_err().to_string().contains("null"));
    }

    #[test]
    fn an_input_hash_is_read_only_in_its_form() {
        let hash = hash();
        let read: InputHash = serde_json::from_value(json!(hash.as_str())).unwrap();
        assert_eq!(read, hash);
        for wrong in [
            "ab".to_owned(),
            hash.as_str().to_uppercase(),
            format!("{}0", hash.as_str()),
            "g".repeat(64),
        ] {
            assert!(
                serde_json::from_value::<InputHash>(json!(wrong)).is_err(),
                "{wrong}"
            );
        }
    }

    #[test]
    fn identifiers_read_from_a_document_own_their_text() {
        let check: Check = serde_json::from_value(json!("links")).unwrap();
        assert_eq!(check, LINKS);
        assert_eq!(check.id(), "links");
        let reason: NotDoneReason = serde_json::from_value(json!("not_built")).unwrap();
        assert_eq!(reason, NotDoneReason::NotBuilt);
        assert!(serde_json::from_value::<NotDoneReason>(json!("later")).is_err());
    }
}
