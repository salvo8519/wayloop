//! Closed enums and static identifiers of the contracts, each written once.
//!
//! A closed enum of the contract (a reason, a format, an exit code, a step) has one spelling per
//! value, written once in [`contract_enum!`]: its identifier `id()`, its serialised form and the
//! value of its JSON Schema all read it, so they cannot differ. The doc comments of the enum
//! and of each value are the descriptions of its schema. A static identifier (a code, a check,
//! the reason of a default) is a newtype of a `&'static str`, written once in [`static_id!`].
//!
//! Both macros write `Serialize` and `JsonSchema` by hand, through paths of this crate, so a
//! crate that uses them needs neither `serde` nor `schemars` among its own dependencies.

use schemars::Schema;
use serde_json::{Map, Value};

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

/// The schema of a static identifier: a described string. Called by [`static_id!`] only.
#[doc(hidden)]
#[must_use]
pub fn string_schema(description: &str) -> Schema {
    let mut schema = Map::new();
    schema.insert("description".to_owned(), description.into());
    schema.insert("type".to_owned(), "string".into());
    Schema::from(schema)
}

/// A closed enum of the contract, from its doc comments and one spelling per value.
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
/// assert_eq!(Light::ALL, [Light::Red, Light::Green]);
/// assert_eq!(Light::Green.id(), "green");
/// assert_eq!(Light::from_id("red"), Some(Light::Red));
/// assert_eq!(Light::Red.to_string(), "red");
/// ```
///
/// The enum derives `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `PartialOrd` and `Ord`,
/// with the values in the order written. It gets `ALL`, every value in that order; `id()`, its
/// spelling; `from_id()`, the value of a spelling; `Display`, which writes `id()`; `Serialize`,
/// which writes `id()` as a string; and `JsonSchema`, whose descriptions are the doc comments.
/// Only doc comments are accepted on the enum and on its values.
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
        $(#[doc = $doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        $vis enum $name {
            $(
                $(#[doc = $value_doc])*
                $variant,
            )+
        }

        impl $name {
            /// Every value, in the order of declaration.
            pub const ALL: [Self; { let ids: &[&str] = &[$($id),+]; ids.len() }] =
                [$(Self::$variant),+];

            /// The spelling of the value in the contract: its serialised form and its value in
            /// the JSON Schema.
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

        impl $crate::ident::__private::serde::Serialize for $name {
            fn serialize<S: $crate::ident::__private::serde::Serializer>(
                &self,
                serializer: S,
            ) -> ::core::result::Result<S::Ok, S::Error> {
                serializer.serialize_str(self.id())
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

/// A static identifier of the contract: a newtype of a `snake_case` `&'static str`, written by
/// constants of the engine.
///
/// ```
/// wayloop_schemas::static_id! {
///     /// A check of the engine.
///     pub struct Probe;
/// }
/// const SCHEMA: Probe = Probe::new("schema");
/// assert_eq!(SCHEMA.id(), "schema");
/// assert_eq!(SCHEMA.to_string(), "schema");
/// ```
///
/// The type derives `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `PartialOrd`, `Ord` and `Hash`,
/// and gets `new()`, `id()`, `Display`, `Serialize` as its string, and `JsonSchema`, a string
/// whose description is the doc comment.
#[macro_export]
macro_rules! static_id {
    (
        $(#[doc = $doc:literal])*
        $vis:vis struct $name:ident;
    ) => {
        $(#[doc = $doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        $vis struct $name(&'static str);

        impl $name {
            /// The identifier `id`, in `snake_case`.
            #[must_use]
            pub const fn new(id: &'static str) -> Self {
                Self(id)
            }

            /// The identifier as written in the contract.
            #[must_use]
            pub const fn id(self) -> &'static str {
                self.0
            }
        }

        impl ::core::fmt::Display for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.write_str(self.0)
            }
        }

        impl $crate::ident::__private::serde::Serialize for $name {
            fn serialize<S: $crate::ident::__private::serde::Serializer>(
                &self,
                serializer: S,
            ) -> ::core::result::Result<S::Ok, S::Error> {
                serializer.serialize_str(self.0)
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
