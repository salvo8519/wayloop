//! Canonical JSON of RFC 8785 (JSON Canonicalization Scheme) over [`serde_json::Value`].
//!
//! The canonical form is the text that `meta.input_hash` hashes (ADR 0009 C1, SPEC-0004 R11):
//! the same value gives the same bytes on every machine, whatever the order of its keys.
//!
//! - Object members are sorted by the UTF-16 code units of their keys, never by the order of
//!   the map. The keys of a [`serde_json::Map`] are unique, so the duplicate keys that RFC 8785
//!   refuses cannot reach this function.
//! - Strings are written as ECMAScript `JSON.stringify` writes them: `"` and `\` escaped, the
//!   control characters U+0000 to U+001F written as `\b`, `\t`, `\n`, `\f`, `\r` or `\u00xx`
//!   with lower-case hexadecimal, every other character as itself in UTF-8.
//! - Numbers are IEEE 754 doubles written in the ECMAScript form (RFC 8785, 3.2.2.3): an
//!   integer outside the exact range of a double (beyond 2^53 in magnitude) is written as the
//!   nearest double, as the RFC requires, so two such integers can share a canonical form.
//!   NaN and the infinities, which RFC 8785 refuses, cannot be held in a
//!   [`serde_json::Value`], so the function is total.
//! - No white space is written.

use core::fmt::Write as _;

use serde_json::{Number, Value};

/// What is left to write, kept on an explicit stack so that a deeply nested value cannot
/// exhaust the call stack.
enum Step<'a> {
    Value(&'a Value),
    Key(&'a str),
    Text(&'static str),
}

/// The canonical JSON text of `value` (RFC 8785).
#[must_use]
pub fn canonical_json(value: &Value) -> String {
    let mut out = String::new();
    let mut steps = vec![Step::Value(value)];
    while let Some(step) = steps.pop() {
        match step {
            Step::Text(text) => out.push_str(text),
            Step::Key(key) => {
                write_string(&mut out, key);
                out.push(':');
            }
            Step::Value(Value::Null) => out.push_str("null"),
            Step::Value(Value::Bool(true)) => out.push_str("true"),
            Step::Value(Value::Bool(false)) => out.push_str("false"),
            Step::Value(Value::Number(number)) => write_number(&mut out, number),
            Step::Value(Value::String(text)) => write_string(&mut out, text),
            Step::Value(Value::Array(items)) => {
                out.push('[');
                steps.push(Step::Text("]"));
                for (index, item) in items.iter().enumerate().rev() {
                    steps.push(Step::Value(item));
                    if index > 0 {
                        steps.push(Step::Text(","));
                    }
                }
            }
            Step::Value(Value::Object(members)) => {
                out.push('{');
                steps.push(Step::Text("}"));
                let mut members: Vec<(&String, &Value)> = members.iter().collect();
                members.sort_by(|(a, _), (b, _)| a.encode_utf16().cmp(b.encode_utf16()));
                for (index, (key, item)) in members.into_iter().enumerate().rev() {
                    steps.push(Step::Value(item));
                    steps.push(Step::Key(key));
                    if index > 0 {
                        steps.push(Step::Text(","));
                    }
                }
            }
        }
    }
    out
}

/// Writes `text` as a JSON string, escaped as `JSON.stringify` escapes it (RFC 8785, 3.2.2.2).
fn write_string(out: &mut String, text: &str) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{c}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            '\u{0}'..='\u{1f}' => {
                // Writing to a String cannot fail.
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            _ => out.push(c),
        }
    }
    out.push('"');
}

/// Writes `number` as the IEEE 754 double it denotes, in the ECMAScript form.
fn write_number(out: &mut String, number: &Number) {
    match number.as_f64() {
        Some(double) => write_double(out, double),
        // Without the `arbitrary_precision` feature of serde_json, which no crate of the
        // workspace enables, every number has a double; with it, the text is kept as given.
        None => out.push_str(&number.to_string()),
    }
}

/// The shortest digits that read back as `double`, closest to it, ties to the even digit, in
/// the exponent form of the standard library: `1.2345e-7` for the digits 12345 and the
/// exponent -7.
///
/// The standard library gives the shortest count of digits, but when two strings of that
/// count read back as `double` it may pick the farther one (for 1424953923781206.25 it writes
/// …206.3). ECMA-262 requires the closer one, and the even one on a tie (…206.2). The same
/// count of digits rounded from the exact value, ties to even, is the closest string; it is
/// kept when it reads back as `double`, which can fail only next to a power of two, where the
/// interval of the double is narrower below than above.
fn shortest_closest(double: f64) -> String {
    let shortest = format!("{double:e}");
    let digits = shortest.split_once('e').map_or(0, |(mantissa, _)| {
        mantissa.chars().filter(char::is_ascii_digit).count()
    });
    let closest = format!("{double:.*e}", digits.saturating_sub(1));
    if closest.parse::<f64>() == Ok(double) {
        closest
    } else {
        shortest
    }
}

