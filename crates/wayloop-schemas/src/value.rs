//! The value forms of the contracts (ADR 0030 G1): one type per form, used wherever the form
//! appears.
//!
//! - A text in a named format ([`BriefId`], [`ProjectPath`], [`LanguageTag`],
//!   [`TrackerHandle`]) is written with [`formatted_text!`](crate::formatted_text!): a string
//!   whose schema carries the format of [`Format`], checked by the core.
//! - An integer from 1 ([`IssueNumber`], [`MilestoneNumber`], [`Line`], [`AdrNumber`],
//!   [`SchemaVersion`]) is written with [`positive_integer!`](crate::positive_integer!): it is
//!   read by value, so `3.0` is `3` (see [`crate::number`]), as the interpreter reads it.
//! - [`Text`] is a prose text of at least one character.
//!
//! Their schemas are inlined where they are used, with the description of the member that uses
//! them; the description of the type is kept for the elements of a list. A value read into
//! these types has passed its schema first (ADR 0030 G1): reading checks the type of the value,
//! not its format, which the interpreter has checked with every error in one pass.

use core::num::NonZeroU64;

use schemars::Schema;
use serde_json::{Map, Value};

use crate::Format;
use crate::number::{integer, is_safe};

#[doc(hidden)]
pub mod __private {
    pub use schemars;
    pub use serde;
    pub use serde_json;
}

/// The schema of an integer from 1, up to `max` when given, with its `description`.
#[doc(hidden)]
#[must_use]
pub fn positive_schema(description: &str, max: Option<u64>) -> Schema {
    let mut schema = Map::new();
    schema.insert("description".to_owned(), description.into());
    schema.insert("minimum".to_owned(), 1.into());
    if let Some(max) = max {
        schema.insert("maximum".to_owned(), max.into());
    }
    schema.insert("type".to_owned(), "integer".into());
    Schema::from(schema)
}

/// The text of a JSON string; `None` for any other value.
#[doc(hidden)]
#[must_use]
pub fn text(value: &Value) -> Option<String> {
    value.as_str().map(str::to_owned)
}

/// An integer from 1 to [`SAFE_INTEGER`](crate::number::SAFE_INTEGER), and to `max` when given,
/// read by value: `3.0` is `3`; `None` for any other value, as the interpreter refuses an
/// integer beyond that range.
#[doc(hidden)]
#[must_use]
pub fn positive(value: &Value, max: Option<u64>) -> Option<NonZeroU64> {
    integer(value)
        .filter(|n| is_safe(*n))
        .and_then(|n| u64::try_from(n).ok())
        .filter(|n| max.is_none_or(|max| *n <= max))
        .and_then(NonZeroU64::new)
}

