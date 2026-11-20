//! Closed enums and static identifiers of the contracts, each written once.
//!
//! A closed enum has one spelling per value, written once in [`closed_enum!`]: `id()`,
//! `from_id()` and `Display` read it, and two values with the same spelling do not compile. A
//! closed enum of a contract (a reason, a format) is written with [`contract_enum!`]: its
//! serialised form and the values of its JSON Schema read the same spelling, and the doc
//! comments of the enum and of each value are the descriptions of its schema. A static
//! identifier (a code, a check, the reason of a default) is written once in [`static_id!`]:
//! constants of the engine borrow their text, values read from a document own it.
//!
//! The macros write `Serialize`, `Deserialize` and `JsonSchema` by hand, through paths of this
//! crate, so a crate that uses them needs neither `serde` nor `schemars` among its own
//! dependencies.

use schemars::Schema;
use serde_json::{Map, Value};

/// Whether no spelling of `ids` appears twice; [`closed_enum!`] checks it at compile time.
#[doc(hidden)]
#[must_use]
pub const fn distinct(ids: &[&str]) -> bool {
    match ids {
        [] => true,
        [first, rest @ ..] => !contains(rest, first) && distinct(rest),
    }
}

/// Whether `ids` holds `id`, in a constant context.
const fn contains(ids: &[&str], id: &str) -> bool {
    match ids {
        [] => false,
        [first, rest @ ..] => same(first.as_bytes(), id.as_bytes()) || contains(rest, id),
    }
}

/// Whether `a` and `b` are the same bytes, in a constant context.
const fn same(a: &[u8], b: &[u8]) -> bool {
    match (a, b) {
        ([], []) => true,
        ([x, a @ ..], [y, b @ ..]) => *x == *y && same(a, b),
        _ => false,
    }
}

#[doc(hidden)]
pub mod __private {
    pub use schemars;
    pub use serde;
}

/// The schema of a closed enum: a `oneOf` of documented string constants, which the generator
/// of [`contract`](crate::contract) turns into an `enum` with the meaning of each value in the
/// description, as it does for a derived enum. Called by [`contract_enum!`] only.
#[doc(hidden)]
#[must_use]
pub fn enum_schema(description: &str, values: &[(&str, &str)]) -> Schema {
    let choices = values
        .iter()
        .map(|(id, meaning)| {
            let mut choice = Map::new();
            choice.insert("const".to_owned(), (*id).into());
            choice.insert("description".to_owned(), (*meaning).into());
            choice.insert("type".to_owned(), "string".into());
            Value::Object(choice)
        })
        .collect::<Vec<_>>();
    let mut schema = Map::new();
    schema.insert("description".to_owned(), description.into());
    schema.insert("oneOf".to_owned(), choices.into());
    Schema::from(schema)
}

/// The schema of a string with its `description`, the base of every string schema written by
/// hand.
#[doc(hidden)]
#[must_use]
pub fn string_schema(description: &str) -> Schema {
    let mut schema = Map::new();
    schema.insert("description".to_owned(), description.into());
    schema.insert("type".to_owned(), "string".into());
    Schema::from(schema)
}

/// A closed enum, from its doc comments and one spelling per value.
///
/// ```
/// wayloop_schemas::closed_enum! {
///     /// Colour of a light.
///     pub enum Light {
///         /// Stop.
///         Red = "red",
///         /// Go.
///         Green = "green",
///     }
/// }
/// assert_eq!(Light::ALL, [Light::Red, Light::Green]);
/// assert_eq!(Light::Green.id(), "green");
/// assert_eq!(Light::from_id("red"), Some(Light::Red));
/// assert_eq!(Light::Red.to_string(), "red");
/// ```
///
/// Two values with the same spelling do not compile:
///
/// ```compile_fail
/// wayloop_schemas::closed_enum! {
///     /// Colour of a light.
///     pub enum Light {
///         /// Stop.
///         Red = "red",
///         /// Also stop.
///         Crimson = "red",
///     }
/// }
/// ```
///
/// The enum derives `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `PartialOrd` and `Ord`,
/// with the values in the order written. It gets `ALL`, every value in that order; `id()`, its
/// spelling; `from_id()`, the value of a spelling; and `Display`, which writes `id()`. Only doc
/// comments are accepted on the enum and on its values.
#[macro_export]
macro_rules! closed_enum {
    (
        $(#[doc = $doc:literal])*
        $vis:vis enum $name:ident {
            $(
                $(#[doc = $value_doc:literal])*
                $variant:ident = $id:literal
            ),+ $(,)?
        }
    ) => {
        $(#[doc = $doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        $vis enum $name {
            $(
                $(#[doc = $value_doc])*
                $variant,
            )+
        }

        const _: () = assert!(
            $crate::ident::distinct(&[$($id),+]),
            "two values of the enum have the same spelling"
        );

        impl $name {
            /// Every value, in the order of declaration.
            pub const ALL: [Self; { let ids: &[&str] = &[$($id),+]; ids.len() }] =
                [$(Self::$variant),+];

            /// The spelling of the value.
            #[must_use]
            pub const fn id(self) -> &'static str {
                match self {
                    $(Self::$variant => $id,)+
                }
            }

            /// The value spelt `id`; `None` when no value has that spelling.
            #[must_use]
            pub fn from_id(id: &str) -> ::core::option::Option<Self> {
                Self::ALL.into_iter().find(|value| value.id() == id)
            }
        }

        impl ::core::fmt::Display for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.write_str(self.id())
            }
        }
    };
}