/// Writes a finite double as ECMAScript `Number.prototype.toString` does (ECMA-262,
/// `Number::toString`, radix 10), which RFC 8785 adopts: the shortest digits that read back as
/// the same double, in plain notation when the decimal exponent is from -6 to 20, in exponent
/// notation otherwise; minus zero is written `0`.
fn write_double(out: &mut String, double: f64) {
    if double == 0.0 {
        out.push('0');
        return;
    }
    let scientific = shortest_closest(double.abs());
    let Some((mantissa, exponent)) = scientific.split_once('e') else {
        // Only NaN and the infinities have no exponent, and a Value holds neither; write what
        // JSON.stringify writes for them.
        out.push_str("null");
        return;
    };
    let Ok(exponent) = exponent.parse::<i32>() else {
        out.push_str("null");
        return;
    };
    let digits: Vec<char> = mantissa.chars().filter(char::is_ascii_digit).collect();
    // At most 17 significant digits and an exponent within -324 and 308: no overflow.
    let count = i32::try_from(digits.len()).unwrap_or(i32::MAX);
    // ECMA-262 names: k digits, the value is digits × 10^(n − k).
    let n = exponent + 1;

    if double.is_sign_negative() {
        out.push('-');
    }
    if count <= n && n <= 21 {
        out.extend(digits.iter());
        out.extend(core::iter::repeat_n(
            '0',
            usize::try_from(n - count).unwrap_or(0),
        ));
    } else if 0 < n && n <= 21 {
        let point = usize::try_from(n).unwrap_or(0);
        out.extend(digits.iter().take(point));
        out.push('.');
        out.extend(digits.iter().skip(point));
    } else if -6 < n && n <= 0 {
        out.push_str("0.");
        out.extend(core::iter::repeat_n('0', usize::try_from(-n).unwrap_or(0)));
        out.extend(digits.iter());
    } else {
        let mut rest = digits.iter();
        out.extend(rest.next());
        if count > 1 {
            out.push('.');
            out.extend(rest);
        }
        out.push('e');
        out.push(if n > 0 { '+' } else { '-' });
        let _ = write!(out, "{}", (n - 1).unsigned_abs());
    }
}

#[cfg(test)]
mod tests {
    use super::canonical_json;
    use serde_json::{Number, Value, json};

    fn number(bits: u64) -> String {
        let value = Value::Number(Number::from_f64(f64::from_bits(bits)).unwrap());
        canonical_json(&value)
    }

    /// RFC 8785, Appendix B: every finite sample of the table, from its IEEE 754 bits.
    #[test]
    fn numbers_follow_the_samples_of_appendix_b() {
        let samples: [(u64, &str); 25] = [
            (0x0000_0000_0000_0000, "0"),
            (0x8000_0000_0000_0000, "0"),
            (0x0000_0000_0000_0001, "5e-324"),
            (0x8000_0000_0000_0001, "-5e-324"),
            (0x7fef_ffff_ffff_ffff, "1.7976931348623157e+308"),
            (0xffef_ffff_ffff_ffff, "-1.7976931348623157e+308"),
            (0x4340_0000_0000_0000, "9007199254740992"),
            (0xc340_0000_0000_0000, "-9007199254740992"),
            (0x4430_0000_0000_0000, "295147905179352830000"),
            (0x44b5_2d02_c7e1_4af5, "9.999999999999997e+22"),
            (0x44b5_2d02_c7e1_4af6, "1e+23"),
            (0x44b5_2d02_c7e1_4af7, "1.0000000000000001e+23"),
            (0x444b_1ae4_d6e2_ef4e, "999999999999999700000"),
            (0x444b_1ae4_d6e2_ef4f, "999999999999999900000"),
            (0x444b_1ae4_d6e2_ef50, "1e+21"),
            (0x3eb0_c6f7_a0b5_ed8c, "9.999999999999997e-7"),
            (0x3eb0_c6f7_a0b5_ed8d, "0.000001"),
            (0x41b3_de43_5555_5553, "333333333.3333332"),
            (0x41b3_de43_5555_5554, "333333333.33333325"),
            (0x41b3_de43_5555_5555, "333333333.3333333"),
            (0x41b3_de43_5555_5556, "333333333.3333334"),
            (0x41b3_de43_5555_5557, "333333333.33333343"),
            (0xbecb_f647_612f_3696, "-0.0000033333333333333333"),
            (0x4314_3ff3_c1cb_0959, "1424953923781206.2"),
            (0x3ff0_0000_0000_0000, "1"),
        ];
        for (bits, expected) in samples {
            assert_eq!(number(bits), expected, "{bits:016x}");
        }
    }

