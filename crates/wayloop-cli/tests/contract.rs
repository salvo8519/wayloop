//! Contract cases of `wayloop validate`, version 1 (`tests/contract/validate/v1/`, SPEC-0001
//! plan D6; format in `tests/contract/README.md`).
//!
//! Each case is sent to the entry point of the core, with the context it names, and to the
//! built binary. The test checks the expectation of the case on the envelope of the entry
//! point; checks that envelope against the open output snapshot of the command, and against
//! the schema of the same type generated for the input direction, where every object is
//! closed, so that the engine writes no member its contract does not describe; reads each
//! envelope back into its type and each accepted input into `ValidateInput` (ADR 0030 G1);
//! recomputes `meta.input_hash` with its own canonical JSON and the `sha2` crate, independently
//! of the core; and checks that the binary prints the same envelope byte for byte and exits
//! with its code (SPEC-0004 R14, R16, AC10).
// A test stops at the line that failed: the restriction lints of the shipped code do not apply.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "the runner reads the case files and runs the built binary: a test of an adapter"
)]

use core::fmt::Write as _;
use core::num::NonZeroU32;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command as Process, Stdio};

use serde_json::Value;
use wayloop_core::schema::Schema;
use wayloop_core::{Code, Command, Context, Envelope, envelope_json, run, run_json};
use wayloop_schemas::contract::{Direction, Part, VALIDATE, reread, schema_for};
use wayloop_schemas::validate::{ValidateData, ValidateInput};

/// The text of the current output snapshot of `validate`.
fn output_snapshot() -> &'static str {
    VALIDATE
        .embedded(Part::Output, VALIDATE.version)
        .unwrap()
        .text
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every case of `validate` v1, by file name.
fn cases() -> BTreeMap<String, Value> {
    let dir = root().join("tests/contract/validate/v1");
    let mut cases = BTreeMap::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "json") {
            let text = std::fs::read_to_string(&path).unwrap();
            let name = path.file_stem().unwrap().to_str().unwrap().to_owned();
            cases.insert(name, serde_json::from_str(&text).unwrap());
        }
    }
    assert_ne!(cases.len(), 0, "no case in {}", dir.display());
    cases
}

/// The bytes the case sends: its text, or its input written as JSON.
fn bytes(case: &Value) -> Vec<u8> {
    match (case.get("input"), case.get("input_text")) {
        (Some(input), None) => serde_json::to_vec(input).unwrap(),
        (None, Some(Value::String(text))) => text.clone().into_bytes(),
        _ => panic!("a case has exactly one of input and input_text"),
    }
}

/// The requested version of the command, from the member `context` of the case.
fn version(case: &Value) -> Option<NonZeroU32> {
    let version = case.get("context")?.get("command_version")?.as_u64()?;
    NonZeroU32::new(u32::try_from(version).unwrap())
}

/// The arguments of the binary for the case, after `validate --input <path>`.
fn context_args(case: &Value) -> Vec<String> {
    version(case).map_or_else(Vec::new, |v| {
        vec!["--command-version".to_owned(), v.to_string()]
    })
}

/// The envelope of the entry point for the case.
fn envelope(case: &Value) -> Envelope<Value> {
    let context = version(case).map_or_else(Context::default, |v| {
        Context::default().with_command_version(v)
    });
    let from_bytes = run_json(Command::Validate, &bytes(case), &context);
    if let Some(input) = case.get("input") {
        let from_value = run(Command::Validate, input, &context);
        assert_eq!(from_value, from_bytes, "the value and its text disagree");
    }
    from_bytes
}

fn check_expectation(name: &str, case: &Value, envelope: &Envelope<Value>) {
    let expect = &case["expect"];
    let written: Value = serde_json::from_str(&envelope_json(envelope).unwrap()).unwrap();
    assert_eq!(
        Value::from(envelope.exit_code().code()),
        expect["exit_code"],
        "{name}: exit code of {written}"
    );
    for (pointer, value) in expect["values"].as_object().unwrap() {
        assert_eq!(
            written.pointer(pointer),
            Some(value),
            "{name}: {pointer} of {written}"
        );
    }
    for pointer in expect["absent"].as_array().unwrap() {
        let pointer = pointer.as_str().unwrap();
        assert_eq!(
            written.pointer(pointer),
            None,
            "{name}: {pointer} of {written}"
        );
    }
    for (pointer, length) in expect["lengths"].as_object().unwrap() {
        let found = written
            .pointer(pointer)
            .and_then(Value::as_array)
            .map(Vec::len);
        assert_eq!(
            found.map(Value::from).as_ref(),
            Some(length),
            "{name}: {pointer} of {written}"
        );
    }
}

