//! `wayloop` command-line interface. An adapter: it parses arguments, calls the core and
//! prints JSON. It holds no decision logic (ADR 0001, ADR 0028).
//!
//! `wayloop validate --input <file|-> [--command-version <n>]` reads the canonical input,
//! passes its bytes to the entry point of the core with the requested version in the context,
//! prints the envelope exactly as the core writes it, and exits with the exit code of the
//! envelope. A command line that is not understood (exit code 2) and an input file that cannot
//! be read (exit code 3) also print an envelope, built by the core; the message for a person
//! goes to standard error.
//!
//! No path panics: `println!` would panic when standard output is closed, so the output is
//! written with `writeln!` and every failure becomes an exit code of ADR 0009 C7, read from the
//! one table of the numbers, `wayloop_core::ExitCode::code`.

#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "an adapter performs I/O: the lists of clippy.toml bind the pure crates only"
)]

use core::num::NonZeroU32;
use std::io::{self, Read, Write};
use std::process::ExitCode;

use serde_json::Value;
use wayloop_core::{
    Command, Context, Envelope, envelope_json, input_unreadable, run_json, usage_error,
};

/// The usage line, for a person, on standard error.
const USAGE: &str = "Commands: version, steps, validate --input <file|-> [--command-version <n>].";

/// What a run did: its exit code and a message for a person, if any, for standard error.
#[derive(Debug)]
struct Ran {
    code: ExitCode,
    message: Option<String>,
}

/// Why a run of the command line could not print its result, and the exit code of ADR 0009
/// C7 for each case.
#[derive(Debug)]
enum Failure {
    /// Standard output could not be written, for example a closed pipe: exit code 3.
    Output(io::Error),
    /// The envelope could not be written as JSON: exit code 5.
    Envelope(String),
}

