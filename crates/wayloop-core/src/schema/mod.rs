//! Interpreter of the subset of JSON Schema (draft 2020-12) that the contracts of Wayloop use
//! (SPEC-0001 R1, R12; plan of SPEC-0001, D5; ADR 0009 C3).
//!
//! The schemas of the contracts are derived from the types of `wayloop-schemas`, inside this
//! subset (ADR 0030); a named format is written only through `wayloop_schemas::Format`, which
//! the format checker matches exhaustively. The core reads them with this interpreter, which
//! reports every error of a document in one pass, each with its RFC 6901 pointer and the
//! reason of ADR 0009 C3, instead of the generic messages of a validator crate.
//!
//! # The subset
//!
//! A schema is an object. Every keyword it uses must be listed here for its place; any other
//! keyword, or a listed one in the wrong place, is refused when the schema is loaded
//! ([`LoadError::Unsupported`]), so that no rule of a schema is silently ignored.
//!
//! | Keyword | Meaning | Reason of the error |
//! | --- | --- | --- |
//! | `type` | One of `boolean`, `integer`, `string`, `array`, `object`; an integer is a number without a fraction, so `2.0` is one | `type`; the other keywords of the value are not checked |
//! | `enum`, `const` | Accepted values: strings, integers, booleans or `null`; numbers compare by value, so `4364.0` equals `4364` | `enum`, with the accepted values in `expected` |
//! | `minLength`, `maxLength` | Bounds of a string, in Unicode code points | `length` |
//! | `minItems`, `maxItems` | Bounds of an array | `length` |
//! | `minimum`, `maximum` | Inclusive integer bounds, compared by value; the schema must also say `type: integer` | `range` |
//! | `format` | One of the named formats below, checked on strings | `syntax`, with a corrected value in `fix` when one can be derived |
//! | `items` | Schema of every element of an array | |
//! | `properties` | Schema of each declared member; without `additionalProperties` the object is open and other members are accepted unchecked (output snapshots, ADR 0009 C5) | |
//! | `additionalProperties` | `false`: a member not in `properties` is refused; or a schema: every member not in `properties` is checked by it (a map, such as the `lines` of `validate`) | `unknown_field`, with the closest declared key in `fix`, for `false` |
//! | `propertyNames` | Schema of the name of every member, checked as a string; its errors are reported at the pointer of the member and told as errors of the name. It is a schema of strings, through its reference, without `enum` or `const` | the reason of the keyword that failed, usually `syntax` |
//! | `required` | Declared members that must be present | `missing` |
//! | `if` and `then` | A conditional requirement: when the value matches the condition of `if`, the members listed in the `required` of `then` must be present | `requires`, with the pointers of the members the condition tested in `pointers` |
//! | `allOf` | A list of `if` and `then` pairs | |
//! | `$ref` | `#/$defs/<name>`, a definition of the same file, the name without `/`, `~` or `%`; only annotations beside it | |
//! | `$defs` | Definitions, at the root only; a definition is not itself a `$ref` | |
//! | `$schema` | At the root only, and only `https://json-schema.org/draft/2020-12/schema` | |
//! | `$id`, `$comment`, `title`, `description` | Annotations, never checked; `$id` at the root only | |
//!
//! A member of an object set to `null` is not given, exactly as an absent member (ADR 0009
//! C2, SPEC-0001): it is accepted when optional, reported as `missing` when required (or
//! `requires` under a `then`), never reported as unknown, and read as absent by a condition.
//! A `null` element of an array is a value, checked like any other, and since no type of the
//! subset is `null` it fails `type`.
//!
//! Every integer the interpreter checks, whatever its schema, lies within plus or minus
//! (2^53 - 1), the range in which an IEEE 754 double holds every integer exactly (RFC 7493,
//! I-JSON; decision of the maintainer, 2026-11-20): a value beyond is refused with `range`,
//! by that one error, right after `type`. Integers written in a schema (`enum`, `const`,
//! `minimum`, `maximum`) are held to the same bound when it is loaded. A number in a part of
//! the document that no schema describes, such as a member of an object without
//! `properties`, is not checked. Numbers are read as IEEE 754 doubles, as RFC 8785 reads them,
//! so digits beyond that precision are not seen: `1.0000000000000001` is the integer 1.
//!
//! The condition of `if` uses `properties`, `required`, `enum`, `const` and `contains` (an
//! array with at least one element matching a condition), and annotations; `then` holds
//! `required` only. Every member named by `required`, by the first level of an `if` or by a
//! `then` is declared in the `properties` of its schema. `pattern` is not in the subset: the
//! core holds no regular expression engine, and a named format gives a precise `fix`.
//!
//! # Named formats
//!
//! | Format | Accepted form | Fix |
//! | --- | --- | --- |
//! | `date` | `YYYY-MM-DD`, a day of the Gregorian calendar (RFC 3339 `full-date`) | none |
//! | `utc-date-time` | `YYYY-MM-DDThh:mm:ssZ`, with an optional fraction of 1 to 9 digits; hours 00 to 23, seconds 00 to 59, upper-case `T` and `Z` | none |
//! | `language-tag` | A BCP 47 tag of language (2 or 3 letters), optional script (4 letters), optional region (2 letters or 3 digits) and variants (5 to 8 letters or digits, or a digit and 3 letters or digits), any case; no extension, private use or grandfathered tag | `_` written `-` |
//! | `tracker-handle` | `@` then 1 to 39 letters, digits or single hyphens, not at either end | `@` added |
//! | `project-path` | A path relative to the root of the project in one spelling: segments joined by `/`, none empty, `.` or `..`; no `\`, no drive letter (`C:`) | `\` written `/`, empty and `.` segments dropped; none for a path that starts with `/` or `\` |
//! | `brief-id`, `council-id`, `gate-id`, `epic-id`, `story-id`, `spec-id`, `change-id`, `task-id` | `BRIEF-`, `COUNCIL-`, `GATE-`, `EPIC-`, `STORY-`, `SPEC-`, `CHANGE-` or `TASK-` then four digits, or five or more without a leading zero (SPEC-0001, Identifiers) | prefix in upper case, separator `-`, sequence padded or unpadded |
//! | `brief-or-council-id` | A `brief-id` or a `council-id`: the subject of a gate issue | as for the identifier it starts with |
//! | `milestone-id` | `M` then `0` or a number without a leading zero | `M` and the number, from `m2`, `M02` or `Milestone 2` |
//! | `handover-id` | `HANDOVER-<issue>-<k>`, both numbers from 1 without a leading zero | prefix in upper case, leading zeros removed |
//! | `requirement-id`, `criterion-id`, `decision-id`, `question-id` | `R`, `AC`, `D` or `Q` then a number from 1 without a leading zero | prefix in upper case, leading zeros removed |
//! | `json-pointer` | An RFC 6901 pointer: empty, or reference tokens each after `/`, in which `~` is followed by `0` or `1` | none |
//!
//! # Order of the errors
//!
//! Every error is found in one pass, in document order: a value's own errors come before the
//! errors inside it; the members of an object are visited in the byte order of their keys
//! (JSON objects have no order, and the document model keeps none), the elements of an array
//! by index; the errors of the name of a member (`propertyNames`) come before the errors of
//! its value, at the same pointer; the absent members of an object follow its present ones,
//! first those of `required` in its order, then those of each `then`. A member missing under both `required`
//! and a `then` is reported once, as `missing`. The closest key of an `unknown_field` is a
//! declared key not present in the object, at an optimal string alignment distance (the
//! restricted Damerau-Levenshtein distance) of at most a third of the length of the unknown
//! key; ties go to the first key in byte order. Keys longer than 64 characters get no fix.
//!
//! # Bounds
//!
//! A schema is at most [`MAX_DEPTH`] levels of subschemas deep, counted through `$ref`; a
//! reference that reaches itself is refused. The interpreter descends into a document only
//! where its schema does, so its recursion is bounded by [`MAX_DEPTH`] whatever the document.