/// Runs the binary with `args`, `stdin` on its standard input; gives its standard output and
/// exit code.
fn binary(args: &[&str], stdin: &[u8]) -> (String, i32) {
    let mut child = Process::new(env!("CARGO_BIN_EXE_wayloop"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(stdin).unwrap();
    let output = child.wait_with_output().unwrap();
    (
        String::from_utf8(output.stdout).unwrap(),
        output.status.code().unwrap(),
    )
}

#[test]
fn every_case_meets_its_expectation_on_the_entry_point() {
    for (name, case) in cases() {
        check_expectation(&name, &case, &envelope(&case));
    }
}

/// The open output snapshot, and the strict schema of the same type: generated for the input
/// direction, every object closed. The strict one is a test, not a contract, and is not
/// written to a file: it comes from the one type.
fn schemas() -> [Schema; 2] {
    let open: Value = serde_json::from_str(output_snapshot()).unwrap();
    let strict = schema_for::<Envelope<ValidateData>>(Direction::Input).unwrap();
    [open, strict].map(|schema| Schema::load(&schema).unwrap())
}

#[test]
fn every_envelope_conforms_to_the_output_snapshot_and_its_closed_form() {
    let [open, strict] = schemas();
    for (name, case) in cases() {
        let written: Value =
            serde_json::from_str(&envelope_json(&envelope(&case)).unwrap()).unwrap();
        assert_eq!(
            open.validate(&written, Code::new("output")),
            [],
            "{name}: {written}"
        );
        assert_eq!(
            strict.validate(&written, Code::new("strict")),
            [],
            "{name}: {written}"
        );
    }
    assert!(!output_snapshot().contains("additionalProperties"));
}

#[test]
fn every_envelope_and_every_accepted_input_read_into_their_types() {
    // ADR 0030 G1, both ways: what the engine writes reads back into its type unchanged, and
    // an input that passed the schema and the rules of the input reads into `ValidateInput`,
    // without its members set to null, which are not given (SPEC-0001 R1).
    for (name, case) in cases() {
        let written: Value =
            serde_json::from_str(&envelope_json(&envelope(&case)).unwrap()).unwrap();
        assert_eq!(
            reread::<Envelope<ValidateData>>(&written).unwrap(),
            written,
            "{name}"
        );
        let refused_input = written["error"]["code"] == "command_input";
        if let (Some(input), false) = (case.get("input"), refused_input) {
            let mut given = String::new();
            canonical(input, &mut given);
            let given: Value = serde_json::from_str(&given).unwrap();
            let read = reread::<ValidateInput>(&given)
                .unwrap_or_else(|e| panic!("{name}: the input does not read: {e}"));
            assert!(read.is_object(), "{name}");
        }
    }
}

/// The canonical JSON of RFC 8785 for the values of the cases, written apart from the core:
/// members without null, keys sorted by UTF-16 code units, integers only (a case with another
/// number fails here), strings escaped as `JSON.stringify` escapes them.
fn canonical(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => {
            let text = n
                .as_i64()
                .map(|i| i.to_string())
                .or_else(|| n.as_u64().map(|u| u.to_string()))
                .expect("integers only");
            out.push_str(&text);
        }
        Value::String(text) => {
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
                    c if u32::from(c) < 0x20 => write!(out, "\\u{:04x}", u32::from(c)).unwrap(),
                    c => out.push(c),
                }
            }
            out.push('"');
        }
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                canonical(item, out);
            }
            out.push(']');
        }
        Value::Object(members) => {
            let mut members: Vec<(&String, &Value)> =
                members.iter().filter(|(_, v)| !v.is_null()).collect();
            members.sort_by(|a, b| a.0.encode_utf16().cmp(b.0.encode_utf16()));
            out.push('{');
            for (i, (key, member)) in members.into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                canonical(&Value::String(key.clone()), out);
                out.push(':');
                canonical(member, out);
            }
            out.push('}');
        }
    }
}

/// The input hash of every case, computed apart from the core (SPEC-0004 R11): the SHA-256 of
/// the canonical JSON of `input`, or of the bytes of `input_text`, which the cases use only for
/// text the engine refuses to read as JSON (not JSON, a repeated key, a number beyond a double).
#[test]
fn the_input_hash_of_every_case_agrees_with_an_independent_computation() {
    use sha2::{Digest, Sha256};
    let (mut json, mut text) = (0, 0);
    for (name, case) in cases() {
        let hashed = if let Some(input) = case.get("input") {
            let mut written = String::new();
            canonical(input, &mut written);
            json += 1;
            written
        } else {
            text += 1;
            case["input_text"].as_str().unwrap().to_owned()
        };
        let digest = Sha256::digest(hashed.as_bytes());
        let hex = digest.iter().fold(String::new(), |mut hex, byte| {
            write!(hex, "{byte:02x}").unwrap();
            hex
        });
        assert_eq!(envelope(&case).meta().input_hash().as_str(), hex, "{name}");
        assert_eq!(case["expect"]["values"]["/meta/input_hash"], hex, "{name}");
    }
    assert!(json > 0 && text > 0);
    // Members set to null are not given: the two cases differ only by nulls.
    let all = cases();
    assert_eq!(
        all["null-is-absent"]["expect"]["values"]["/meta/input_hash"],
        all["brief-valid"]["expect"]["values"]["/meta/input_hash"]
    );
}

