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

use serde_json::{Value, json};
use wayloop_core::{Command, Context, envelope_json, run};
use wayloop_runtime::test_support::{TempProject, ran};

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A temporary project of the runtime's test support, where the binary runs.
struct Project(TempProject);

impl Project {
    fn new(name: &str, files: &[(&str, String)]) -> Self {
        let files: Vec<(&str, &[u8])> = files.iter().map(|(p, c)| (*p, c.as_bytes())).collect();
        Self(TempProject::new(&format!("cli-{name}"), &files))
    }

    /// Standard output, as text and as JSON, standard error, and the exit code of
    /// `wayloop validate args`.
    fn validate(&self, args: &[&str]) -> (String, Value, String, i32) {
        self.validate_in("", args)
    }

    /// `wayloop validate` with `args`, run in the folder `below` of the project.
    fn validate_in(&self, below: &str, args: &[&str]) -> (String, Value, String, i32) {
        let output = Process::new(env!("CARGO_BIN_EXE_wayloop"))
            .arg("validate")
            .args(args)
            .current_dir(self.0.root().join(below))
            .output()
            .unwrap();
        let text = String::from_utf8(output.stdout).unwrap();
        let value = serde_json::from_str(&text).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        (text, value, stderr, output.status.code().unwrap())
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
    let (text, _, _, code) = briefs("discover").validate(&[]);
    assert_eq!(text, format!("{}\n", envelope_json(&expected).unwrap()));
    assert_eq!(code, 1);
}

#[test]
fn validate_warns_when_it_runs_outside_the_root_of_the_project() {
    // SPEC-0001 AC37: from a folder of the project, discovery finds no location and no file.
    let project = briefs("subfolder");
    let (_, envelope, _, code) = project.validate_in("plan", &[]);
    assert_eq!(code, 0);
    assert_eq!(envelope["data"]["documents"], json!([]));
    let warnings = envelope["diagnostics"]["warnings"].as_array().unwrap();
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0]["code"], "artifact_location_missing");
    assert_eq!(warnings[0]["pointer"], "/locations");
    // Named files give no locations: the check is not done, and nothing is warned.
    let (_, envelope, _, _) = project.validate(&["plan/briefs/BRIEF-0001.yaml"]);
    assert_eq!(envelope["diagnostics"]["warnings"], json!([]));
    let not_done = envelope["diagnostics"]["not_done"].as_array().unwrap();
    let absent = json!({"check": "artifact_locations", "reason": "input_absent"});
    assert!(not_done.contains(&absent), "{not_done:?}");
}

#[test]
fn validate_reads_only_the_files_named_as_one_set() {
    // A named Markdown file is a living specification, normalised by the core (SPEC-0001 R12):
    // the specification of the artifact model, in the form it defines. Its milestone M0 is
    // named too, as a plan item, so that its link resolves (R3).
    let spec = "specs/0001-artifact-model/spec.md";
    let milestone = "plan/items/M0.yaml";
    let m0 = "kind: milestone\nschema_version: 1\nid: M0\ntitle: Foundations\nstatus: draft\n\
              gate: The artifact model validates.\n";
    let project = Project::new(
        "named",
        &[
            ("plan/briefs/BRIEF-0001.yaml", example("brief.valid.yaml")),
            ("plan/briefs/BRIEF-0003.yaml", example("brief.invalid.yaml")),
            (milestone, m0.to_owned()),
            (spec, fs::read_to_string(repository().join(spec)).unwrap()),
        ],
    );
    let (_, value, _, code) = project.validate(&[
        "./plan/briefs/BRIEF-0001.yaml",
        spec,
        milestone,
        "plan/briefs/BRIEF-0001.yaml",
        "--command-version",
        "1",
    ]);
    assert_eq!(code, 0, "{value}");
    let documents = value["data"]["documents"].as_array().unwrap();
    assert_eq!(documents.len(), 3, "{value}");
    // In the byte order of their paths.
    let read: Vec<(&Value, &Value)> = documents
        .iter()
        .map(|d| (&d["origin"]["file"], &d["id"]))
        .collect();
    assert_eq!(
        read,
        [
            (&json!("plan/briefs/BRIEF-0001.yaml"), &json!("BRIEF-0001")),
            (&json!(milestone), &json!("M0")),
            (&json!(spec), &json!("SPEC-0001")),
        ]
    );
}

#[test]
fn validate_checks_the_adr_citations_of_the_repository() {
    // #344: run at the root of this repository, the command reads the numbers of its ADRs
    // from `docs/en/project/adr/`, so the check `adr_citations` runs on SPEC-0001.
    let output = Process::new(env!("CARGO_BIN_EXE_wayloop"))
        .args(["validate", "specs/0001-artifact-model/spec.md"])
        .current_dir(repository())
        .output()
        .unwrap();
    let envelope: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(ran(&envelope, "adr_citations"), "{envelope}");
}

/// A living specification in Markdown that cites `ADR 0002, 0003` and `ADR 0014`, with its
/// milestone M0 at its location.
const CITING_SPEC: &str = "# SPEC-0001: Time entries

| | |
| --- | --- |
| Status | Draft |
| Milestone | M0 · Foundations |
| Owner | @example-owner |
| Related | ADR 0002, 0003; ADR 0014 |
| Commit scope | core |

## Problem
Freelancers need to record time without leaving the terminal.