/// A text in a named format of the contracts, from its doc comment and its [`Format`].
///
/// ```
/// wayloop_schemas::formatted_text! {
///     /// A path of the project.
///     pub struct Where = wayloop_schemas::Format::ProjectPath;
/// }
/// let read: Where = serde_json::from_str(r#""plan/a.yaml""#).unwrap();
/// assert_eq!(read.as_str(), "plan/a.yaml");
/// assert_eq!(serde_json::to_string(&Where::new("b")).unwrap(), r#""b""#);
/// ```
///
/// The type derives `Debug`, `Clone`, `PartialEq`, `Eq`, `PartialOrd`, `Ord` and `Hash`, and gets
/// `new()`, `as_str()`, `Display`, `Serialize` and `Deserialize` as its string, and an inlined
/// `JsonSchema`: a string in the format, whose description is the doc comment.
#[macro_export]
macro_rules! formatted_text {
    (
        $(#[doc = $doc:literal])*
        $vis:vis struct $name:ident = $format:expr;
    ) => {
        $(#[doc = $doc])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        $vis struct $name(::std::string::String);

        impl $name {
            /// The value `text`, as written; its format is checked by the schema.
            #[must_use]
            pub fn new(text: impl ::core::convert::Into<::std::string::String>) -> Self {
                Self(text.into())
            }

            /// The text as written.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        $crate::__value_traits!($name, $crate::value::text, "expected a string");

        $crate::__inline_schema!(
            $name,
            $crate::contract::rules::formatted($format, concat!($($doc, "\n"),*))
        );
    };
}

/// An integer from 1 of the contracts, from its doc comment, with an optional inclusive upper
/// bound written `<= <max>`, a plain decimal literal such as `10`: the message of a refusal
/// copies it as written.
///
/// ```
/// wayloop_schemas::positive_integer! {
///     /// A count of apples, from 1.
///     pub struct Apples;
/// }
/// let read: Apples = serde_json::from_str("3.0").unwrap();
/// assert_eq!(read.get().get(), 3);
/// assert_eq!(Apples::MAX, None);
/// let refused = serde_json::from_str::<Apples>("0").unwrap_err().to_string();
/// assert!(refused.starts_with("expected an integer from 1"), "{refused}");
/// assert!(serde_json::from_str::<Apples>("1.5").is_err());
///
/// wayloop_schemas::positive_integer! {
///     /// A score, from 1 to 5.
///     pub struct Score <= 5;
/// }
/// assert!(serde_json::from_str::<Score>("5").is_ok());
/// let refused = serde_json::from_str::<Score>("6").unwrap_err().to_string();
/// assert!(refused.starts_with("expected an integer from 1 to 5"), "{refused}");
/// ```
///
/// A bound below 1 is refused when the type is compiled:
///
/// ```compile_fail
/// wayloop_schemas::positive_integer! {
///     /// Nothing fits.
///     pub struct Never <= 0;
/// }
/// ```
///
/// The type derives `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `PartialOrd`, `Ord` and
/// `Hash`, and gets `MAX`, its upper bound when it has one; `new()`, `get()`, `Display`,
/// `Serialize` as its number, `Deserialize` that reads an integer by value within its bounds,
/// and an inlined `JsonSchema`: an integer from 1, with its `maximum` when bounded, whose
/// description is the doc comment.
#[macro_export]
macro_rules! positive_integer {
    (@define [$($doc:literal),*] $vis:vis $name:ident, $max:expr, $expected:expr) => {
        $(#[doc = $doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        $vis struct $name(::core::num::NonZeroU64);

        impl $name {
            /// The number `n`.
            #[must_use]
            pub const fn new(n: ::core::num::NonZeroU64) -> Self {
                Self(n)
            }

            /// The number.
            #[must_use]
            pub const fn get(self) -> ::core::num::NonZeroU64 {
                self.0
            }

            /// The inclusive upper bound; `None` when the number has none.
            pub const MAX: ::core::option::Option<u64> = $max;

            /// The number `value` holds, read by value, within the bounds of the type.
            fn read(
                value: &$crate::value::__private::serde_json::Value,
            ) -> ::core::option::Option<::core::num::NonZeroU64> {
                $crate::value::positive(value, Self::MAX)
            }
        }

        $crate::__value_traits!($name, $name::read, $expected);

        $crate::__inline_schema!(
            $name,
            $crate::value::positive_schema(concat!($($doc, "\n"),*), $name::MAX)
        );
    };
    (
        $(#[doc = $doc:literal])*
        $vis:vis struct $name:ident;
    ) => {
        $crate::positive_integer!(
            @define [$($doc),*] $vis $name,
            ::core::option::Option::None,
            "expected an integer from 1"
        );
    };
    (
        $(#[doc = $doc:literal])*
        $vis:vis struct $name:ident <= $max:literal;
    ) => {
        const _: () = ::core::assert!($max >= 1, "the bound of a positive integer is at least 1");

        $crate::positive_integer!(
            @define [$($doc),*] $vis $name,
            ::core::option::Option::Some($max),
            ::core::concat!("expected an integer from 1 to ", ::core::stringify!($max))
        );
    };
}

/// `Display` and `Serialize` of a value form, those of the value it wraps; and `Deserialize`,
/// which reads a JSON value and keeps it when `read` gives the wrapped value, refusing it with
/// `expected` otherwise.
#[doc(hidden)]
#[macro_export]
macro_rules! __value_traits {
    ($name:ident, $read:path, $expected:expr) => {
        impl ::core::fmt::Display for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                ::core::fmt::Display::fmt(&self.0, f)
            }
        }

        impl $crate::value::__private::serde::Serialize for $name {
            fn serialize<S: $crate::value::__private::serde::Serializer>(
                &self,
                serializer: S,
            ) -> ::core::result::Result<S::Ok, S::Error> {
                $crate::value::__private::serde::Serialize::serialize(&self.0, serializer)
            }
        }

        impl<'de> $crate::value::__private::serde::Deserialize<'de> for $name {
            fn deserialize<D: $crate::value::__private::serde::Deserializer<'de>>(
                deserializer: D,
            ) -> ::core::result::Result<Self, D::Error> {
                let value: $crate::value::__private::serde_json::Value =
                    $crate::value::__private::serde::Deserialize::deserialize(deserializer)?;
                $read(&value).map(Self).ok_or_else(|| {
                    <D::Error as $crate::value::__private::serde::de::Error>::custom($expected)
                })
            }
        }
    };
}

/// `JsonSchema` of a value form: inlined where it is used, with the `schema` given.
#[doc(hidden)]
#[macro_export]
macro_rules! __inline_schema {
    ($name:ident, $schema:expr) => {
        impl $crate::value::__private::schemars::JsonSchema for $name {
            fn inline_schema() -> bool {
                true
            }

            fn schema_name() -> ::std::borrow::Cow<'static, str> {
                ::std::borrow::Cow::Borrowed(stringify!($name))
            }

            fn json_schema(
                _: &mut $crate::value::__private::schemars::SchemaGenerator,
            ) -> $crate::value::__private::schemars::Schema {
                $schema
            }
        }
    };
}

crate::formatted_text! {
    /// Identifier of a brief: `BRIEF-` and four digits, or five or more without a leading zero,
    /// for example `BRIEF-0001` (SPEC-0001, Identifiers).
    pub struct BriefId = Format::BriefId;
}

crate::formatted_text! {
    /// Identifier of a council report: `COUNCIL-` and four digits, or five or more without a
    /// leading zero, for example `COUNCIL-0001`.
    pub struct CouncilId = Format::CouncilId;
}

crate::formatted_text! {
    /// Identifier of a brief or of a council report, the subject of a gate issue:
    /// `BRIEF-<seq>` or `COUNCIL-<seq>`, for example `BRIEF-0001` or `COUNCIL-0001`.
    pub struct BriefOrCouncilId = Format::BriefOrCouncilId;
}

crate::formatted_text! {
    /// Identifier of a gate issue: `GATE-` and four digits, or five or more without a leading
    /// zero, for example `GATE-0001`.
    pub struct GateId = Format::GateId;
}

crate::formatted_text! {
    /// Identifier of a milestone: `M` and a number from 0 without a leading zero, for example
    /// `M0` or `M12`; it records the order of creation, not of delivery.
    pub struct MilestoneId = Format::MilestoneId;
}

crate::formatted_text! {
    /// Identifier of an epic: `EPIC-` and four digits, or five or more without a leading zero,
    /// for example `EPIC-0003`.
    pub struct EpicId = Format::EpicId;
}

crate::formatted_text! {
    /// Identifier of a user story: `STORY-` and four digits, or five or more without a leading
    /// zero, for example `STORY-0042`.
    pub struct StoryId = Format::StoryId;
}

crate::formatted_text! {
    /// Identifier of a living specification: `SPEC-` and four digits, or five or more without a
    /// leading zero, for example `SPEC-0003`.
    pub struct SpecId = Format::SpecId;
}

crate::formatted_text! {
    /// Identifier of a task: `TASK-` and four digits, or five or more without a leading zero,
    /// for example `TASK-0107`.
    pub struct TaskId = Format::TaskId;
}

crate::formatted_text! {
    /// Identifier of a requirement of a living specification: `R` and a number from 1 without
    /// a leading zero, for example `R2`.
    pub struct RequirementId = Format::RequirementId;
}

crate::formatted_text! {
    /// Identifier of an acceptance criterion within its artifact: `AC` and a number from 1
    /// without a leading zero, for example `AC1`.
    pub struct CriterionId = Format::CriterionId;
}

crate::formatted_text! {
    /// A calendar date, `YYYY-MM-DD` (RFC 3339 `full-date`), for example `2027-03-31`.
    pub struct Date = Format::Date;
}

crate::formatted_text! {
    /// Path of a repository file relative to the root of the project, in one spelling: segments
    /// joined by `/`, none empty, `.` or `..`; no leading `/`, no `\` and no drive letter, so
    /// that it never leaves the project.
    pub struct ProjectPath = Format::ProjectPath;
}

crate::formatted_text! {
    /// A BCP 47 language tag: a language of 2 or 3 letters, then optionally a script of 4
    /// letters, a region of 2 letters or 3 digits, and variants, joined by hyphens, for example
    /// `it`, `en-GB` or `zh-Hant-TW`.
    pub struct LanguageTag = Format::LanguageTag;
}

crate::formatted_text! {
    /// A tracker handle: `@` followed by the user name of the tracker, 1 to 39 letters, digits
    /// or single hyphens, not starting or ending with a hyphen, for example `@example-owner`.
    pub struct TrackerHandle = Format::TrackerHandle;
}

crate::positive_integer! {
    /// Number of a tracker issue, from 1.
    pub struct IssueNumber;
}

crate::positive_integer! {
    /// Number of a tracker milestone, from 1; a sequence apart from issues.
    pub struct MilestoneNumber;
}

crate::positive_integer! {
    /// Line of a file, counted from 1.
    pub struct Line;
}

crate::positive_integer! {
    /// Number of an ADR of the project, from 1.
    pub struct AdrNumber;
}

crate::positive_integer! {
    /// Version of a schema, from 1.
    pub struct SchemaVersion;
}

/// The schema of a string of at least one character, with its `description`.
#[doc(hidden)]
#[must_use]
pub fn non_empty_schema(description: &str) -> Schema {
    let mut schema = crate::ident::string_schema(description);
    schema.insert("minLength".to_owned(), 1.into());
    schema
}

/// A string of at least one character, with no format, from its doc comment: the type gets
/// `new()`, `as_str()`, `Display`, `Serialize` and `Deserialize` as its string, and an inlined
/// `JsonSchema` whose description is the doc comment.
macro_rules! non_empty_text {
    (
        $(#[doc = $doc:literal])*
        pub struct $name:ident;
    ) => {
        $(#[doc = $doc])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            /// The value `text`, as written; its length is checked by the schema.
            #[must_use]
            pub fn new(text: impl Into<String>) -> Self {
                Self(text.into())
            }

            /// The text as written.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        crate::__value_traits!($name, text, "expected a string");

        crate::__inline_schema!($name, non_empty_schema(concat!($($doc, "\n"),*)));
    };
}

// SPEC-0001 R6: an optional text is left out until it has a value, never written empty.
non_empty_text! {
    /// A prose text of at least one character, in the language of the document.
    pub struct Text;
}

non_empty_text! {
    /// A name of the project policy, an identifier rather than prose: a type of commits, a
    /// scope or an area, spelt as the policy spells it, of at least one character.
    pub struct PolicyName;
}

/// The schema of an object whose members the contract does not describe, with its
/// `description`: a document of another schema. It stays open in an input too, since it declares
/// no member (ADR 0030 G2, as amended for #256); the core checks its content. A map is not one:
/// it keeps the schema of its values (#307).
#[doc(hidden)]
#[must_use]
pub fn free_object_schema(description: &str) -> Schema {
    let mut schema = Map::new();
    schema.insert("description".to_owned(), description.into());
    schema.insert("type".to_owned(), "object".into());
    Schema::from(schema)
}

#[cfg(test)]
mod tests {
    use super::{BriefId, IssueNumber, Line, PolicyName, ProjectPath, Text};
    use core::num::NonZeroU64;
    use schemars::JsonSchema;
    use serde_json::json;

    #[test]
    fn a_positive_integer_is_read_by_value_and_written_as_a_number() {
        let read: IssueNumber = serde_json::from_value(json!(12.0)).unwrap();
        assert_eq!(read.get().get(), 12);
        assert_eq!(read.to_string(), "12");
        assert_eq!(serde_json::to_value(read).unwrap(), json!(12));
        let beyond = json!(crate::number::SAFE_INTEGER + 1);
        let largest: Line = serde_json::from_value(json!(crate::number::SAFE_INTEGER)).unwrap();
        assert_eq!(largest.get().get(), 9_007_199_254_740_991);
        for refused in [
            json!(0),
            json!(-1),
            json!(1.5),
            json!("1"),
            json!(null),
            beyond,
        ] {
            assert!(
                serde_json::from_value::<Line>(refused.clone()).is_err(),
                "{refused}"
            );
        }
        let two = Line::new(NonZeroU64::new(2).unwrap());
        assert_eq!(two.get().get(), 2);
    }

    #[test]
    fn a_formatted_text_reads_and_writes_its_string() {
        let read: BriefId = serde_json::from_value(json!("BRIEF-0001")).unwrap();
        assert_eq!(read.as_str(), "BRIEF-0001");
        assert_eq!(read.to_string(), "BRIEF-0001");
        assert_eq!(serde_json::to_value(&read).unwrap(), json!("BRIEF-0001"));
        assert_eq!(ProjectPath::new("a/b").as_str(), "a/b");
        assert!(serde_json::from_value::<BriefId>(json!(1)).is_err());
    }

    #[test]
    fn value_forms_are_inlined_with_their_schema() {
        assert!(BriefId::inline_schema() && IssueNumber::inline_schema() && Text::inline_schema());
        let mut generator = schemars::SchemaGenerator::default();
        let schema = ProjectPath::json_schema(&mut generator).to_value();
        assert_eq!(schema["format"], "project-path");
        assert_eq!(schema["type"], "string");
        assert_eq!(Line::schema_name(), "Line");
        let schema = Line::json_schema(&mut generator).to_value();
        assert_eq!(
            (schema["type"].clone(), schema["minimum"].clone()),
            (json!("integer"), json!(1))
        );
        let text = Text::json_schema(&mut generator).to_value();
        assert_eq!(text["minLength"], 1);
        let name = PolicyName::json_schema(&mut generator).to_value();
        assert_eq!(
            (name["minLength"].clone(), name["type"].clone()),
            (json!(1), json!("string"))
        );
        assert!(name["description"].as_str().unwrap().contains("identifier"));
        assert_eq!(PolicyName::new("feat").as_str(), "feat");
        let read: Text = serde_json::from_value(json!("x")).unwrap();
        assert_eq!(read, Text::new("x"));
        assert_eq!(read.as_str(), "x");
        assert_eq!(serde_json::to_value(&read).unwrap(), json!("x"));
    }
}
