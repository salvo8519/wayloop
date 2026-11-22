//! `wayloop validate [<path>...]` end to end: the built binary runs in a temporary project and
//! prints the envelope the entry point gives for the input read from its files (SPEC-0001 R12;
//! plan of SPEC-0001, D1; tasks, T6).
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
    reason = "the test writes temporary projects and runs the built binary in them"
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as Process;

use serde_json::Value;
use wayloop_core::{Command, Context, envelope_json, run};

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A temporary project with `files`; removed when dropped.
struct Project(PathBuf);

impl Project {
    fn new(name: &str, files: &[(&str, String)]) -> Self {
        let root = std::env::temp_dir().join(format!("wayloop-cli-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        for (path, content) in files {
            let path = root.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, content).unwrap();
        }
        Self(root)
    }

    /// Standard output, as text and as JSON, and the exit code of `wayloop validate args`.
    fn validate(&self, args: &[&str]) -> (String, Value, i32) {
        let output = Process::new(env!("CARGO_BIN_EXE_wayloop"))
            .arg("validate")
            .args(args)
            .current_dir(&self.0)
            .output()
            .unwrap();
        let text = String::from_utf8(output.stdout).unwrap();
        let value = serde_json::from_str(&text).unwrap();
        (text, value, output.status.code().unwrap())
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn example(name: &str) -> String {
    fs::read_to_string(
        repository()
            .join("specs/0001-artifact-model/examples")
            .join(name),
    )
    .unwrap()
}

/// The two briefs of the examples, at their locations.
fn briefs(name: &str) -> Project {
    Project::new(
        name,
        &[
            ("plan/briefs/BRIEF-0001.yaml", example("brief.valid.yaml")),
            ("plan/briefs/BRIEF-0003.yaml", example("brief.invalid.yaml")),
            ("plan/briefs/README.md", "# Briefs\n".to_owned()),
        ],
    )
}

#[test]
fn validate_reads_the_project_and_prints_the_envelope_of_its_golden_input() {
    let case: Value = serde_json::from_str(
        &fs::read_to_string(repository().join("tests/contract/validate/v1/files-brief-pair.json"))
            .unwrap(),
    )
    .unwrap();
    let expected = run(Command::Validate, &case["input"], &Context::default());
    let (text, _, code) = briefs("discover").validate(&[]);
    assert_eq!(text, format!("{}\n", envelope_json(&expected).unwrap()));
    assert_eq!(code, 1);
}

#[test]
fn validate_reads_only_the_files_named_as_one_set() {
    let project = briefs("named");
    let (_, value, code) = project.validate(&[
        "./plan/briefs/BRIEF-0001.yaml",
        "plan/briefs/README.md",
        "plan/briefs/BRIEF-0001.yaml",
        "--command-version",
        "1",
    ]);
    assert_eq!(code, 0, "{value}");
    let documents = value["data"]["documents"].as_array().unwrap();
    assert_eq!(documents.len(), 2, "{value}");
    assert_eq!(
        documents[0]["origin"]["file"],
        "plan/briefs/BRIEF-0001.yaml"
    );
    assert_eq!(documents[0]["id"], "BRIEF-0001");
    assert_eq!(documents[1]["origin"]["file"], "plan/briefs/README.md");
}

#[test]
fn a_path_outside_the_project_is_a_usage_error_without_the_path() {
    let project = briefs("outside");
    let absolute = project.0.join("plan/briefs/BRIEF-0001.yaml");
    for path in [absolute.to_str().unwrap(), "../BRIEF-0001.yaml"] {
        let (text, value, code) = project.validate(&[path]);
        assert_eq!(code, 2, "{path}");
        assert_eq!(value["error"]["code"], "usage");
        assert!(!text.contains("BRIEF-0001"), "{text}");
    }
}

#[test]
fn a_named_file_that_does_not_exist_is_an_error_of_the_environment() {
    let (text, value, code) = briefs("missing").validate(&["plan/briefs/BRIEF-0009.yaml"]);
    assert_eq!(code, 3);
    assert_eq!(value["error"]["code"], "input_unreadable");
    assert_eq!(value["error"]["reason"], "missing");
    assert!(!text.contains("BRIEF-0009"), "{text}");
}

#[test]
fn a_project_without_artifacts_validates_as_an_empty_set() {
    let (_, value, code) = Project::new("empty", &[]).validate(&[]);
    assert_eq!(code, 0);
    assert_eq!(value["data"]["documents"], Value::Array(Vec::new()));
}
