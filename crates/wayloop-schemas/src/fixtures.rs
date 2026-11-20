//! Values shared by the tests of this crate, written once.

use core::num::NonZeroU64;

use crate::{Code, Error, Pointer, Reason};

/// The code of the refusals of the tests.
pub(crate) const INVALID_INPUT: Code = Code::new("invalid_input");

/// The pointer written as `pointer`, which the test knows to be in the form of RFC 6901.
pub(crate) fn at(pointer: &str) -> Pointer {
    Pointer::parse(pointer).unwrap()
}

/// The number `n`, which the test knows to be at least 1.
pub(crate) fn number(n: u64) -> NonZeroU64 {
    NonZeroU64::new(n).unwrap()
}

/// Two refusals found in one pass: an unknown field with its fix, then the missing field.
pub(crate) fn unknown_then_missing() -> (Error, Error) {
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
