//! The commands of the entry point (ADR 0028), the context a channel passes with them, and the
//! reading of an input sent as JSON text.

use core::cell::RefCell;
use core::fmt;
use core::num::NonZeroU32;

use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value};

use wayloop_schemas::contract::{Contract, VALIDATE};

use crate::{Code, DefaultReason, Error, ErrorReport, Pointer, Reason};

/// The input of a command is refused: it is not JSON, repeats a key, or breaks the input
/// schema of the command or one of the rules of its input. No other check runs.
pub const COMMAND_INPUT: Code = Code::new("command_input");

/// The engine could not compute the result; the input may be valid. Its errors have the
/// reason `internal` and the command exits with code 5 (ADR 0009 C3, C7).
pub const INTERNAL_ERROR: Code = Code::new("internal_error");

/// Why the engine applied the current version of a command: the channel did not request one
/// (ADR 0028, amended on 2026-11-20).
pub const CURRENT_COMMAND_VERSION: DefaultReason = DefaultReason::new("current_command_version");

/// A command of the engine. Its identifier and its versions are those of its contract in the
/// registry of `wayloop-schemas` (ADR 0030 G5): one source for both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Command {
    /// `validate`: checks a set of artifacts and inputs (SPEC-0001).
    Validate,
}

impl Command {
    /// All commands.
    pub const ALL: [Command; 1] = [Command::Validate];

    /// The contract of the command.
    #[must_use]
    pub const fn contract(self) -> &'static Contract {
        match self {
            Command::Validate => &VALIDATE,
        }
    }

    /// Identifier of the command, written in `command` of the envelope: the name of its
    /// contract.
    #[must_use]
    pub const fn id(self) -> &'static str {
        self.contract().name
    }

    /// Current version of the input and output schemas of the command.
    #[must_use]
    pub const fn version(self) -> NonZeroU32 {
        self.contract().version
    }

    /// Whether the engine runs `version` of the command: every version from 1 to the current
    /// one, since a contract never drops a version (ADR 0030 G5).
    #[must_use]
    pub const fn runs(self, version: NonZeroU32) -> bool {
        version.get() <= self.version().get()
    }

    /// The command whose identifier is `id`.
    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.id() == id)
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

/// The settings of a channel, passed to the entry point with the command (ADR 0028): the
/// engine computes their effect and reports them in the envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Context {
    dry_run: bool,
    command_version: Option<NonZeroU32>,
}

impl Context {
    /// The context with the version of the command the channel requests (ADR 0028, amended
    /// on 2026-11-20). Without one, the engine runs the current version and lists it in
    /// `meta.defaults_applied`; an unknown version is refused.
    #[must_use]
    pub const fn with_command_version(mut self, version: NonZeroU32) -> Self {
        self.command_version = Some(version);
        self
    }

    /// The version of the command the channel requested, if any.
    #[must_use]
    pub const fn command_version(self) -> Option<NonZeroU32> {
        self.command_version
    }

    /// The context with `dry_run` set: the command only reports the changes it would make.
    #[must_use]
    pub const fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    /// Whether the command only reports the changes it would make, written in `meta.dry_run`.
    #[must_use]
    pub const fn dry_run(self) -> bool {
        self.dry_run
    }
}

/// The JSON value of `text`, or every error that refuses it with the code
/// [`COMMAND_INPUT`]: `range` at the pointer of a number beyond the range of a double (such as
/// `1e400`), `syntax` at the root when it is not JSON, or `duplicate` at each member that
/// repeats a key of its object (plan of SPEC-0001, decided on 2026-11-20). A JSON value cannot
/// hold two equal keys, so a parser that kept one of them would change the input silently.
///
/// # Errors
///
/// The report of the errors that refuse `text`, in the order found.
pub fn parse_json(text: &[u8]) -> Result<Value, Box<ErrorReport>> {
    let state = State::default();
    let mut deserializer = serde_json::Deserializer::from_slice(text);
    let parsed = Seed {
        at: Pointer::root(),
        state: &state,
    }
    .deserialize(&mut deserializer)
    .and_then(|value| deserializer.end().map(|()| value));
    match parsed {
        Err(error) => {
            // serde_json has no category for it: its message names it, and a test of this
            // module fails if the message changes.
            let error = if error.to_string().starts_with(OUT_OF_RANGE) {
                let message = format!("The number is beyond the range of a double: {error}.");
                Error::new(
                    COMMAND_INPUT,
                    message,
                    state.last.into_inner(),
                    Reason::Range,
                )
            } else {
                let message = format!("The input is not JSON: {error}.");
                Error::new(COMMAND_INPUT, message, Pointer::root(), Reason::Syntax)
            };
            Err(Box::new(ErrorReport::single(error)))
        }
        Ok(value) => {
            let errors = state.duplicates.into_inner().into_iter().map(|at| {
                let message = "The key appears more than once in its object.".to_owned();
                Error::new(COMMAND_INPUT, message, at, Reason::Duplicate)
            });
            ErrorReport::from_errors(errors).map_or(Ok(value), |report| Err(Box::new(report)))
        }
    }
}

/// The start of the message of `serde_json` for a number beyond the range of a double.
const OUT_OF_RANGE: &str = "number out of range";

/// What the parser records on its way: every repeated key, and the value it entered last.
#[derive(Default)]
struct State {
    duplicates: RefCell<Vec<Pointer>>,
    last: RefCell<Pointer>,
}

/// Builds the value at `at`, recording the pointer of every repeated key.
struct Seed<'a> {
    at: Pointer,
    state: &'a State,
}