impl Failure {
    /// The exit code, from the one table of the numbers.
    fn exit_code(&self) -> ExitCode {
        let code = match self {
            Failure::Output(_) => wayloop_core::ExitCode::EnvironmentError,
            Failure::Envelope(_) => wayloop_core::ExitCode::InternalError,
        };
        ExitCode::from(code.code())
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut stdout = io::stdout().lock();
    let (code, message) = match run(&args, &mut io::stdin().lock(), &mut stdout) {
        Ok(ran) => (ran.code, ran.message),
        Err(failure) => {
            let message = match &failure {
                Failure::Output(error) => format!("cannot write the output: {error}"),
                Failure::Envelope(error) => format!("cannot write the envelope: {error}"),
            };
            (failure.exit_code(), Some(message))
        }
    };
    if let Some(message) = message {
        // A failure to write the message to standard error leaves nothing else to report to:
        // the exit code still carries the outcome.
        let _ = writeln!(io::stderr().lock(), "wayloop: {message}");
    }
    code
}

/// Runs one command, reading standard input from `stdin`, and writes its JSON result to `out`.
fn run(args: &[String], stdin: &mut impl Read, out: &mut impl Write) -> Result<Ran, Failure> {
    let text = match args.first().map(String::as_str) {
        Some("validate") => return validate(args.get(1..).unwrap_or_default(), stdin, out),
        Some("--version" | "version") => format!(
            "{{\"name\":\"wayloop\",\"version\":\"{}\"}}",
            wayloop_core::VERSION
        ),
        Some("steps") => {
            let ids: Vec<String> = wayloop_core::IssueStep::ALL
                .iter()
                .map(|s| format!("\"{}\"", s.id()))
                .collect();
            format!("{{\"issue_steps\":[{}]}}", ids.join(","))
        }
        other => {
            let command = other.unwrap_or_default();
            let detail = if command.is_empty() {
                "No command was given."
            } else {
                "The command is not known."
            };
            return usage(command, detail, out);
        }
    };
    write(out, &text)?;
    Ok(Ran {
        code: ExitCode::from(wayloop_core::ExitCode::Success.code()),
        message: None,
    })
}

/// `wayloop validate` with its options `args`.
fn validate(args: &[String], stdin: &mut impl Read, out: &mut impl Write) -> Result<Ran, Failure> {
    let mut input = None;
    let mut context = Context::default();
    let mut version_given = false;
    for pair in args.chunks(2) {
        match pair {
            [flag, path] if flag == "--input" && input.is_none() => input = Some(path),
            [flag, n] if flag == "--command-version" && !version_given => {
                let Some(version) = n.parse::<u32>().ok().and_then(NonZeroU32::new) else {
                    let detail = "--command-version takes an integer from 1.";
                    return usage("validate", detail, out);
                };
                context = context.with_command_version(version);
                version_given = true;
            }
            _ => {
                let detail =
                    "The options are --input <file|-> and --command-version <n>, each once.";
                return usage("validate", detail, out);
            }
        }
    }
    let Some(path) = input else {
        return usage("validate", "--input <file|-> is required.", out);
    };
    let mut bytes = Vec::new();
    let read = if path == "-" {
        stdin.read_to_end(&mut bytes).map(|_| ())
    } else {
        std::fs::read(path).map(|content| bytes = content)
    };
    if let Err(error) = read {
        // The envelope holds no path and no message of the platform (SPEC-0004 R15); the
        // message for a person, on standard error, has both.
        let not_found = error.kind() == io::ErrorKind::NotFound;
        let detail = if not_found {
            "The input file does not exist."
        } else {
            "The input file cannot be read."
        };
        let envelope = input_unreadable(Command::Validate, detail, not_found);
        return print(
            &envelope,
            out,
            Some(format!("cannot read the input {path}: {error}")),
        );
    }
    let envelope = run_json(Command::Validate, &bytes, &context);
    print(&envelope, out, None)
}

/// Prints the envelope of a command line that is not understood, with the usage line.
fn usage(command: &str, detail: &str, out: &mut impl Write) -> Result<Ran, Failure> {
    let envelope = usage_error(command, detail);
    print(&envelope, out, Some(format!("{detail} {USAGE}")))
}

/// Prints `envelope` and gives its exit code, with `message` for standard error.
fn print(
    envelope: &Envelope<Value>,
    out: &mut impl Write,
    message: Option<String>,
) -> Result<Ran, Failure> {
    let text = envelope_json(envelope).map_err(|e| Failure::Envelope(e.to_string()))?;
    write(out, &text)?;
    Ok(Ran {
        code: ExitCode::from(envelope.exit_code().code()),
        message,
    })
}

/// Writes `text` and a new line to `out`.
fn write(out: &mut impl Write, text: &str) -> Result<(), Failure> {
    writeln!(out, "{text}").map_err(Failure::Output)?;
    out.flush().map_err(Failure::Output)
}

#[cfg(test)]
mod tests {
    use super::{Failure, run};
    use serde_json::Value;
    use std::io::{self, Write};
    use std::process::ExitCode;

    fn with_stdin(args: &[&str], stdin: &[u8]) -> Result<(String, ExitCode), Failure> {
        let args: Vec<String> = args.iter().map(ToString::to_string).collect();
        let mut out = Vec::new();
        let ran = run(&args, &mut &stdin[..], &mut out)?;
        Ok((String::from_utf8(out).unwrap(), ran.code))
    }

    fn output(args: &[&str]) -> String {
        with_stdin(args, b"").unwrap().0
    }

    /// The envelope printed for `args` and the exit code.
    fn envelope(args: &[&str], stdin: &[u8]) -> (Value, ExitCode) {
        let (text, code) = with_stdin(args, stdin).unwrap();
        assert!(
            text.ends_with('\n') && text.matches('\n').count() == 1,
            "{text}"
        );
        (serde_json::from_str(&text).unwrap(), code)
    }

    /// A sink that refuses every write, like a closed pipe.
    struct Closed;