use core::fmt;

use serde_json::Value;

use wayloop_schemas::contract::{ContractError, Direction, Registered};

use crate::{Code, Error, Pointer};

mod check;
mod closest;
pub(crate) mod format;
mod load;

#[cfg(test)]
pub(crate) mod tests;

/// Deepest nesting of subschemas a schema may have, counted through `$ref`: the root is at
/// depth 0 and each `properties` member, `items`, `additionalProperties`, `propertyNames`,
/// `if`, `contains` and reference target adds one level (an entry of `allOf` and a `then` add
/// none).
pub const MAX_DEPTH: usize = 32;

/// Why a schema was refused when loaded. A refused schema is a defect of the engine, not of
/// the user's input: the schemas are files of the engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    /// A keyword outside the subset, or one not allowed at its place.
    Unsupported {
        /// Pointer into the schema to the subschema that holds the keyword.
        pointer: Pointer,
        /// The keyword.
        keyword: String,
    },
    /// A keyword of the subset with a value it does not accept.
    Invalid {
        /// Pointer into the schema to the faulty value.
        pointer: Pointer,
        /// What is wrong, for a person.
        problem: &'static str,
    },
    /// Subschemas nested deeper than [`MAX_DEPTH`].
    TooDeep {
        /// Pointer into the schema to a subschema reached beyond the bound, or whose expansion
        /// goes beyond it.
        pointer: Pointer,
    },
    /// A reference that reaches its own definition.
    Recursive {
        /// Pointer into the schema to the definition.
        pointer: Pointer,
    },
}

