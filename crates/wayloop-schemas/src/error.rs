//! The error report of the engine: a refusal is a value, never a panic (ADR 0009 C3).
//!
//! The types are contract types: they live in this crate and `wayloop-core` re-exports them,
//! so that the engine is reached at `wayloop_core::Error`. An [`Error`] carries as data every
//! field of a refusal that an agent acts on (`code`, `message`, `pointer`, `pointers`,
//! `reason`, `expected`, `fix`) and `source`, where the faulty value is stored (SPEC-0001
//! Errors, R12). An [`ErrorReport`] lists every error found in one pass in `errors`, the first
//! one repeated at the top level. An optional field that is not given is left out of the
//! serialised form, and an empty list is written `[]`: absent means not given, `[]` means none
//! (C2). The codes themselves are constants of the core (C8).

use core::fmt;
use core::num::NonZeroU64;

use serde::{Serialize, Serializer};
use serde_json::Value;

/// Why the engine refused an input, or [`Reason::Internal`] when it failed to compute the
/// result: the documented enum of ADR 0009 C3 and its amendment of 2026-11-20. It serialises as
/// [`Reason::id`]; clients accept values they do not know (C5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
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
    /// The engine could not compute the result: the input may be valid (ADR 0009 C3, amended
    /// on 2026-11-20). The command exits with code 5, not 1 (C7).
    Internal,
}

impl Reason {
    /// All reasons, in the order of ADR 0009 C3 and its amendments.
    pub const ALL: [Reason; 13] = [
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
        Reason::Internal,
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
            Reason::Internal => "internal",
        }
    }
}

impl fmt::Display for Reason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

/// A stable error code. Codes are never renamed or reused (ADR 0009 C8); every code of the
/// engine is a constant of this type, so that one place lists them all. It serialises as its
/// identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
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

/// An RFC 6901 JSON pointer from the root of the input sent: empty for the whole input, or a
/// sequence of `/`-prefixed reference tokens in which `~` is escaped as `~0` or `~1`. It
/// serialises as its string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize)]
#[serde(transparent)]
pub struct Pointer(String);

impl Pointer {
    /// The pointer to the whole input, `""`.
    #[must_use]
    pub const fn root() -> Self {
        Self(String::new())
    }

    /// The pointer written as `text`, or `None` when `text` is not in the form of RFC 6901.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        if !(text.is_empty() || text.starts_with('/')) {
            return None;
        }
        let mut chars = text.chars();
        while let Some(c) = chars.next() {
            if c == '~' && !matches!(chars.next(), Some('0' | '1')) {
                return None;
            }
        }
        Some(Self(text.to_owned()))
    }

    /// The pointer to the member `token` of the value at this pointer: `token` is appended as
    /// one reference token, with `~` written `~0` and `/` written `~1` (RFC 6901).
    #[must_use]
    pub fn join(&self, token: &str) -> Self {
        let mut text =
            String::with_capacity(self.0.len().saturating_add(token.len()).saturating_add(1));
        text.push_str(&self.0);
        text.push('/');
        for c in token.chars() {
            match c {
                '~' => text.push_str("~0"),
                '/' => text.push_str("~1"),
                other => text.push(other),
            }
        }
        Self(text)
    }

    /// The pointer to the element `index` of the array at this pointer.
    #[must_use]
    pub fn join_index(&self, index: usize) -> Self {
        Self(format!("{}/{index}", self.0))
    }

    /// The pointer as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Pointer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Where the faulty value is stored (SPEC-0001 Errors, R12): the field `source` of an error.
///
/// It serialises as the object of its form, with no tag, because the field names tell the
/// forms apart and the field is output only. A `Deserialize`, if one is ever added, must put
/// `#[serde(deny_unknown_fields)]` on each form, or an untagged enum would accept an object
/// with extra fields as the first form that matches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Source {
    /// A repository file: its path relative to the root, with `/` separators, and the line of
    /// the value.
    File {
        /// Path of the file relative to the root of the repository.
        file: String,
        /// Line of the faulty value; lines are counted from 1.
        line: NonZeroU64,
    },
    /// A tracker issue, by its number.
    Issue {
        /// Number of the issue.
        issue: NonZeroU64,
    },
    /// A tracker milestone, by its number.
    Milestone {
        /// Number of the milestone.
        milestone: NonZeroU64,
    },
}

