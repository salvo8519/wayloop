//! `wayloop` command-line interface. An adapter: it parses arguments, calls the core and
//! prints JSON. It holds no decision logic (ADR 0001).

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--version" | "version") => {
            println!(
                "{{\"name\":\"wayloop\",\"version\":\"{}\"}}",
                wayloop_core::VERSION
            );
            ExitCode::SUCCESS
        }
        Some("steps") => {
            let ids: Vec<String> = wayloop_core::IssueStep::ALL
                .iter()
                .map(|s| format!("\"{}\"", s.id()))
                .collect();
            println!("{{\"issue_steps\":[{}]}}", ids.join(","));
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!(
                "wayloop {}: pre-alpha. Commands: version, steps.",
                wayloop_core::VERSION
            );
            ExitCode::from(2)
        }
    }
}