    impl Write for Closed {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::from(io::ErrorKind::BrokenPipe))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn version_and_steps_print_one_json_line() {
        let version = output(&["version"]);
        assert_eq!(
            version,
            format!(
                "{{\"name\":\"wayloop\",\"version\":\"{}\"}}\n",
                wayloop_core::VERSION
            )
        );
        assert_eq!(output(&["--version"]), version);
        assert_eq!(with_stdin(&["steps"], b"").unwrap().1, ExitCode::SUCCESS);
        assert_eq!(
            output(&["steps"]),
            "{\"issue_steps\":[\"claim\",\"reanalysis\",\"counter_review\",\"checklist\",\
             \"build\",\"verify\",\"review\",\"pull_request\",\"merge\"]}\n"
        );
    }

    #[test]
    fn a_command_line_not_understood_prints_a_usage_envelope_and_exits_with_2() {
        for (args, command) in [
            (&[][..], ""),
            (&["nope"][..], "nope"),
            (&["validate"][..], "validate"),
            (&["validate", "--input"][..], "validate"),
            (&["validate", "--file", "-"][..], "validate"),
            (&["validate", "--input", "-", "extra"][..], "validate"),
            (
                &["validate", "--input", "-", "--input", "-"][..],
                "validate",
            ),
            (
                &["validate", "--input", "-", "--command-version", "0"][..],
                "validate",
            ),
            (
                &["validate", "--input", "-", "--command-version", "x"][..],
                "validate",
            ),
            (
                &[
                    "validate",
                    "--command-version",
                    "1",
                    "--command-version",
                    "1",
                    "--input",
                    "-",
                ][..],
                "validate",
            ),
        ] {
            let (value, code) = envelope(args, b"{}");
            assert_eq!(code, ExitCode::from(2), "{args:?}");
            assert_eq!(value["command"], command, "{args:?}");
            assert_eq!(value["error"]["code"], "usage", "{args:?}");
            assert_eq!(value["error"]["reason"], "syntax", "{args:?}");
            assert!(value.get("data").is_none(), "{args:?}");
        }
    }

    #[test]
    fn a_closed_output_is_an_environment_error_not_a_panic() {
        for args in [
            &["version"][..],
            &["validate", "--input", "-"][..],
            &["nope"][..],
        ] {
            let args: Vec<String> = args.iter().map(ToString::to_string).collect();
            let failure = run(&args, &mut &b"{}"[..], &mut Closed).unwrap_err();
            assert!(matches!(failure, Failure::Output(_)));
            assert_eq!(failure.exit_code(), ExitCode::from(3));
        }
    }

    #[test]
    fn validate_prints_the_envelope_and_exits_with_its_code() {
        let (value, code) = envelope(&["validate", "--input", "-"], br#"{"documents": []}"#);
        assert_eq!(value["data"]["documents"], serde_json::json!([]));
        assert_eq!(code, ExitCode::from(0));
        let (value, code) = envelope(&["validate", "--input", "-"], b"{");
        assert_eq!(value["error"]["reason"], "syntax");
        assert_eq!(code, ExitCode::from(1));
        let (value, code) = envelope(
            &["validate", "--command-version", "2", "--input", "-"],
            br#"{"documents": []}"#,
        );
        assert_eq!(value["error"]["code"], "command_input");
        assert_eq!(value["error"]["expected"], serde_json::json!([1]));
        assert_eq!(code, ExitCode::from(1));
    }

    #[test]
    fn an_input_file_that_cannot_be_read_prints_an_envelope_and_exits_with_3() {
        let path = "/nonexistent/wayloop/input.json";
        let (value, code) = envelope(&["validate", "--input", path], b"");
        assert_eq!(code, ExitCode::from(3));
        assert_eq!(value["error"]["code"], "input_unreadable");
        assert_eq!(value["error"]["reason"], "missing");
        assert!(!value.to_string().contains(path), "no path in the envelope");
        // A folder exists but cannot be read as a file.
        let (value, code) = envelope(&["validate", "--input", "/"], b"");
        assert_eq!(code, ExitCode::from(3));
        assert_eq!(value["error"]["reason"], "state");
    }

    #[test]
    fn a_failure_to_write_the_envelope_is_an_internal_error() {
        assert_eq!(
            Failure::Envelope(String::new()).exit_code(),
            ExitCode::from(5)
        );
    }
}
