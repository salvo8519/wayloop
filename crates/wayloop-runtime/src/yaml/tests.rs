//! Tests of the YAML reading rules (plan of SPEC-0001, D1; tasks, T6).

use core::fmt::Write as _;

use serde_json::{Value, json};
use wayloop_core::artifacts::locations::{MAX_NESTING, MAX_VALUES};

use wayloop_core::artifacts::locations::MAX_DOCUMENT_BYTES;
use wayloop_schemas::validate::ParseError;

use super::Parsed;

/// `text` parsed with the room of a document.
fn parse(text: &str) -> Result<Parsed, ParseError> {
    super::parse(text, MAX_DOCUMENT_BYTES)
}

fn parsed(text: &str) -> Parsed {
    parse(text).unwrap_or_else(|e| panic!("{text:?}: {e:?}"))
}

fn document(text: &str) -> Value {
    Value::Object(parsed(text).document)
}

/// The line and the message of the refusal of `text`.
fn refused(text: &str) -> (u64, String) {
    let error = parse(text).map(|p| p.document).unwrap_err();
    (error.line.get().get(), error.message)
}

fn lines(text: &str) -> Vec<(String, u64)> {
    parsed(text)
        .lines
        .0
        .into_iter()
        .map(|(pointer, line)| (pointer, line.get().get()))
        .collect()
}

#[test]
fn no_on_yes_and_dates_stay_strings() {
    let read = document(
        "a: no\nb: on\nc: yes\nd: off\ne: 2026-11-17\nf: 2026-11-17T10:00:00Z\ng: y\nh: n\n",
    );
    assert_eq!(
        read,
        json!({"a": "no", "b": "on", "c": "yes", "d": "off", "e": "2026-11-17",
               "f": "2026-11-17T10:00:00Z", "g": "y", "h": "n"})
    );
}

#[test]
fn scalars_follow_the_yaml_1_2_core_schema() {
    let read = document(concat!(
        "n1: null\nn2: ~\nn3:\nn4: NULL\nn5: Null\n",
        "t1: true\nt2: True\nt3: TRUE\nf1: false\nf2: False\nf3: FALSE\n",
        "i1: 42\ni2: -7\ni3: +3\ni4: 0o17\ni5: 0x1F\ni6: 0123\n",
        "r1: 1.5\nr2: .5\nr3: -2.\nr4: 1e3\nr5: 6.02E+23\nr6: +.25e-1\n",
        "s1: '42'\ns2: \"true\"\ns3: tRue\ns4: 0x\ns5: 0o8\ns6: 1.2.3\ns7: '~'\ns8: 1e\n",
        "s9: |\n  42\ns10: >-\n  null\ns11: .\ns12: -.nan\ns13: e3\n",
    ));
    assert_eq!(
        read,
        json!({
            "n1": null, "n2": null, "n3": null, "n4": null, "n5": null,
            "t1": true, "t2": true, "t3": true, "f1": false, "f2": false, "f3": false,
            "i1": 42, "i2": -7, "i3": 3, "i4": 15, "i5": 31, "i6": 123,
            "r1": 1.5, "r2": 0.5, "r3": -2.0, "r4": 1000.0, "r5": 6.02e23, "r6": 0.025,
            "s1": "42", "s2": "true", "s3": "tRue", "s4": "0x", "s5": "0o8", "s6": "1.2.3",
            "s7": "~", "s8": "1e", "s9": "42\n", "s10": "null", "s11": ".", "s12": "-.nan",
            "s13": "e3"
        })
    );
}

#[test]
fn integers_beyond_64_bits_become_the_nearest_double_as_in_json() {
    let read = document(
        "a: 18446744073709551615\nb: 18446744073709551616\nc: -9223372036854775808\n\
         d: 0xFFFFFFFFFFFFFFFFFF\ne: 0o7\n",
    );
    assert_eq!(read["a"], json!(u64::MAX));
    assert_eq!(read["b"], json!(18_446_744_073_709_551_616.0_f64));
    assert_eq!(read["c"], json!(i64::MIN));
    assert_eq!(read["d"], json!(4_722_366_482_869_645_213_695.0_f64));
    assert_eq!(read["e"], json!(7));
}

