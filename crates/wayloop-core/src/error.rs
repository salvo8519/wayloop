//! The error of the engine: a refusal is a value, never a panic (ADR 0009 C3).
//!
//! The type is written by hand, with no dependency, so that the core stays on its allowlist.
//! It carries as data the fields of a refusal that an agent acts on: the stable code, the
//! reason from the documented enum, and the RFC 6901 pointer to the part of the input that
//! caused it. The message, the other pointers (`pointers`), the expected values and the
//! suggested fix of C3, and the list `errors[]` of every error found in one pass, are added
//! with the first command that needs them, as are the codes themselves (C8).

use core::fmt;

/// Why the engine refused an input: the documented enum of ADR 0009 C3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Reason {
    /// The input is not well-formed.
    Syntax,
    /// A required field is absent.
    Missing,
    /// A field is not part of the schema.
    UnknownField,
    /// A value that must be unique appears more than once.
    Duplicate,
    /// A value has the wrong type.
    Type,
    /// A value is not one of the accepted values.
    Enum,
    /// Two values contradict each other.
    Conflict,
    /// A field is given without another field it requires.
    Requires,
    /// A number is outside its accepted range.
    Range,
    /// A string or a list is too short or too long.
    Length,
    /// The current state does not allow the command.
    State,
    /// The expected revision is older than the state (ADR 0009 C6).
    Stale,
}

impl Reason {
    /// All reasons, in the order of ADR 0009 C3.
    pub const ALL: [Reason; 12] = [
        Reason::Syntax,
        Reason::Missing,
        Reason::UnknownField,
        Reason::Duplicate,
        Reason::Type,
        Reason::Enum,
        Reason::Conflict,
        Reason::Requires,
        Reason::Range,
        Reason::Length,
        Reason::State,
        Reason::Stale,
    ];

    /// Stable identifier, as it appears in the `reason` field of a refusal.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Reason::Syntax => "syntax",
            Reason::Missing => "missing",
            Reason::UnknownField => "unknown_field",
            Reason::Duplicate => "duplicate",
            Reason::Type => "type",
            Reason::Enum => "enum",
            Reason::Conflict => "conflict",
            Reason::Requires => "requires",
            Reason::Range => "range",
            Reason::Length => "length",
            Reason::State => "state",
            Reason::Stale => "stale",
        }
    }
}

impl fmt::Display for Reason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

/// A stable error code. Codes are never renamed or reused (ADR 0009 C8); every code of the
/// engine is a constant of this type, so that one place lists them all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Code(&'static str);

impl Code {
    /// A code with the given `snake_case` identifier.
    #[must_use]
    pub const fn new(id: &'static str) -> Self {
        Self(id)
    }

    /// The identifier written in the error report.
    #[must_use]
    pub const fn id(self) -> &'static str {
        self.0
    }
}

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

/// A refusal of the engine, with the code, reason and pointer of ADR 0009 C3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    code: Code,
    reason: Reason,
    pointer: String,
}

impl Error {
    /// A refusal with its stable `code` (never renamed or reused, ADR 0009 C8), its `reason`,
    /// and the RFC 6901 `pointer` from the root of the input sent; `""` is the whole input.
    #[must_use]
    pub fn new(code: Code, reason: Reason, pointer: impl Into<String>) -> Self {
        Self {
            code,
            reason,
            pointer: pointer.into(),
        }
    }

    /// Stable error code.
    #[must_use]
    pub const fn code(&self) -> Code {
        self.code
    }

    /// Why the input was refused.
    #[must_use]
    pub const fn reason(&self) -> Reason {
        self.reason
    }

    /// RFC 6901 pointer to the refused part of the input; empty for the whole input.
    #[must_use]
    pub fn pointer(&self) -> &str {
        &self.pointer
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.pointer.is_empty() {
            write!(f, "{}: {} at the root of the input", self.code, self.reason)
        } else {
            write!(f, "{}: {} at {}", self.code, self.reason, self.pointer)
        }
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::{Code, Error, Reason};

    const INVALID_INPUT: Code = Code::new("invalid_input");
    const STALE_REVISION: Code = Code::new("stale_revision");

    // Exhaustive: a new variant does not compile until it gets a position here, and then
    // the test fails until ALL lists it in that position.
    const fn position(reason: Reason) -> usize {
        match reason {
            Reason::Syntax => 0,
            Reason::Missing => 1,
            Reason::UnknownField => 2,
            Reason::Duplicate => 3,
            Reason::Type => 4,
            Reason::Enum => 5,
            Reason::Conflict => 6,
            Reason::Requires => 7,
            Reason::Range => 8,
            Reason::Length => 9,
            Reason::State => 10,
            Reason::Stale => 11,
        }
    }

    #[test]
    fn all_lists_every_reason_once_in_order() {
        const COUNT: usize = 12;
        assert_eq!(Reason::ALL.len(), COUNT);
        for (i, reason) in Reason::ALL.into_iter().enumerate() {
            assert_eq!(position(reason), i);
        }
    }

    #[test]
    fn reasons_are_the_documented_enum_of_adr_0009() {
        let ids: Vec<&str> = Reason::ALL.iter().map(|r| r.id()).collect();
        assert_eq!(
            ids,
            [
                "syntax",
                "missing",
                "unknown_field",
                "duplicate",
                "type",
                "enum",
                "conflict",
                "requires",
                "range",
                "length",
                "state",
                "stale",
            ]
        );
    }

    #[test]
    fn an_error_carries_its_code_reason_and_pointer_as_data() {
        let error = Error::new(INVALID_INPUT, Reason::Missing, "/issue/title");
        assert_eq!(error.code().id(), "invalid_input");
        assert_eq!(error.reason(), Reason::Missing);
        assert_eq!(error.pointer(), "/issue/title");
    }

    #[test]
    fn display_names_the_code_the_reason_and_the_place() {
        let error = Error::new(INVALID_INPUT, Reason::UnknownField, "/stpe");
        assert_eq!(error.to_string(), "invalid_input: unknown_field at /stpe");
        let root = Error::new(INVALID_INPUT, Reason::Syntax, "");
        assert_eq!(
            root.to_string(),
            "invalid_input: syntax at the root of the input"
        );
    }

    #[test]
    fn it_is_a_standard_error_without_a_source() {
        let error: Box<dyn std::error::Error> =
            Box::new(Error::new(STALE_REVISION, Reason::Stale, "/revision"));
        assert!(error.source().is_none());
        assert_eq!(error.to_string(), "stale_revision: stale at /revision");
    }
}