/// A refusal of the engine, with the fields of ADR 0009 C3 and its `source`.
///
/// `pointers`, `expected`, `fix` and `source` are optional and left out of the serialised form
/// when not given; a list given empty is written `[]` (C2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Error {
    code: Code,
    message: String,
    pointer: Pointer,
    #[serde(skip_serializing_if = "Option::is_none")]
    pointers: Option<Vec<Pointer>>,
    reason: Reason,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fix: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<Source>,
}

impl Error {
    /// A refusal with its stable `code` (never renamed or reused, ADR 0009 C8), a `message`
    /// for a person, the `pointer` to the refused part of the input, and its `reason`: the
    /// required fields of C3, in its order.
    #[must_use]
    pub fn new(code: Code, message: impl Into<String>, pointer: Pointer, reason: Reason) -> Self {
        Self {
            code,
            message: message.into(),
            pointer,
            pointers: None,
            reason,
            expected: None,
            fix: None,
            source: None,
        }
    }

    /// The error with the pointers of the other parts of the input involved, such as both
    /// sides of a conflict.
    #[must_use]
    pub fn with_pointers(mut self, pointers: impl IntoIterator<Item = Pointer>) -> Self {
        self.pointers = Some(pointers.into_iter().collect());
        self
    }

    /// The error with the values the input accepts at `pointer`, never a pattern; an empty
    /// list says that no value is accepted.
    #[must_use]
    pub fn with_expected<I>(mut self, expected: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<Value>,
    {
        self.expected = Some(expected.into_iter().map(Into::into).collect());
        self
    }

    /// The error with a suggested fix: the closest field for a misspelt key, or a corrected
    /// value.
    #[must_use]
    pub fn with_fix(mut self, fix: impl Into<Value>) -> Self {
        self.fix = Some(fix.into());
        self
    }

    /// The error with the place where the faulty value is stored, serialised as `source`.
    #[must_use]
    pub fn with_location(mut self, source: Source) -> Self {
        self.source = Some(source);
        self
    }

    /// Stable error code.
    #[must_use]
    pub const fn code(&self) -> Code {
        self.code
    }

    /// What went wrong, for a person.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Pointer to the refused part of the input; [`Pointer::root`] for the whole input.
    #[must_use]
    pub const fn pointer(&self) -> &Pointer {
        &self.pointer
    }

    /// Pointers to the other parts of the input involved; `None` when not given.
    #[must_use]
    pub fn pointers(&self) -> Option<&[Pointer]> {
        self.pointers.as_deref()
    }

    /// Why the input was refused.
    #[must_use]
    pub const fn reason(&self) -> Reason {
        self.reason
    }

    /// Values accepted at `pointer`; `None` when not given, empty when none is accepted.
    #[must_use]
    pub fn expected(&self) -> Option<&[Value]> {
        self.expected.as_deref()
    }

    /// Suggested fix, if one can be derived.
    #[must_use]
    pub const fn fix(&self) -> Option<&Value> {
        self.fix.as_ref()
    }

    /// Where the faulty value is stored, serialised as `source`; absent for a document sent
    /// directly. Named apart from [`std::error::Error::source`], which is the cause of an error.
    #[must_use]
    pub const fn location(&self) -> Option<&Source> {
        self.source.as_ref()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.pointer.as_str().is_empty() {
            write!(f, "{}: {} at the root of the input", self.code, self.reason)?;
        } else {
            write!(f, "{}: {} at {}", self.code, self.reason, self.pointer)?;
        }
        write!(f, ": {}", self.message)
    }
}

impl std::error::Error for Error {}

/// Every error found in one pass (ADR 0009 C3). It serialises as the first error, with all
/// the errors in `errors`, so that `errors[0]` equals the top level by construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorReport {
    first: Error,
    rest: Vec<Error>,
}