/// A closed enum of a contract: a [`closed_enum!`] whose spelling is also its serialised form
/// and the values of its JSON Schema.
///
/// ```
/// wayloop_schemas::contract_enum! {
///     /// Colour of a light.
///     pub enum Light {
///         /// Stop.
///         Red = "red",
///         /// Go.
///         Green = "green",
///     }
/// }
/// assert_eq!(serde_json::to_string(&Light::Green).unwrap(), r#""green""#);
/// assert_eq!(serde_json::from_str::<Light>(r#""red""#).unwrap(), Light::Red);
/// assert!(serde_json::from_str::<Light>(r#""blue""#).is_err());
/// ```
///
/// On top of [`closed_enum!`] the enum gets `Serialize`, which writes `id()` as a string;
/// `Deserialize`, which reads a spelling and refuses any other text; and `JsonSchema`, whose
/// descriptions are the doc comments. The type reads documents of its own version of the
/// contract: a client that reads a newer version treats the member as a string and accepts
/// values it does not know (ADR 0009 C5).
#[macro_export]
macro_rules! contract_enum {
    (
        $(#[doc = $doc:literal])*
        $vis:vis enum $name:ident {
            $(
                $(#[doc = $value_doc:literal])*
                $variant:ident = $id:literal
            ),+ $(,)?
        }
    ) => {
        $crate::closed_enum! {
            $(#[doc = $doc])*
            $vis enum $name {
                $(
                    $(#[doc = $value_doc])*
                    $variant = $id,
                )+
            }
        }

        impl $crate::ident::__private::serde::Serialize for $name {
            fn serialize<S: $crate::ident::__private::serde::Serializer>(
                &self,
                serializer: S,
            ) -> ::core::result::Result<S::Ok, S::Error> {
                serializer.serialize_str(self.id())
            }
        }

        impl<'de> $crate::ident::__private::serde::Deserialize<'de> for $name {
            fn deserialize<D: $crate::ident::__private::serde::Deserializer<'de>>(
                deserializer: D,
            ) -> ::core::result::Result<Self, D::Error> {
                const IDS: &[&str] = &[$($id),+];
                let id = <::std::string::String as $crate::ident::__private::serde::Deserialize>
                    ::deserialize(deserializer)?;
                Self::from_id(&id).ok_or_else(|| {
                    <D::Error as $crate::ident::__private::serde::de::Error>::unknown_variant(
                        &id, IDS,
                    )
                })
            }
        }

        impl $crate::ident::__private::schemars::JsonSchema for $name {
            fn schema_name() -> ::std::borrow::Cow<'static, str> {
                ::std::borrow::Cow::Borrowed(stringify!($name))
            }

            fn json_schema(
                _: &mut $crate::ident::__private::schemars::SchemaGenerator,
            ) -> $crate::ident::__private::schemars::Schema {
                $crate::ident::enum_schema(
                    concat!($($doc, "\n"),*),
                    &[$(($id, concat!($($value_doc, "\n"),*))),+],
                )
            }
        }
    };
}

/// A static identifier of a contract: a newtype of a text that the constants of the engine
/// borrow and a value read from a document owns.
///
/// ```
/// wayloop_schemas::static_id! {
///     /// A check of the engine.
///     pub struct Probe;
/// }
/// const SCHEMA: Probe = Probe::new("schema");
/// assert_eq!(SCHEMA.id(), "schema");
/// assert_eq!(SCHEMA.to_string(), "schema");
/// let read: Probe = serde_json::from_str(r#""links""#).unwrap();
/// assert_eq!(read.id(), "links");
/// ```
///
/// The type derives `Debug`, `Clone`, `PartialEq`, `Eq`, `PartialOrd`, `Ord` and `Hash`, and
/// gets `new()`, a constant constructor; `id()`; `Display`; `Serialize` and `Deserialize` as
/// its string; and `JsonSchema`, a string whose description is the doc comment. The macro
/// checks no spelling: the type documents the convention its values follow.
#[macro_export]
macro_rules! static_id {
    (
        $(#[doc = $doc:literal])*
        $vis:vis struct $name:ident;
    ) => {
        $(#[doc = $doc])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        $vis struct $name(::std::borrow::Cow<'static, str>);

        impl $name {
            /// The identifier `id`, borrowed: usable in a constant.
            #[must_use]
            pub const fn new(id: &'static str) -> Self {
                Self(::std::borrow::Cow::Borrowed(id))
            }

            /// The identifier as written in the contract.
            #[must_use]
            pub fn id(&self) -> &str {
                &self.0
            }
        }

        impl ::core::fmt::Display for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl $crate::ident::__private::serde::Serialize for $name {
            fn serialize<S: $crate::ident::__private::serde::Serializer>(
                &self,
                serializer: S,
            ) -> ::core::result::Result<S::Ok, S::Error> {
                serializer.serialize_str(&self.0)
            }
        }

        impl<'de> $crate::ident::__private::serde::Deserialize<'de> for $name {
            fn deserialize<D: $crate::ident::__private::serde::Deserializer<'de>>(
                deserializer: D,
            ) -> ::core::result::Result<Self, D::Error> {
                <::std::string::String as $crate::ident::__private::serde::Deserialize>
                    ::deserialize(deserializer)
                    .map(|id| Self(::std::borrow::Cow::Owned(id)))
            }
        }

        impl $crate::ident::__private::schemars::JsonSchema for $name {
            fn schema_name() -> ::std::borrow::Cow<'static, str> {
                ::std::borrow::Cow::Borrowed(stringify!($name))
            }

            fn json_schema(
                _: &mut $crate::ident::__private::schemars::SchemaGenerator,
            ) -> $crate::ident::__private::schemars::Schema {
                $crate::ident::string_schema(concat!($($doc, "\n"),*))
            }
        }
    };
}
