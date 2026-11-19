//! `wayloop` command-line interface. An adapter: it parses arguments, calls the core and
//! prints JSON. It holds no decision logic (ADR 0001).
//!
//! No path panics: `println!` would panic when standard output is closed, so the output is
//! written with `writeln!` and every failure becomes an exit code of ADR 0009 C7.

use std::io::{self, Write};
use std::process::ExitCode;

/// Why a run of the command line failed, and the exit code of ADR 0009 C7 for each case.
#[derive(Debug)]
enum Failure {
    /// The arguments name no known command: exit code 2.
    Usage,
    /// Standard output could not be written, for example a closed pipe: exit code 3.
    Output(io::Error),
}

impl Failure {
    fn exit_code(&self) -> ExitCode {
        match self {
            Failure::Usage => ExitCode::from(2),
            Failure::Output(_) => ExitCode::from(3),
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut stdout = io::stdout().lock();
    match run(&args, &mut stdout) {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            let message = match &failure {
                Failure::Usage => format!(
                    "wayloop {}: pre-alpha. Commands: version, steps.",
                    wayloop_core::VERSION
                ),
                Failure::Output(error) => format!("wayloop: cannot write the output: {error}"),
            };
            // A failure to write the message to standard error leaves nothing else to report
            // to: the exit code still carries the failure.
            let _ = writeln!(io::stderr().lock(), "{message}");
            failure.exit_code()
        }
    }
}

/// Runs one command and writes its JSON result to `out`.
fn run(args: &[String], out: &mut impl Write) -> Result<(), Failure> {
    let text = match args.first().map(String::as_str) {
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
        _ => return Err(Failure::Usage),
    };
    writeln!(out, "{text}").map_err(Failure::Output)?;
    out.flush().map_err(Failure::Output)
}

#[cfg(test)]
mod tests {
    use super::{Failure, run};
    use std::io::{self, Write};
    use std::process::ExitCode;

    fn output(args: &[&str]) -> Result<String, Failure> {
        let args: Vec<String> = args.iter().map(ToString::to_string).collect();
        let mut out = Vec::new();
        run(&args, &mut out)?;
        Ok(String::from_utf8(out).unwrap())
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
        let version = output(&["version"]).unwrap();
        assert_eq!(
            version,
            format!(
                "{{\"name\":\"wayloop\",\"version\":\"{}\"}}\n",
                wayloop_core::VERSION
            )
        );
        assert_eq!(output(&["--version"]).unwrap(), version);
        assert_eq!(
            output(&["steps"]).unwrap(),
            "{\"issue_steps\":[\"claim\",\"reanalysis\",\"counter_review\",\"checklist\",\
             \"build\",\"verify\",\"review\",\"pull_request\",\"merge\"]}\n"
        );
    }

    #[test]
    fn an_unknown_command_is_a_usage_error() {
        for args in [&[][..], &["nope"][..]] {
            let failure = output(args).unwrap_err();
            assert!(matches!(failure, Failure::Usage));
            assert_eq!(failure.exit_code(), ExitCode::from(2));
        }
    }

    #[test]
    fn a_closed_output_is_an_environment_error_not_a_panic() {
        let failure = run(&["version".to_string()], &mut Closed).unwrap_err();
        assert!(matches!(failure, Failure::Output(_)));
        assert_eq!(failure.exit_code(), ExitCode::from(3));
    }
}