#[test]
fn numbers_json_cannot_hold_are_refused_with_their_line() {
    for (text, at) in [
        ("a: 1\nb: .inf\n", 2),
        ("a: -.Inf\n", 1),
        ("a: .NaN\n", 1),
        ("a: 1e400\n", 1),
        ("a:\n  - 1e999999\n", 2),
        (&format!("a: 0x{}\n", "F".repeat(300)), 1),
        (&format!("a: {}\n", "9".repeat(400)), 1),
    ] {
        let (line, message) = refused(text);
        assert_eq!(line, at, "{text}");
        assert!(
            message.contains("has no value in JSON"),
            "{text}: {message}"
        );
    }
}

#[test]
fn tags_of_the_core_schema_are_accepted_when_the_value_has_their_type() {
    let read = document(
        "a: !!str 42\nb: !!int 42\nc: !!bool true\nd: !!null ~\ne: !!float 1.5\n\
         f: !!str yes\ng: !!seq [1]\nh: !!map {x: 1}\n!!str k: v\n",
    );
    assert_eq!(
        read,
        json!({"a": "42", "b": 42, "c": true, "d": null, "e": 1.5, "f": "yes",
               "g": [1], "h": {"x": 1}, "k": "v"})
    );
    for (text, at) in [
        ("a: 1\nb: !!int x\n", 2),
        ("a: !!bool 1\n", 1),
        ("a: !custom x\n", 1),
        ("a: !int 42\n", 1),
        ("a: !str x\n", 1),
        ("a: !!binary aGk=\n", 1),
        ("a: !!map [1]\n", 1),
        ("a: !!seq {x: 1}\n", 1),
        ("a: !thing {x: 1}\n", 1),
        ("!!int 1: x\n", 1),
        ("a: !!float .inf\n", 1),
    ] {
        let (line, message) = refused(text);
        assert_eq!(line, at, "{text}: {message}");
    }
}

#[test]
fn every_value_has_its_line_members_at_their_key() {
    let text = "# A comment\nkind: brief\ngoals:\n  - one\n  - two\nnested:\n  inner: |\n    text\n  list: [a,\n    b]\nlast: x\n";
    assert_eq!(
        lines(text),
        [
            (String::new(), 2),
            ("/goals".to_owned(), 3),
            ("/goals/0".to_owned(), 4),
            ("/goals/1".to_owned(), 5),
            ("/kind".to_owned(), 2),
            ("/last".to_owned(), 11),
            ("/nested".to_owned(), 6),
            ("/nested/inner".to_owned(), 7),
            ("/nested/list".to_owned(), 9),
            ("/nested/list/0".to_owned(), 9),
            ("/nested/list/1".to_owned(), 10),
        ]
    );
}

#[test]
fn keys_are_escaped_in_pointers() {
    let read = lines("a/b: 1\nc~d: 2\n");
    assert_eq!(
        read,
        [
            (String::new(), 1),
            ("/a~1b".to_owned(), 1),
            ("/c~0d".to_owned(), 2)
        ]
    );
}

#[test]
fn a_duplicate_key_is_refused_with_the_line_of_the_second() {
    let (line, message) = refused("kind: brief\nid: BRIEF-0001\ntitle: A\nid: BRIEF-0002\n");
    assert_eq!(line, 4);
    assert!(
        message.contains("`id`") && message.contains("line 2"),
        "{message}"
    );
    // A quoted key and a plain key with the same text are the same key.
    assert_eq!(refused("a: 1\nb:\n  c: 1\n  'c': 2\n").0, 4);
    // The same key in two mappings is allowed.
    assert_eq!(
        document("a:\n  x: 1\nb:\n  x: 2\n"),
        json!({"a": {"x": 1}, "b": {"x": 2}})
    );
}

#[test]
fn a_malformed_file_is_refused_with_its_line() {
    for (text, at) in [
        ("kind: brief\ntitle: [unclosed\n", 3),
        ("kind: brief\n  bad: indent\n", 2),
        ("a: 'open\n", 1),
        ("a: *missing\n", 1),
    ] {
        let (line, message) = refused(text);
        assert_eq!(line, at, "{text}: {message}");
        assert!(!message.is_empty());
    }
}

