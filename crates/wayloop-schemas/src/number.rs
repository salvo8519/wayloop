//! Numbers of the contracts, read by value (plan of SPEC-0001, Numbers; decision of the
//! maintainer, 2026-11-20).
//!
//! An integer is a finite JSON number without a fraction, whatever its text: `4364.0` is the
//! integer `4364`, as JSON Schema and the canonical JSON of RFC 8785 read it. Every integer of a
//! contract lies within plus or minus [`SAFE_INTEGER`], the range in which an IEEE 754 double
//! holds every integer exactly (RFC 7493, I-JSON). The interpreter of the core checks documents
//! with these functions, and the number types of this crate read with them, so that a document
//! the interpreter accepts always reads into its type (ADR 0030 G1).

use serde_json::Value;

/// The largest magnitude of an integer, 2^53 - 1.
pub const SAFE_INTEGER: i64 = 9_007_199_254_740_991;

/// Whether `n` is within plus or minus [`SAFE_INTEGER`].
#[must_use]
pub fn is_safe(n: i128) -> bool {
    n.unsigned_abs() <= SAFE_INTEGER.unsigned_abs().into()
}

/// The integer value of `value`: a JSON number without a fraction. A number beyond the range
/// of `i128` saturates, which keeps every comparison with an `i64` bound exact.
#[must_use]
pub fn integer(value: &Value) -> Option<i128> {
    let number = value.as_number()?;
    if let Some(i) = number.as_i64() {
        return Some(i.into());
    }
    if let Some(u) = number.as_u64() {
        return Some(u.into());
    }
    let float = number.as_f64()?;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the value has no fraction, and a cast from a float saturates"
    )]
    let whole = float.trunc() as i128;
    (float.is_finite() && float.trunc().to_bits() == float.to_bits()).then_some(whole)
}

#[cfg(test)]
mod tests {
    use super::{SAFE_INTEGER, integer, is_safe};
    use serde_json::json;

    #[test]
    fn an_integer_is_read_by_value() {
        assert_eq!(integer(&json!(4364)), Some(4364));
        assert_eq!(integer(&json!(4364.0)), Some(4364));
        assert_eq!(integer(&json!(-7)), Some(-7));
        assert_eq!(integer(&json!(u64::MAX)), Some(u64::MAX.into()));
        assert_eq!(integer(&json!(1.5)), None);
        assert_eq!(integer(&json!("1")), None);
        assert_eq!(integer(&json!(1e300)).map(|n| n > 0), Some(true));
    }

    #[test]
    fn the_safe_range_is_plus_or_minus_two_to_the_53_minus_one() {
        assert!(is_safe(SAFE_INTEGER.into()));
        assert!(is_safe((-SAFE_INTEGER).into()));
        assert!(!is_safe(i128::from(SAFE_INTEGER) + 1));
        assert!(!is_safe(-i128::from(SAFE_INTEGER) - 1));
    }
}