    #[test]
    fn numbers_switch_to_the_exponent_form_where_ecmascript_does() {
        for (value, expected) in [
            (1e20, "100000000000000000000"),
            (1e21, "1e+21"),
            (123e18, "123000000000000000000"),
            (1.5e21, "1.5e+21"),
            (1e-6, "0.000001"),
            (1.5e-6, "0.0000015"),
            (1e-7, "1e-7"),
            (-1.25e-7, "-1.25e-7"),
            (0.5, "0.5"),
            (12.5, "12.5"),
            (-4.5, "-4.5"),
        ] {
            let value = Value::Number(Number::from_f64(value).unwrap());
            assert_eq!(canonical_json(&value), expected);
        }
    }

    #[test]
    fn integers_are_written_as_doubles() {
        assert_eq!(canonical_json(&json!(0)), "0");
        assert_eq!(canonical_json(&json!(-17)), "-17");
        assert_eq!(
            canonical_json(&json!(9_007_199_254_740_992_u64)),
            "9007199254740992"
        );
        // Beyond 2^53 the nearest double is written, as RFC 8785 requires.
        assert_eq!(canonical_json(&json!(u64::MAX)), "18446744073709552000");
        assert_eq!(canonical_json(&json!(i64::MIN)), "-9223372036854776000");
    }

    /// RFC 8785, 3.2.2: the example of the whole transformation, parsed from its text.
    #[test]
    fn the_example_of_section_3_2_2() {
        let input = r#"{
            "numbers": [333333333.33333329, 1E30, 4.50, 2e-3, 0.000000000000000000000000001],
            "string": "\u20ac$\u000F\u000aA'\u0042\u0022\u005c\\\"\/",
            "literals": [null, true, false]
        }"#;
        let value: Value = serde_json::from_str(input).unwrap();
        assert_eq!(
            canonical_json(&value),
            "{\"literals\":[null,true,false],\"numbers\":[333333333.3333333,1e+30,4.5,0.002,1e-27],\"string\":\"\u{20ac}$\\u000f\\nA'B\\\"\\\\\\\\\\\"/\"}"
        );
    }

    /// RFC 8785, 3.2.3: keys are sorted by UTF-16 code units, so the emoji (a surrogate pair
    /// starting at D83D) comes before U+FB33, unlike in code point or UTF-8 order.
    #[test]
    fn keys_are_sorted_by_utf16_code_units() {
        let input = r#"{
            "\u20ac": "Euro Sign",
            "\r": "Carriage Return",
            "\ufb33": "Hebrew Letter Dalet With Dagesh",
            "1": "One",
            "\ud83d\ude00": "Emoji: Grinning Face",
            "\u0080": "Control",
            "\u00f6": "Latin Small Letter O With Diaeresis"
        }"#;
        let value: Value = serde_json::from_str(input).unwrap();
        let expected = concat!(
            "{\"\\r\":\"Carriage Return\",",
            "\"1\":\"One\",",
            "\"\u{80}\":\"Control\",",
            "\"\u{f6}\":\"Latin Small Letter O With Diaeresis\",",
            "\"\u{20ac}\":\"Euro Sign\",",
            "\"\u{1f600}\":\"Emoji: Grinning Face\",",
            "\"\u{fb33}\":\"Hebrew Letter Dalet With Dagesh\"}"
        );
        assert_eq!(canonical_json(&value), expected);
    }

    #[test]
    fn strings_escape_only_what_json_stringify_escapes() {
        let value = json!("\u{0}\u{8}\t\n\u{b}\u{c}\r\u{1f}\"\\/\u{7f}\u{2028}\u{e9}");
        assert_eq!(
            canonical_json(&value),
            "\"\\u0000\\b\\t\\n\\u000b\\f\\r\\u001f\\\"\\\\/\u{7f}\u{2028}\u{e9}\""
        );
    }

    #[test]
    fn nested_values_have_no_white_space_and_keep_array_order() {
        let value = json!({"b": [3, {"z": null, "a": []}, "x"], "a": {}, "c": true});
        assert_eq!(
            canonical_json(&value),
            r#"{"a":{},"b":[3,{"a":[],"z":null},"x"],"c":true}"#
        );
    }

    #[test]
    fn the_order_of_keys_in_the_text_does_not_change_the_form() {
        let one: Value = serde_json::from_str(r#"{"b":1,"a":{"y":2,"x":[1,2]}}"#).unwrap();
        let two: Value = serde_json::from_str(r#"{"a":{"x":[1,2],"y":2},"b":1}"#).unwrap();
        assert_eq!(canonical_json(&one), canonical_json(&two));
    }

    #[test]
    fn deep_nesting_does_not_exhaust_the_stack() {
        let mut value = json!(1);
        for _ in 0..100_000 {
            value = Value::Array(vec![value]);
        }
        let text = canonical_json(&value);
        assert_eq!(text.len(), 200_001);
        // Dropping a Value this deep recurses in serde_json: take it apart by hand.
        while let Value::Array(mut items) = value {
            value = items.pop().unwrap_or(Value::Null);
        }
    }
}