#[test]
fn a_file_holds_one_document_and_it_is_a_mapping() {
    assert_eq!(refused("").0, 1);
    assert!(refused("# only a comment\n").1.contains("no YAML document"));
    let (line, message) = refused("a: 1\n---\nb: 2\n");
    assert_eq!(line, 2);
    assert!(message.contains("more than one"), "{message}");
    for (text, at, what) in [
        ("- a\n- b\n", 1, "a sequence"),
        ("\n\njust text\n", 3, "a string"),
        ("~\n", 1, "empty"),
        ("---\n", 2, "empty"),
        ("42\n", 1, "a number"),
        ("true\n", 1, "a boolean"),
    ] {
        let (line, message) = refused(text);
        assert_eq!(line, at, "{text}");
        assert!(message.contains(what), "{text}: {message}");
    }
    assert_eq!(document("---\na: 1\n...\n"), json!({"a": 1}));
    assert_eq!(document("\u{feff}a: 1\n"), json!({"a": 1}));
    assert_eq!(lines("a: 1\r\nb: 2\r\n")[2], ("/b".to_owned(), 2));
}

#[test]
fn a_key_is_a_scalar_written_in_place() {
    for text in ["? [a]\n: 1\n", "? {a: 1}\n: 2\n", "x: &k key\n*k : 1\n"] {
        let (_, message) = refused(text);
        assert!(
            message.contains("a key must be a scalar"),
            "{text}: {message}"
        );
    }
    assert_eq!(document("1: a\n~: b\n"), json!({"1": "a", "~": "b"}));
}

#[test]
fn an_alias_repeats_a_complete_value() {
    let read = document("base: &b {x: 1, y: [2, 3]}\ncopy: *b\nname: &n text\nagain: *n\n");
    assert_eq!(read["copy"], json!({"x": 1, "y": [2, 3]}));
    assert_eq!(read["again"], "text");
    let anchored_key = document("&k key: 1\nother: *k\n");
    assert_eq!(anchored_key, json!({"key": 1, "other": "key"}));
    // An alias to its own collection, still open, is refused.
    let (line, message) = refused("a: &s\n  - *s\n");
    assert_eq!(line, 2);
    assert!(message.contains("not complete"), "{message}");
    // `<<` is an ordinary key in YAML 1.2.
    assert_eq!(document("<<: x\n"), json!({"<<": "x"}));
}

/// A mapping whose member `a` holds ten scalars and each of eight more members ten aliases of
/// the one before: 10^9 values once expanded, a few hundred bytes of text.
pub(crate) fn alias_bomb() -> String {
    let mut text = String::from("kind: brief\na: &a [x, x, x, x, x, x, x, x, x, x]\n");
    let names = ["a", "b", "c", "d", "e", "f", "g", "h", "i"];
    for pair in names.windows(2) {
        let (previous, name) = (pair[0], pair[1]);
        let refs = vec![format!("*{previous}"); 10].join(", ");
        writeln!(text, "{name}: &{name} [{refs}]").unwrap();
    }
    text
}

#[test]
fn an_alias_bomb_is_refused_before_it_is_expanded() {
    let text = alias_bomb();
    assert!(text.len() < 1000);
    let (line, message) = refused(&text);
    assert!(message.contains(&MAX_VALUES.to_string()), "{message}");
    assert!((6..=10).contains(&line), "{line}");
}

#[test]
fn values_count_towards_the_bound_with_and_without_aliases() {
    // Exactly at the bound: the root, the key `a`, the sequence and its elements.
    let elements = MAX_VALUES - 3;
    let at_bound = format!("a: [{}]\n", vec!["1"; elements].join(","));
    assert_eq!(
        parsed(&at_bound).document["a"].as_array().unwrap().len(),
        elements
    );
    let beyond = format!("a: [{}]\n", vec!["1"; elements + 1].join(","));
    assert!(refused(&beyond).1.contains("values"));
    // An alias counts every value it repeats: the root; `s`, its sequence and its elements;
    // `t` and the sequence repeated; `u`, its sequence and its element.
    let with_alias = |n: usize| {
        let elements = vec!["1"; n].join(",");
        format!("s: &s [{elements}]\nt: *s\nu: [1]\n")
    };
    let n = (MAX_VALUES - 8) / 2;
    assert_eq!(1 + (n + 2) + (n + 2) + 3, MAX_VALUES);
    assert!(parse(&with_alias(n)).is_ok());
    let (line, message) = refused(&with_alias(n + 1));
    assert_eq!(line, 3);
    assert!(message.contains("aliases"), "{message}");
}