/// The place of `pointer` in the schema, for a person.
fn place(pointer: &Pointer) -> String {
    if pointer.as_str().is_empty() {
        "the root of the schema".to_owned()
    } else {
        pointer.to_string()
    }
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Unsupported { pointer, keyword } => write!(
                f,
                "the keyword `{keyword}` at {} is outside the subset of the interpreter",
                place(pointer)
            ),
            LoadError::Invalid { pointer, problem } => {
                write!(f, "at {}: {problem}", place(pointer))
            }
            LoadError::TooDeep { pointer } => write!(
                f,
                "at {}: the schema is nested deeper than {MAX_DEPTH} levels",
                place(pointer)
            ),
            LoadError::Recursive { pointer } => write!(
                f,
                "at {}: the definition reaches itself through a reference",
                place(pointer)
            ),
        }
    }
}

impl std::error::Error for LoadError {}

/// Why the schema of a registered contract type was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractFault {
    /// A rule of the contracts that is not a rule of the subset: descriptions, meanings of enum
    /// values, levels of objects, open outputs (`wayloop_schemas::contract`).
    Rule(ContractError),
    /// A keyword outside the subset, refused by [`Schema::load`].
    Subset(LoadError),
}

impl fmt::Display for ContractFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContractFault::Rule(error) => write!(f, "a rule of the contracts: {error}"),
            ContractFault::Subset(error) => write!(f, "the subset of the interpreter: {error}"),
        }
    }
}

impl std::error::Error for ContractFault {}

/// The schema of the contract type `registered` for `direction`, checked against the rules of
/// the contracts and then loaded by [`Schema::load`]: the subset has one definition, this
/// loader (ADR 0030 G3).
///
/// # Errors
///
/// [`ContractFault::Rule`] for the first rule of the contracts the schema breaks, then
/// [`ContractFault::Subset`] for the first place outside the subset.
pub fn contract_schema(
    registered: Registered,
    direction: Direction,
) -> Result<Value, ContractFault> {
    let schema = (registered.schema)(direction).map_err(ContractFault::Rule)?;
    Schema::load(&schema).map_err(ContractFault::Subset)?;
    Ok(schema)
}

/// A loaded schema of the subset, ready to validate documents.
#[derive(Debug, Clone)]
pub struct Schema {
    compiled: load::Compiled,
}

impl Schema {
    /// Loads `schema`, refusing any keyword outside the subset of this module.
    ///
    /// # Errors
    ///
    /// A [`LoadError`] naming the first place of `schema` outside the subset.
    pub fn load(schema: &Value) -> Result<Self, LoadError> {
        load::load(schema).map(|compiled| Self { compiled })
    }

    /// Every error of `document` against this schema, in document order, each with the
    /// given `code`; empty when the document is valid.
    #[must_use]
    pub fn validate(&self, document: &Value, code: Code) -> Vec<Error> {
        let mut checker = check::Checker {
            schema: &self.compiled,
            code,
            errors: Vec::new(),
            naming: false,
        };
        checker.check(self.compiled.root, document, &Pointer::root());
        checker.errors
    }
}