## Scope
**In scope:** Recording time entries
**Out of scope:** Export

## User stories
- As a freelance developer, I want to record time, so that I bill it.

## Requirements (EARS)
- **R1**: THE TOOL SHALL store durations in minutes.

## Acceptance criteria
- **AC1** (R1): Given an empty log, when 90 minutes are recorded, then the log holds 90 minutes. Verification: TDD.
";

#[test]
fn validate_warns_of_a_cited_adr_the_project_does_not_have() {
    // SPEC-0001 Q5 through the command line: of the three ADRs cited, only 0003 exists, so
    // exactly 0002 and 0014 are warned.
    let m0 = "kind: milestone\nschema_version: 1\nid: M0\ntitle: Foundations\nstatus: draft\n\
              gate: The artifact model validates.\n";
    let project = Project::new(
        "adr-citations",
        &[
            ("plan/items/M0.yaml", m0.to_owned()),
            ("specs/0001-time-entries/spec.md", CITING_SPEC.to_owned()),
            (
                "docs/en/project/adr/0003-ledger.md",
                "# ADR 0003\n".to_owned(),
            ),
            ("docs/en/project/adr/index.md", "# ADRs\n".to_owned()),
        ],
    );
    let (_, envelope, _, code) = project.validate(&[]);
    assert_eq!(code, 0, "{envelope}");
    assert!(ran(&envelope, "adr_citations"), "{envelope}");
    let warnings: Vec<(&Value, &Value)> = envelope["diagnostics"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| (&w["code"], &w["message"]))
        .collect();
    assert_eq!(
        warnings,
        [
            (
                &json!("adr_citation_missing"),
                &json!("SPEC-0001 cites ADR 0002, which is not an ADR of the project.")
            ),
            (
                &json!("adr_citation_missing"),
                &json!("SPEC-0001 cites ADR 0014, which is not an ADR of the project.")
            ),
        ],
        "{envelope}"
    );
    // Without the folder, the check is not done.
    let (_, envelope, _, _) = Project::new("no-adr-folder", &[]).validate(&[]);
    let not_done = envelope["diagnostics"]["not_done"].as_array().unwrap();
    let absent = json!({"check": "adr_citations", "reason": "input_absent"});
    assert!(not_done.contains(&absent), "{not_done:?}");
}

#[test]
fn a_path_outside_the_project_is_a_usage_error_without_the_path() {
    let project = briefs("outside");
    let absolute = project.0.root().join("plan/briefs/BRIEF-0001.yaml");
    for path in [absolute.to_str().unwrap(), "../BRIEF-0001.yaml"] {
        let (text, value, _, code) = project.validate(&[path]);
        assert_eq!(code, 2, "{path}");
        assert_eq!(value["error"]["code"], "usage");
        assert!(!text.contains("BRIEF-0001"), "{text}");
    }
}

#[test]
fn a_named_file_that_does_not_exist_is_an_error_of_the_environment() {
    let (text, value, _, code) = briefs("missing").validate(&["plan/briefs/BRIEF-0009.yaml"]);
    assert_eq!(code, 3);
    assert_eq!(value["error"]["code"], "input_unreadable");
    assert_eq!(value["error"]["reason"], "missing");
    assert!(!text.contains("BRIEF-0009"), "{text}");
}

#[test]
fn a_project_without_artifacts_validates_as_an_empty_set() {
    let (_, value, _, code) = Project::new("empty", &[]).validate(&[]);
    assert_eq!(code, 0);
    assert_eq!(value["data"]["documents"], Value::Array(Vec::new()));
}

#[cfg(unix)]
#[test]
fn a_named_link_out_of_the_project_is_a_usage_error_and_stderr_is_escaped() {
    let outside = TempProject::new("cli-creds", &[("creds.yaml", b"secret: x\n")]);
    let project = briefs("link-out");
    let link = project.0.root().join("plan/briefs/x\u{1b}.yaml");
    std::os::unix::fs::symlink(outside.root().join("creds.yaml"), link).unwrap();
    let (text, value, stderr, code) = project.validate(&["plan/briefs/x\u{1b}.yaml"]);
    assert_eq!(code, 2, "{text}");
    assert_eq!(value["error"]["code"], "usage");
    assert!(
        !text.contains("creds") && !text.contains("x\u{1b}"),
        "{text}"
    );
    assert!(stderr.contains("plan/briefs/x\\u{1b}.yaml"), "{stderr}");
    assert!(!stderr.contains('\u{1b}'), "{stderr}");
}

#[test]
fn a_project_of_more_than_max_files_is_a_usage_error() {
    use wayloop_core::artifacts::locations::MAX_FILES;
    let project = Project::new("many", &[]);
    let items = project.0.root().join("plan/items");
    fs::create_dir_all(&items).unwrap();
    for n in 0..=MAX_FILES {
        fs::write(items.join(format!("i{n:05}.yaml")), b"").unwrap();
    }
    let (text, value, stderr, code) = project.validate(&[]);
    assert_eq!(code, 2, "{text}");
    assert_eq!(value["error"]["code"], "usage");
    let message = value["error"]["message"].as_str().unwrap();
    assert!(message.contains(&MAX_FILES.to_string()), "{message}");
    assert!(stderr.contains(&MAX_FILES.to_string()), "{stderr}");
}
