//! The binary prints the document of `generate` on standard output, and nothing else.
#![allow(clippy::unwrap_used)]

use std::process::Command;

#[test]
fn the_binary_prints_the_generated_document() {
    let output = Command::new(env!("CARGO_BIN_EXE_wayloop-schemagen"))
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let printed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(printed, wayloop_schemagen::generate().unwrap());
}
