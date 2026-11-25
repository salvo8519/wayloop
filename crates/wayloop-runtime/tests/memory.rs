//! The bytes the runtime counts bound the memory a loaded set holds (SPEC-0001 plan, D1;
//! review of #319). The growth of the resident memory of the process (`/proc/self/status`,
//! Linux) measures what the input holds after `load`, with what the allocator kept of the
//! work; the test binary holds this one test, so no other test allocates at the same time.
#![cfg(target_os = "linux")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "the test writes temporary projects and reads them back"
)]

use std::fmt::Write as _;

use wayloop_core::artifacts::locations::{MAX_DOCUMENT_BYTES, MAX_PROJECT_BYTES};
use wayloop_runtime::project::{discover, project_input};
use wayloop_runtime::test_support::TempProject;
use wayloop_schemas::validate::EntryFormat;

/// The resident memory of this process, in bytes (`VmRSS` of `/proc/self/status`).
fn resident() -> usize {
    let status = std::fs::read_to_string("/proc/self/status").unwrap();
    let line = status.lines().find(|l| l.starts_with("VmRSS:")).unwrap();
    let kib: usize = line.split_whitespace().nth(1).unwrap().parse().unwrap();
    kib * 1024
}

/// A YAML mapping of `keys` distinct three-character keys, each with the value `1`.
fn short_keys(keys: usize) -> String {
    const DIGITS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut text = String::new();
    for n in 0..keys {
        let key: String = [n / 3844, n / 62 % 62, n % 62]
            .iter()
            .map(|d| char::from(DIGITS[*d]))
            .collect();
        writeln!(text, "{key}: 1").unwrap();
    }
    text
}

/// The heap held by the input of `files` once loaded, and its formats.
fn held(files: &[(&str, &[u8])]) -> (usize, Vec<EntryFormat>) {
    let project = TempProject::new("memory", files);
    let found = discover(project.root()).unwrap().files;
    let before = resident();
    let input = project_input(project.root(), found).unwrap();
    let held = resident().saturating_sub(before);
    let formats = input.documents.iter().map(|d| d.format).collect();
    (held, formats)
}

#[test]
fn the_counted_bytes_bound_the_memory_of_documents_and_of_the_set() {
    // One document of 20,000 short keys, the most of them a document holds: it is loaded and
    // holds no more than its bound.
    let one = short_keys(20_000);
    let (bytes, formats) = held(&[("plan/items/a.yaml", one.as_bytes())]);
    assert_eq!(formats, [EntryFormat::Document]);
    assert!(bytes <= MAX_DOCUMENT_BYTES, "{bytes} bytes held");
    // With 49,000 keys it is refused.
    let (_, formats) = held(&[("plan/items/a.yaml", short_keys(49_000).as_bytes())]);
    assert_eq!(formats, [EntryFormat::Unreadable]);

    // Many files of short keys, more than the set holds: the later ones are refused, and the
    // set holds no more than its bound.
    let each = short_keys(20_000);
    let names: Vec<String> = (0..40)
        .map(|n| format!("plan/items/i{n:02}.yaml"))
        .collect();
    let files: Vec<(&str, &[u8])> = names
        .iter()
        .map(|n| (n.as_str(), each.as_bytes()))
        .collect();
    let (bytes, formats) = held(&files);
    assert!(
        bytes <= MAX_PROJECT_BYTES,
        "{bytes} bytes held for {MAX_PROJECT_BYTES}"
    );
    assert_eq!(formats[0], EntryFormat::Document);
    assert_eq!(formats.last(), Some(&EntryFormat::Unreadable));
}
