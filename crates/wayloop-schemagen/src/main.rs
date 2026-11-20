//! `wayloop-schemagen`: prints the snapshots of the current version of every contract as one
//! JSON document on standard output, and writes no file (ADR 0030 G4). Run by
//! `scripts/schema_snapshots.py` through `uv run just generate`.
//!
//! No path panics: the document is written with `writeln!`, and a failure is printed on
//! standard error with exit code 1.

use std::io::{self, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let result = wayloop_schemagen::generate()
        .map_err(|fault| fault.to_string())
        .and_then(|document| {
            writeln!(io::stdout().lock(), "{document}").map_err(|error| error.to_string())
        });
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            // Nothing more can be done when standard error is closed too.
            let _ = writeln!(io::stderr().lock(), "wayloop-schemagen: {message}");
            ExitCode::FAILURE
        }
    }
}