#[test]
fn nesting_is_bounded_with_and_without_aliases() {
    let nested = |levels: usize| {
        let mut text = String::new();
        for level in 0..levels - 1 {
            text.push_str(&" ".repeat(level));
            text.push_str("k:\n");
        }
        text.push_str(&" ".repeat(levels - 1));
        text.push_str("k: v\n");
        text
    };
    // The root mapping is level 1, each nested mapping one more, and the scalar one more.
    assert!(parse(&nested(MAX_NESTING - 1)).is_ok());
    let (line, message) = refused(&nested(MAX_NESTING));
    assert_eq!(line, u64::try_from(MAX_NESTING).unwrap());
    assert!(message.contains(&MAX_NESTING.to_string()), "{message}");
    let flow = format!(
        "a: {}{}\n",
        "[".repeat(MAX_NESTING),
        "]".repeat(MAX_NESTING)
    );
    assert!(refused(&flow).1.contains("nested"));
    // An alias adds the levels of its value where it is used.
    let deep = format!(
        "a: &d {}x{}\nb: [*d]\n",
        "[".repeat(MAX_NESTING - 2),
        "]".repeat(MAX_NESTING - 2)
    );
    let (line, message) = refused(&deep);
    assert_eq!(line, 2);
    assert!(message.contains("nested"), "{message}");
}

/// The refusal of `text` for the memory its document would take.
fn refused_for_memory(text: &str) -> String {
    let (_, message) = refused(text);
    assert!(
        message.contains(&MAX_DOCUMENT_BYTES.to_string()),
        "{message}"
    );
    message
}

#[test]
fn an_alias_of_a_large_scalar_is_counted_by_its_bytes() {
    let big = "x".repeat(500_000);
    let aliases = vec!["*a"; 80_000].join(",");
    let text = format!("a: &a {big}\nb: [{aliases}]\n");
    assert!(text.len() < 1 << 20);
    refused_for_memory(&text);
}

#[test]
fn a_mapping_anchor_repeated_by_many_aliases_is_counted_by_its_bytes() {
    let key = "k".repeat(400);
    let aliases = vec!["*a"; 30_000].join(",");
    let text = format!("a: &a {{{key}: x}}\nb: [{aliases}]\n");
    refused_for_memory(&text);
}

#[test]
fn the_pointers_of_lines_are_counted_by_their_bytes() {
    let key = "k".repeat(800_000);
    let children = vec!["1"; 99_000].join(",");
    let text = format!("? {key}\n: [{children}]\n");
    assert!(text.len() < 1 << 20);
    refused_for_memory(&text);
}

#[test]
fn the_copies_kept_for_nested_anchors_are_counted_by_their_bytes() {
    let mut text = String::from("a: ");
    for level in 0..58 {
        write!(text, "&a{level} [").unwrap();
    }
    text.push_str(&"x".repeat(290_000));
    text.push_str(&"]".repeat(58));
    text.push('\n');
    assert!(text.len() < 1 << 20);
    refused_for_memory(&text);
}

#[test]
fn a_float_tag_accepts_a_decimal_integer_and_messages_write_tags_as_yaml() {
    assert_eq!(
        document("a: !!float 1\nb: !!float -2e1\n"),
        json!({"a": 1.0, "b": -20.0})
    );
    let (_, message) = refused("a: !!float 0x1\n");
    assert!(message.contains("`!!float`"), "{message}");
    let (_, message) = refused("a: !local x\n");
    assert!(message.contains("`!local`"), "{message}");
    let (_, message) = refused("%TAG !e! tag:example.com,2026:\n---\na: !e!thing x\n");
    assert!(
        message.contains("`!<tag:example.com,2026:thing>`"),
        "{message}"
    );
}

#[test]
fn messages_quote_user_text_cut_to_80_characters() {
    let key = "k".repeat(200);
    let (_, message) = refused(&format!("{key}: 1\n{key}: 2\n"));
    assert!(
        message.contains(&format!("`{}...`", "k".repeat(80))),
        "{message}"
    );
    assert!(!message.contains(&"k".repeat(81)), "{message}");
    let number = format!("1e{}", "9".repeat(200));
    let (_, message) = refused(&format!("a: {number}\n"));
    assert!(message.len() < 200, "{message}");
}
