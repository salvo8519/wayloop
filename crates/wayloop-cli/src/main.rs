//! `wayloop` command-line interface. An adapter: it parses arguments, calls the core and
//! prints JSON. It holds no decision logic (ADR 0001, ADR 0028).
//!
//! `wayloop validate [<path>...] [--command-version <n>]` reads the stored artifacts of the
//! project in the current folder with `wayloop-runtime`: every artifact at the locations of the
//! core, or the files named, as one set. `wayloop validate --input <file|->` reads the
//! canonical input as JSON instead. Either way the input goes to the entry point of the core
//! with the requested version in the context, and the command line prints the envelope exactly
//! as the core writes it and exits with the exit code of the envelope. A command line that is
//! not understood, a path named outside the project included (exit code 2), and an input or a
//! file that cannot be read (exit code 3) also print an envelope, built by the core; the
//! message for a person, with the path, goes to standard error.
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
use std::path::Path;
use std::process::ExitCode;

use serde_json::Value;
use wayloop_core::artifacts::locations;
use wayloop_core::{
    Command, Context, Envelope, Reason, envelope_json, input_unreadable, run_json, usage_error,
};
use wayloop_runtime::project;

/// The usage line, for a person, on standard error.
const USAGE_LINE: &str = "Commands: version, steps, \
     validate [<path>...] [--command-version <n>], \
     validate --input <file|-> [--command-version <n>].";

/// The options of `validate`, for a person.
const VALIDATE_OPTIONS: &str = "The options are --input <file|-> and --command-version <n>, each once, and paths without \
     --input.";

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
    /// The input read from the project could not be written as JSON: exit code 5.
    Input(String),
}

impl Failure {
    /// The exit code, from the one table of the numbers.
    fn exit_code(&self) -> ExitCode {
        let code = match self {
            Failure::Output(_) => wayloop_core::ExitCode::EnvironmentError,
            Failure::Envelope(_) | Failure::Input(_) => wayloop_core::ExitCode::InternalError,
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
                Failure::Input(error) => format!("cannot write the input: {error}"),
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
    let first = args.first().map(String::as_str);
    if first.and_then(Command::from_id) == Some(Command::Validate) {
        return validate(args.get(1..).unwrap_or_default(), stdin, out);
    }
    let text = match first {
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

/// `wayloop validate` with its arguments `args`.
fn validate(args: &[String], stdin: &mut impl Read, out: &mut impl Write) -> Result<Ran, Failure> {
    let command = Command::Validate;
    let mut input = None;
    let mut context = Context::default();
    let mut version_given = false;
    let mut paths = Vec::new();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--input" if input.is_none() => {
                let Some(path) = args.next() else {
                    return usage(command.id(), VALIDATE_OPTIONS, out);
                };
                input = Some(path);
            }
            "--command-version" if !version_given => {
                let Some(version) = args
                    .next()
                    .and_then(|n| n.parse::<u32>().ok())
                    .and_then(NonZeroU32::new)
                else {
                    let detail = "--command-version takes an integer from 1.";
                    return usage(command.id(), detail, out);
                };
                context = context.with_command_version(version);
                version_given = true;
            }
            option if option.starts_with("--") => {
                return usage(command.id(), VALIDATE_OPTIONS, out);
            }
            path => paths.push(path),
        }
    }
    match input {
        Some(_) if !paths.is_empty() => usage(command.id(), VALIDATE_OPTIONS, out),
        Some(path) => from_input(command, path, context, stdin, out),
        None => from_project(command, &paths, context, out),
    }
}

/// Validates the canonical input read from the file `path`, or from `stdin` for `-`.
fn from_input(
    command: Command,
    path: &str,
    context: Context,
    stdin: &mut impl Read,
    out: &mut impl Write,
) -> Result<Ran, Failure> {
    let mut bytes = Vec::new();
    let read = if path == "-" {
        stdin.read_to_end(&mut bytes).map(|_| ())
    } else {
        std::fs::read(path).map(|content| bytes = content)
    };
    if let Err(error) = read {
        return unreadable(
            command,
            &error,
            format!("cannot read the input {path}: {error}"),
            out,
        );
    }
    let envelope = run_json(command, &bytes, &context);
    print(&envelope, out, None)
}

/// Validates the stored artifacts of the project in the current folder: those at the
/// locations of the core, or the files at `paths`.
fn from_project(
    command: Command,
    paths: &[&str],
    context: Context,
    out: &mut impl Write,
) -> Result<Ran, Failure> {
    let root = Path::new(".");
    let files = if paths.is_empty() {
        project::discover(root)
    } else {
        let named = paths.iter().map(|p| project::segments(Path::new(p)));
        match locations::given(named) {
            Ok(files) => Ok(files),
            Err((index, refused)) => {
                let path = paths.get(index).copied().unwrap_or_default();
                let envelope = usage_error(command.id(), refused.message());
                return print(
                    &envelope,
                    out,
                    Some(format!("{path}: {refused} {USAGE_LINE}")),
                );
            }
        }
    };
    let input = match files.and_then(|files| project::load(root, files)) {
        Ok(input) => input,
        Err(error) => {
            let message = format!("cannot read {error}");
            return unreadable(command, error.error(), message, out);
        }
    };
    let input = serde_json::to_value(&input).map_err(|e| Failure::Input(e.to_string()))?;
    let envelope = wayloop_core::run(command, &input, &context);
    print(&envelope, out, None)
}

/// Prints the envelope of an input or a file that cannot be read, with `message` for a
/// person. The envelope holds no path and no message of the platform (SPEC-0004 R15).
fn unreadable(
    command: Command,
    error: &io::Error,
    message: String,
    out: &mut impl Write,
) -> Result<Ran, Failure> {
    let reason = if error.kind() == io::ErrorKind::NotFound {
        Reason::Missing
    } else {
        Reason::State
    };
    print(&input_unreadable(command, reason), out, Some(message))
}

/// Prints the envelope of a command line that is not understood, with the usage line.
fn usage(command: &str, detail: &str, out: &mut impl Write) -> Result<Ran, Failure> {
    let envelope = usage_error(command, detail);
    print(&envelope, out, Some(format!("{detail} {USAGE_LINE}")))
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
            (&["validate", "--input"][..], "validate"),
            (&["validate", "--command-version"][..], "validate"),
            (&["validate", "a.yaml", "--input", "-"][..], "validate"),
            (&["validate", "/etc/a.yaml"][..], "validate"),
            (&["validate", "../a.yaml"][..], "validate"),
            (&["validate", "plan/a.txt"][..], "validate"),
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
            assert!(
                !value.to_string().contains("/etc"),
                "no path in the envelope"
            );
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
        for failure in [
            Failure::Envelope(String::new()),
            Failure::Input(String::new()),
        ] {
            assert_eq!(failure.exit_code(), ExitCode::from(5));
        }
    }
}