#[test]
fn the_binary_prints_the_envelope_of_the_entry_point_byte_for_byte() {
    let dir = std::env::temp_dir().join(format!("wayloop-contract-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for (name, case) in cases() {
        let expected = envelope(&case);
        let text = format!("{}\n", envelope_json(&expected).unwrap());
        let code = i32::from(expected.exit_code().code());
        let file = dir.join(format!("{name}.json"));
        std::fs::write(&file, bytes(&case)).unwrap();
        let extra = context_args(&case);
        let mut args = vec!["validate", "--input", file.to_str().unwrap()];
        args.extend(extra.iter().map(String::as_str));
        let (stdout, exit) = binary(&args, b"");
        assert_eq!((stdout.as_str(), exit), (text.as_str(), code), "{name}");
        let mut args = vec!["validate", "--input", "-"];
        args.extend(extra.iter().map(String::as_str));
        let (stdout, exit) = binary(&args, &bytes(&case));
        assert_eq!(
            (stdout.as_str(), exit),
            (text.as_str(), code),
            "{name} on standard input"
        );
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn usage_and_unreadable_input_print_an_envelope_with_exit_codes_2_and_3() {
    // M7: the binary always prints an envelope; the message for a person is on stderr.
    let [open, strict] = schemas();
    for (args, code, error) in [
        (&["validate", "--input"][..], 2, "usage"),
        (&["validate", "/nonexistent/wayloop.yaml"][..], 2, "usage"),
        (
            &["validate", "--input", "-", "--command-version", "0"][..],
            2,
            "usage",
        ),
        (
            &["validate", "--input", "/nonexistent/wayloop.json"][..],
            3,
            "input_unreadable",
        ),
        (&["nope"][..], 2, "usage"),
    ] {
        let (stdout, exit) = binary(args, b"");
        assert_eq!(exit, code, "{args:?}");
        let written: Value = serde_json::from_str(&stdout).unwrap();
        assert_eq!(written["error"]["code"], error, "{args:?}");
        assert!(written.get("data").is_none());
        assert_eq!(open.validate(&written, Code::new("output")), [], "{args:?}");
        if args[0] == "validate" {
            assert_eq!(
                strict.validate(&written, Code::new("strict")),
                [],
                "{args:?}"
            );
        }
    }
}

#[test]
fn a_case_names_what_it_covers_and_nothing_else() {
    let allowed = [
        "description",
        "covers",
        "context",
        "input",
        "input_text",
        "expect",
    ];
    let expect = ["exit_code", "values", "absent", "lengths"];
    for (name, case) in cases() {
        let keys: Vec<&String> = case.as_object().unwrap().keys().collect();
        assert!(keys.iter().all(|k| allowed.contains(&k.as_str())), "{name}");
        assert!(case["description"].is_string(), "{name}");
        assert_ne!(
            case["covers"].as_array().unwrap().len(),
            0,
            "{name}: covers"
        );
        let given: Vec<&String> = case["expect"].as_object().unwrap().keys().collect();
        assert_eq!(given.len(), expect.len(), "{name}");
        assert!(given.iter().all(|k| expect.contains(&k.as_str())), "{name}");
    }
}

#[test]
fn the_maximal_case_gives_every_member_of_the_input() {
    // Review of #320: a new member of the input must reach the maximal case, as its
    // description says, or this test names it.
    let schema: Value = serde_json::from_str(
        VALIDATE
            .embedded(Part::Schema, VALIDATE.version)
            .unwrap()
            .text,
    )
    .unwrap();
    let declared = |schema: &Value| -> Vec<String> {
        let mut keys: Vec<String> = schema["properties"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        keys.sort();
        keys
    };
    let input = &cases()["maximal"]["input"];
    let given: Vec<String> = input.as_object().unwrap().keys().cloned().collect();
    assert_eq!(given, declared(&schema));
    let mut members: Vec<String> = input["documents"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|entry| entry.as_object().unwrap().keys().cloned())
        .collect();
    members.sort();
    members.dedup();
    let entry = declared(&schema["$defs"]["DocumentEntry"]);
    let not_in_case: Vec<&String> = entry.iter().filter(|m| !members.contains(m)).collect();
    // `text` and `error` belong to other formats, `input_kind` to inputs, not to artifacts.
    assert_eq!(not_in_case, ["error", "input_kind", "text"]);
}
