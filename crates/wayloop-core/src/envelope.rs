//! The part of the envelope the core computes (ADR 0009 C1, SPEC-0001 plan D6): the hash of the
//! input and the meta of a result.
//!
//! The envelope types are contract types of `wayloop-schemas`, re-exported at the root of the
//! crate; `Envelope<D>` is generic over the data of one command (ADR 0030 G1). The entry point
//! of ADR 0028 (#221) fills them with these functions; no channel computes the hash or the
//! versions itself.

use core::num::NonZeroU32;

use serde_json::Value;
use wayloop_schemas::{DefaultApplied, InputHash, Meta};

use crate::VERSION;
use crate::canonical::canonical_json;
use crate::sha256::sha256;

/// `meta.input_hash` of `input`: the SHA-256 of its canonical JSON (RFC 8785), in lower-case
/// hexadecimal (ADR 0009 C1, SPEC-0004 R11). The same input gives the same hash whatever the
/// order of its keys.
#[must_use]
pub fn input_hash(input: &Value) -> InputHash {
    InputHash::from_digest(&sha256(canonical_json(input).as_bytes()))
}

/// The meta of a result of this engine for version `command_version` of a command on `input`,
/// with the defaults applied in order. It holds no clock time, host name or absolute path
/// (SPEC-0004 R15).
#[must_use]
pub fn meta(
    command_version: NonZeroU32,
    input: &Value,
    dry_run: bool,
    defaults_applied: Vec<DefaultApplied>,
) -> Meta {
    Meta::new(
        VERSION,
        command_version,
        input_hash(input),
        dry_run,
        defaults_applied,
    )
}

#[cfg(test)]
mod tests {
    use super::{input_hash, meta};
    use crate::VERSION;
    use crate::canonical::canonical_json;
    use crate::test_support::SplitMix64;
    use core::fmt::Write;
    use core::num::NonZeroU32;
    use serde_json::{Map, Value, json};
    use sha2::{Digest, Sha256};

    #[test]
    fn the_hash_of_the_empty_object_is_the_sha256_of_its_two_bytes() {
        // SHA-256 of "{}", computed with the hashing crate used in tests only.
        let expected = Sha256::digest(b"{}");
        assert_eq!(
            input_hash(&json!({})).as_str(),
            expected.iter().fold(String::new(), |mut text, byte| {
                write!(text, "{byte:02x}").unwrap();
                text
            })
        );
        assert_eq!(
            input_hash(&json!({})).as_str(),
            "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a"
        );
    }

    #[test]
    fn white_space_and_key_order_of_the_text_do_not_change_the_hash() {
        let one: Value = serde_json::from_str(
            r#"{ "issue": 253, "labels": ["core", "contract"], "a": {"y": 1.0, "x": null} }"#,
        )
        .unwrap();
        let two: Value = serde_json::from_str(
            r#"{"a":{"x":null,"y":1},"labels":["core","contract"],"issue":253}"#,
        )
        .unwrap();
        assert_eq!(input_hash(&one), input_hash(&two));
        let other: Value = serde_json::from_str(
            r#"{"a":{"x":null,"y":1},"labels":["contract","core"],"issue":253}"#,
        )
        .unwrap();
        assert_ne!(input_hash(&one), input_hash(&other), "array order counts");
    }

    const KEYS: [&str; 8] = [
        "a",
        "B",
        "\u{e9}",
        "\u{20ac}",
        "\u{1f600}",
        "\u{fb33}",
        "1",
        "id",
    ];

    fn value(random: &mut SplitMix64, depth: u32) -> Value {
        match random.below(if depth == 0 { 4 } else { 6 }) {
            0 => Value::Null,
            1 => json!(random.next() % 2 == 0),
            2 => json!(f64::from(u32::try_from(random.next() % 100_000).unwrap()) / 8.0),
            3 => json!(KEYS[random.below(8)]),
            4 => (0..random.below(4))
                .map(|_| value(random, depth - 1))
                .collect(),
            _ => {
                let mut map = Map::new();
                for _ in 0..random.below(5) {
                    map.insert(KEYS[random.below(8)].into(), value(random, depth - 1));
                }
                Value::Object(map)
            }
        }
    }

    /// The JSON text of `value` with the members of every object in a random order and random
    /// spaces, as another channel might send it.
    fn shuffled_text(value: &Value, random: &mut SplitMix64) -> String {
        match value {
            Value::Object(map) => {
                let mut members: Vec<(&String, &Value)> = map.iter().collect();
                for i in (1..members.len()).rev() {
                    members.swap(i, random.below(u64::try_from(i + 1).unwrap()));
                }
                let members: Vec<String> = members
                    .into_iter()
                    .map(|(key, item)| {
                        format!(
                            "{} : {}",
                            serde_json::to_string(key).unwrap(),
                            shuffled_text(item, random)
                        )
                    })
                    .collect();
                format!("{{ {} }}", members.join(" , "))
            }
            Value::Array(items) => {
                let items: Vec<String> = items
                    .iter()
                    .map(|item| shuffled_text(item, random))
                    .collect();
                format!("[{}]", items.join(", "))
            }
            other => other.to_string(),
        }
    }

    #[test]
    fn the_same_input_with_keys_in_any_order_gives_one_hash() {
        // SPEC-0004 AC7, on this platform: the hash depends on the value, not on its text.
        let mut random = SplitMix64(0x0253_0253_0253_0253);
        for _ in 0..500 {
            let original = value(&mut random, 4);
            let text = shuffled_text(&original, &mut random);
            let parsed: Value = serde_json::from_str(&text).unwrap();
            assert_eq!(parsed, original, "{text}");
            assert_eq!(input_hash(&parsed), input_hash(&original), "{text}");
            // The canonical form is a fixed point: read back, it gives itself. (It may read
            // back as another Value: 4364.0 is written 4364, as RFC 8785 writes every double.)
            let canonical = canonical_json(&original);
            let read_back: Value = serde_json::from_str(&canonical).unwrap();
            assert_eq!(canonical_json(&read_back), canonical);
        }
    }

    #[test]
    fn meta_carries_the_engine_version_and_the_input_hash() {
        let input = json!({"documents": []});
        let meta = meta(NonZeroU32::MIN, &input, true, vec![]);
        assert_eq!(meta.engine_version(), VERSION);
        assert_eq!(meta.command_version(), NonZeroU32::MIN);
        assert_eq!(meta.input_hash(), &input_hash(&input));
        assert!(meta.dry_run());
        assert_eq!(meta.defaults_applied(), []);
    }
}