impl<'de> DeserializeSeed<'de> for Seed<'_> {
    type Value = Value;

    fn deserialize<D: de::Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        // A number is a leaf: when it fails to parse, the value entered last is the number.
        self.state.last.replace(self.at.clone());
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Seed<'_> {
    type Value = Value;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a JSON value")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Value, E> {
        Ok(Value::Bool(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Value, E> {
        Ok(Value::from(v))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Value, E> {
        Ok(Value::from(v))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Value, E> {
        Ok(Value::from(v))
    }

    fn visit_str<E>(self, v: &str) -> Result<Value, E> {
        Ok(Value::String(v.to_owned()))
    }

    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        let mut items = Vec::new();
        loop {
            let seed = Seed {
                at: self.at.join_index(items.len()),
                state: self.state,
            };
            match seq.next_element_seed(seed)? {
                Some(item) => items.push(item),
                None => return Ok(Value::Array(items)),
            }
        }
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        let mut members = Map::new();
        while let Some(key) = map.next_key::<String>()? {
            let at = self.at.join(&key);
            if members.contains_key(&key) {
                self.state.duplicates.borrow_mut().push(at.clone());
            }
            let value = map.next_value_seed(Seed {
                at,
                state: self.state,
            })?;
            members.insert(key, value);
        }
        Ok(Value::Object(members))
    }
}

#[cfg(test)]
mod tests {
    use super::{COMMAND_INPUT, Command, Context, parse_json};
    use crate::Reason;
    use serde_json::json;

    fn refused(text: &str) -> Vec<(String, Reason)> {
        parse_json(text.as_bytes())
            .unwrap_err()
            .errors()
            .inspect(|e| assert_eq!(*e.code(), COMMAND_INPUT))
            .map(|e| (e.pointer().to_string(), e.reason()))
            .collect()
    }

    #[test]
    fn json_text_parses_to_the_value_serde_json_gives() {
        let text = r#"{"a": [1, -2, 3.5, 18446744073709551616, 4364.0, true, null, "x\u00e9"],
            "b": {"c": {}}, "": []}"#;
        assert_eq!(
            parse_json(text.as_bytes()).unwrap(),
            serde_json::from_str::<serde_json::Value>(text).unwrap()
        );
        // A number beyond the range of a double is refused with range, at its pointer.
        assert_eq!(refused("1e400"), [(String::new(), Reason::Range)]);
        assert_eq!(
            refused(r#"{"a": [1, {"b~": -1e400}], "c": 2}"#),
            [("/a/1/b~0".to_owned(), Reason::Range)]
        );
    }

    #[test]
    fn every_repeated_key_is_refused_at_its_pointer() {
        assert_eq!(
            refused(r#"{"documents": [], "documents": []}"#),
            [("/documents".to_owned(), Reason::Duplicate)]
        );
        assert_eq!(
            refused(r#"{"a": [{"b~/": 1, "b~/": 2}], "c": {"d": 1, "d": 1, "d": 1}}"#),
            [
                ("/a/0/b~0~1".to_owned(), Reason::Duplicate),
                ("/c/d".to_owned(), Reason::Duplicate),
                ("/c/d".to_owned(), Reason::Duplicate),
            ]
        );
        assert_eq!(
            parse_json(br#"{"a": {"a": 1}}"#).unwrap(),
            json!({"a": {"a": 1}})
        );
    }

    #[test]
    fn text_that_is_not_json_is_refused_with_syntax_at_the_root() {
        for text in ["", "{", r#"{"a": 1} x"#, "[1,]", "\u{feff}{}", "nul"] {
            assert_eq!(refused(text), [(String::new(), Reason::Syntax)], "{text:?}");
        }
        let invalid_utf8 = [b'"', 0xff, b'"'];
        assert!(parse_json(&invalid_utf8).is_err());
    }

    #[test]
    fn a_value_json_cannot_hold_is_refused_as_not_a_json_value() {
        use serde::de::DeserializeSeed;
        use serde::de::value::{BytesDeserializer, Error};
        let state = super::State::default();
        let seed = super::Seed {
            at: crate::Pointer::root(),
            state: &state,
        };
        let error = seed
            .deserialize(BytesDeserializer::<Error>::new(b"x"))
            .unwrap_err();
        assert!(error.to_string().contains("a JSON value"), "{error}");
    }

    #[test]
    fn a_command_has_an_identifier_and_a_version() {
        assert_eq!(Command::ALL.map(Command::id), ["validate"]);
        assert_eq!(Command::Validate.to_string(), "validate");
        assert_eq!(Command::Validate.version().get(), 1);
    }

    #[test]
    fn the_context_carries_the_dry_run_and_the_command_version() {
        assert!(!Context::default().dry_run());
        assert!(Context::default().with_dry_run(true).dry_run());
        assert_eq!(Context::default().command_version(), None);
        let two = core::num::NonZeroU32::new(2).unwrap();
        assert_eq!(
            Context::default()
                .with_command_version(two)
                .command_version(),
            Some(two)
        );
    }

    #[test]
    fn a_command_is_found_by_its_identifier_and_runs_its_versions() {
        assert_eq!(Command::from_id("validate"), Some(Command::Validate));
        assert_eq!(Command::from_id("Validate"), None);
        assert_eq!(Command::from_id(""), None);
        let current = Command::Validate.version();
        assert!(Command::Validate.runs(core::num::NonZeroU32::MIN));
        assert!(Command::Validate.runs(current));
        assert!(!Command::Validate.runs(current.saturating_add(1)));
        assert_eq!(Command::Validate.contract().name, "validate");
    }
}