impl ErrorReport {
    /// The report of `errors`, in the order they were found; `None` when there is none.
    #[must_use]
    pub fn from_errors(errors: impl IntoIterator<Item = Error>) -> Option<Self> {
        let mut errors = errors.into_iter();
        let first = errors.next()?;
        Some(Self {
            first,
            rest: errors.collect(),
        })
    }

    /// The first error, repeated at the top level.
    #[must_use]
    pub const fn first(&self) -> &Error {
        &self.first
    }

    /// Every error, the first included, in the order found.
    pub fn errors(&self) -> impl Iterator<Item = &Error> {
        core::iter::once(&self.first).chain(&self.rest)
    }
}

impl Serialize for ErrorReport {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Wire<'a> {
            #[serde(flatten)]
            top: &'a Error,
            errors: Errors<'a>,
        }

        struct Errors<'a>(&'a ErrorReport);

        impl Serialize for Errors<'_> {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.collect_seq(self.0.errors())
            }
        }

        Wire {
            top: &self.first,
            errors: Errors(self),
        }
        .serialize(serializer)
    }
}

/// Every error on its own line, in the order found.
impl fmt::Display for ErrorReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.first)?;
        for error in &self.rest {
            write!(f, "\n{error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ErrorReport {}

#[cfg(test)]
mod tests {
    use super::{Code, Error, ErrorReport, Pointer, Reason, Source};
    use core::num::NonZeroU64;
    use serde_json::json;

    fn at(pointer: &str) -> Pointer {
        Pointer::parse(pointer).unwrap()
    }

    fn number(n: u64) -> NonZeroU64 {
        NonZeroU64::new(n).unwrap()
    }

    const INVALID_INPUT: Code = Code::new("invalid_input");
    const STALE_REVISION: Code = Code::new("stale_revision");
    const DELTA_CONFLICT: Code = Code::new("delta_conflict");
    const TRANSITION_REFUSED: Code = Code::new("transition_refused");

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
            Reason::Internal => 12,
        }
    }

    #[test]
    fn all_lists_every_reason_once_in_order() {
        const COUNT: usize = 13;
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
                "internal",
            ]
        );
    }

    #[test]
    fn a_pointer_is_empty_or_starts_with_a_slash() {
        assert_eq!(Pointer::root().as_str(), "");
        assert_eq!(Pointer::parse("").unwrap(), Pointer::root());
        for valid in ["/", "/issue/title", "/a~0b/c~1d", "/0", "//"] {
            assert_eq!(at(valid).as_str(), valid);
            assert_eq!(at(valid).to_string(), valid);
        }
    }

    #[test]
    fn a_pointer_grows_by_one_escaped_token_at_a_time() {
        let root = Pointer::root();
        assert_eq!(root.join("issue").as_str(), "/issue");
        assert_eq!(root.join("issue").join("title").as_str(), "/issue/title");
        assert_eq!(root.join("a/b~c").as_str(), "/a~1b~0c");
        assert_eq!(root.join("~1").as_str(), "/~01");
        assert_eq!(root.join("").as_str(), "/");
        assert_eq!(root.join("goals").join_index(0).as_str(), "/goals/0");
        assert_eq!(root.join_index(12).as_str(), "/12");
        // Every pointer built this way is one that parse accepts as written.
        let built = root.join("x~/y").join_index(3).join("");
        assert_eq!(Pointer::parse(built.as_str()), Some(built));
    }

    #[test]
    fn a_pointer_out_of_the_form_of_rfc_6901_is_refused() {
        for invalid in ["issue", "issue/title", " /a", "/a~", "/a~2", "/~x", "~0"] {
            assert_eq!(Pointer::parse(invalid), None, "{invalid}");
        }
    }

    #[test]
    fn an_error_carries_its_code_message_pointer_and_reason_as_data() {
        let error = Error::new(
            INVALID_INPUT,
            "The issue has no title.",
            at("/issue/title"),
            Reason::Missing,
        );
        assert_eq!(error.code().id(), "invalid_input");
        assert_eq!(error.message(), "The issue has no title.");
        assert_eq!(error.pointer().as_str(), "/issue/title");
        assert_eq!(error.reason(), Reason::Missing);
        assert_eq!(error.pointers(), None);
        assert_eq!(error.expected(), None);
        assert_eq!(error.fix(), None);
        assert_eq!(error.location(), None);
    }

    #[test]
    fn display_names_the_code_the_reason_the_place_and_the_message() {
        let error = Error::new(
            INVALID_INPUT,
            "Unknown field.",
            at("/stpe"),
            Reason::UnknownField,
        );
        assert_eq!(
            error.to_string(),
            "invalid_input: unknown_field at /stpe: Unknown field."
        );
        let root = Error::new(INVALID_INPUT, "Not JSON.", Pointer::root(), Reason::Syntax);
        assert_eq!(
            root.to_string(),
            "invalid_input: syntax at the root of the input: Not JSON."
        );
    }

    #[test]
    fn it_is_a_standard_error_without_a_source() {
        let error: Box<dyn std::error::Error> = Box::new(Error::new(
            STALE_REVISION,
            "The revision is stale.",
            at("/revision"),
            Reason::Stale,
        ));
        assert!(error.source().is_none());
        assert_eq!(
            error.to_string(),
            "stale_revision: stale at /revision: The revision is stale."
        );
    }

    #[test]
    fn every_reason_serialises_as_its_identifier() {
        for reason in Reason::ALL {
            assert_eq!(serde_json::to_value(reason).unwrap(), json!(reason.id()));
        }
    }

    #[test]
    fn a_code_and_a_pointer_serialise_as_strings() {
        assert_eq!(
            serde_json::to_value(INVALID_INPUT).unwrap(),
            json!("invalid_input")
        );
        assert_eq!(serde_json::to_value(at("/a~1b")).unwrap(), json!("/a~1b"));
    }

    #[test]
    fn absent_fields_are_left_out_of_the_serialised_error() {
        let error = Error::new(
            INVALID_INPUT,
            "No problem.",
            at("/problem"),
            Reason::Missing,
        );
        assert_eq!(
            serde_json::to_value(&error).unwrap(),
            json!({
                "code": "invalid_input",
                "message": "No problem.",
                "pointer": "/problem",
                "reason": "missing",
            })
        );
    }

    #[test]
    fn empty_lists_are_given_as_none_not_left_out() {
        // ADR 0009 C2: absent means not given, [] means none (SPEC-0001 R13: no status is
        // allowed out of a terminal one).
        let error = Error::new(
            TRANSITION_REFUSED,
            "A done task does not change status.",
            at("/status"),
            Reason::State,
        )
        .with_pointers(Vec::<Pointer>::new())
        .with_expected(Vec::<serde_json::Value>::new());
        assert_eq!(error.pointers(), Some(&[][..]));
        assert_eq!(error.expected(), Some(&[][..]));
        assert_eq!(
            serde_json::to_value(&error).unwrap(),
            json!({
                "code": "transition_refused",
                "message": "A done task does not change status.",
                "pointer": "/status",
                "pointers": [],
                "reason": "state",
                "expected": [],
            })
        );
    }

    fn conflict() -> Error {
        Error::new(
            DELTA_CONFLICT,
            "Two operations change R2.",
            at("/operations/1"),
            Reason::Conflict,
        )
        .with_pointers([at("/operations/0"), at("/operations/1")])
        .with_expected([json!(1), json!("two")])
        .with_fix("removed")
        .with_location(Source::File {
            file: "specs/0001-artifact-model/spec.md".into(),
            line: number(42),
        })
    }

    #[test]
    fn an_error_serialises_every_field_of_c3_with_its_source() {
        let error = conflict();
        assert_eq!(
            error.pointers(),
            Some(&[at("/operations/0"), at("/operations/1")][..])
        );
        assert_eq!(error.expected(), Some(&[json!(1), json!("two")][..]));
        assert_eq!(error.fix(), Some(&json!("removed")));
        assert_eq!(
            serde_json::to_value(&error).unwrap(),
            json!({
                "code": "delta_conflict",
                "message": "Two operations change R2.",
                "pointer": "/operations/1",
                "pointers": ["/operations/0", "/operations/1"],
                "reason": "conflict",
                "expected": [1, "two"],
                "fix": "removed",
                "source": {"file": "specs/0001-artifact-model/spec.md", "line": 42},
            })
        );
    }

    #[test]
    fn each_form_of_source_serialises_with_its_own_fields() {
        let cases = [
            (
                Source::File {
                    file: "plan/items/TASK-0001.yaml".into(),
                    line: number(7),
                },
                json!({"file": "plan/items/TASK-0001.yaml", "line": 7}),
            ),
            (Source::Issue { issue: number(252) }, json!({"issue": 252})),
            (
                Source::Milestone {
                    milestone: number(3),
                },
                json!({"milestone": 3}),
            ),
        ];
        for (source, expected) in cases {
            let error = Error::new(INVALID_INPUT, "No title.", at("/title"), Reason::Missing)
                .with_location(source.clone());
            assert_eq!(error.location(), Some(&source));
            assert_eq!(serde_json::to_value(&error).unwrap()["source"], expected);
        }
    }

    #[test]
    fn a_report_needs_at_least_one_error() {
        assert!(ErrorReport::from_errors([]).is_none());
    }

    fn unknown_then_missing() -> (Error, Error) {
        (
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
        )
    }

    #[test]
    fn a_report_keeps_its_first_error_at_the_top_level() {
        let (first, second) = unknown_then_missing();
        let report = ErrorReport::from_errors([first.clone(), second.clone()]).unwrap();
        assert_eq!(report.first(), &first);
        assert_eq!(
            report.errors().collect::<Vec<_>>(),
            [&first, &second],
            "every error, in the order found"
        );
        let value = serde_json::to_value(&report).unwrap();
        assert_eq!(
            value,
            json!({
                "code": "invalid_input",
                "message": "Unknown field.",
                "pointer": "/problema",
                "reason": "unknown_field",
                "fix": "problem",
                "errors": [
                    {
                        "code": "invalid_input",
                        "message": "Unknown field.",
                        "pointer": "/problema",
                        "reason": "unknown_field",
                        "fix": "problem",
                    },
                    {
                        "code": "invalid_input",
                        "message": "No problem.",
                        "pointer": "/problem",
                        "reason": "missing",
                    },
                ],
            })
        );
        let mut top = value.as_object().unwrap().clone();
        top.remove("errors");
        assert_eq!(serde_json::Value::Object(top), value["errors"][0]);
    }

    #[test]
    fn a_report_of_one_error_lists_it_once_in_errors() {
        let only = Error::new(STALE_REVISION, "Stale.", at("/revision"), Reason::Stale);
        let report = ErrorReport::from_errors([only.clone()]).unwrap();
        assert_eq!(report.errors().collect::<Vec<_>>(), [&only]);
        assert_eq!(
            serde_json::to_value(&report).unwrap()["errors"],
            json!([serde_json::to_value(&only).unwrap()])
        );
    }

    #[test]
    fn a_full_report_serialises_its_fields_in_the_order_of_c3() {
        let report = ErrorReport::from_errors([conflict()]).unwrap();
        let error = concat!(
            r#""code":"delta_conflict","message":"Two operations change R2.","#,
            r#""pointer":"/operations/1","pointers":["/operations/0","/operations/1"],"#,
            r#""reason":"conflict","expected":[1,"two"],"fix":"removed","#,
            r#""source":{"file":"specs/0001-artifact-model/spec.md","line":42}"#,
        );
        assert_eq!(
            serde_json::to_string(&report).unwrap(),
            format!("{{{error},\"errors\":[{{{error}}}]}}")
        );
    }

    #[test]
    fn a_report_is_a_standard_error_listing_every_error_on_its_own_line() {
        let (first, second) = unknown_then_missing();
        let report: Box<dyn std::error::Error> =
            Box::new(ErrorReport::from_errors([first, second]).unwrap());
        assert!(report.source().is_none());
        assert_eq!(
            report.to_string(),
            "invalid_input: unknown_field at /problema: Unknown field.\n\
             invalid_input: missing at /problem: No problem."
        );
    }
}
