//! Tests of reading a project into the canonical input of `validate` (SPEC-0001 R12; plan,
//! D1; tasks, T6). They write temporary projects and read them back: the I/O opt-out of the
//! crate root covers them, since the runtime performs I/O.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use wayloop_core::artifacts::locations::{MAX_FILE_BYTES, Stored, StoredFile};
use wayloop_core::{Command, Context, run};
use wayloop_schemas::validate::{EntryFormat, ValidateInput};
use wayloop_schemas::value::ProjectPath;

use super::{Content, discover, entry, list, load, segments};

/// The root of the repository.
fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The text of an example of SPEC-0001.
fn example(name: &str) -> String {
    let path = repository()
        .join("specs/0001-artifact-model/examples")
        .join(name);
    fs::read_to_string(path).unwrap()
}

/// The input of a contract case of `validate` v1.
fn case_input(name: &str) -> Value {
    let path = repository().join(format!("tests/contract/validate/v1/{name}.json"));
    let case: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    case["input"].clone()
}

/// A temporary project with `files`, empty first; removed when dropped.
struct Project(PathBuf);

impl Project {
    fn new(name: &str, files: &[(&str, &[u8])]) -> Self {
        let root = std::env::temp_dir().join(format!("wayloop-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        for (path, content) in files {
            let path = root.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, content).unwrap();
        }
        Self(root)
    }

    /// The input of every stored artifact of the project, as `wayloop validate` reads it.
    fn input(&self) -> ValidateInput {
        load(&self.0, discover(&self.0).unwrap()).unwrap()
    }

    /// The envelope of the entry point for the input of the project.
    fn envelope(&self) -> Value {
        let input = serde_json::to_value(self.input()).unwrap();
        let envelope = run(Command::Validate, &input, &Context::default());
        serde_json::from_str(&wayloop_core::envelope_json(&envelope).unwrap()).unwrap()
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn paths(files: &[StoredFile]) -> Vec<(&str, Stored)> {
    files.iter().map(|f| (f.path.as_str(), f.stored)).collect()
}

#[test]
fn discovery_finds_the_locations_of_the_core_sorted_with_relative_slash_paths() {
    let project = Project::new(
        "discover",
        &[
            ("specs/0002-state-machine/spec.md", b"# SPEC-0002"),
            ("specs/0001-artifact-model/spec.md", b"# SPEC-0001"),
            ("specs/0001-artifact-model/plan.md", b"# Plan"),
            ("specs/_templates/spec.md", b"# Template"),
            ("specs/constitution.md", b"# Constitution"),
            ("plan/items/TASK-0107.yaml", b"kind: task\n"),
            ("plan/council/COUNCIL-0001.yaml", b"kind: council_report\n"),
            ("plan/briefs/BRIEF-0002.yaml", b"kind: brief\n"),
            ("plan/briefs/BRIEF-0001.yaml", b"kind: brief\n"),
            ("plan/briefs/README.md", b"# Briefs"),
            ("plan/briefs/deeper/BRIEF-0003.yaml", b"kind: brief\n"),
            ("plan/other/BRIEF-0004.yaml", b"kind: brief\n"),
            ("BRIEF-0005.yaml", b"kind: brief\n"),
        ],
    );
    let found = discover(&project.0).unwrap();
    assert_eq!(
        paths(&found),
        [
            ("plan/briefs/BRIEF-0001.yaml", Stored::Yaml),
            ("plan/briefs/BRIEF-0002.yaml", Stored::Yaml),
            ("plan/council/COUNCIL-0001.yaml", Stored::Yaml),
            ("plan/items/TASK-0107.yaml", Stored::Yaml),
            ("specs/0001-artifact-model/spec.md", Stored::Markdown),
            ("specs/0002-state-machine/spec.md", Stored::Markdown),
        ]
    );
    let input = load(&project.0, found).unwrap();
    assert_eq!(input.documents[4].format, EntryFormat::Markdown);
    assert_eq!(input.documents[4].text.as_deref(), Some("# SPEC-0001"));
    let empty = Project::new("discover-empty", &[]);
    assert_eq!(discover(&empty.0).unwrap(), []);
    assert_eq!(empty.input().documents, []);
}

#[test]
fn a_location_is_listed_down_to_its_depth_only() {
    let project = Project::new(
        "depth",
        &[
            ("a/one.yaml", b"x: 1\n"),
            ("a/b/two.yaml", b"x: 2\n"),
            ("a/b/c/three.yaml", b"x: 3\n"),
        ],
    );
    for (depth, expected) in [
        (1, &["a/one.yaml"][..]),
        (2, &["a/b/two.yaml", "a/one.yaml"][..]),
        (3, &["a/b/c/three.yaml", "a/b/two.yaml", "a/one.yaml"][..]),
    ] {
        let mut found = Vec::new();
        list(&project.0, "a", depth, &mut found).unwrap();
        found.sort();
        assert_eq!(found, expected, "depth {depth}");
    }
}

#[cfg(unix)]
#[test]
fn discovery_follows_no_symbolic_link() {
    let outside = Project::new("outside", &[("BRIEF-0009.yaml", b"kind: brief\n")]);
    let project = Project::new(
        "links",
        &[("plan/briefs/BRIEF-0001.yaml", b"kind: brief\n")],
    );
    let briefs = project.0.join("plan/briefs");
    std::os::unix::fs::symlink(
        outside.0.join("BRIEF-0009.yaml"),
        briefs.join("BRIEF-0009.yaml"),
    )
    .unwrap();
    std::os::unix::fs::symlink(&outside.0, project.0.join("specs")).unwrap();
    let found = discover(&project.0).unwrap();
    assert_eq!(
        paths(&found),
        [("plan/briefs/BRIEF-0001.yaml", Stored::Yaml)]
    );
}

#[cfg(unix)]
#[test]
fn a_location_that_cannot_be_listed_is_an_error_of_the_environment() {
    let project = Project::new("not-a-folder", &[("plan/briefs", b"a file")]);
    let error = discover(&project.0).unwrap_err();
    assert!(error.path().ends_with("plan/briefs"), "{error}");
    assert!(error.to_string().contains("plan/briefs"), "{error}");
    assert!(std::error::Error::source(&error).is_some());
    let missing = load(
        &project.0,
        vec![StoredFile {
            path: ProjectPath::new("plan/items/TASK-0001.yaml"),
            stored: Stored::Yaml,
        }],
    )
    .unwrap_err();
    assert_eq!(missing.error().kind(), std::io::ErrorKind::NotFound);
}

#[test]
fn the_examples_load_into_the_committed_golden_inputs() {
    let valid = example("brief.valid.yaml");
    let invalid = example("brief.invalid.yaml");
    let project = Project::new(
        "examples",
        &[
            ("plan/briefs/BRIEF-0001.yaml", valid.as_bytes()),
            ("plan/briefs/BRIEF-0003.yaml", invalid.as_bytes()),
        ],
    );
    let input = serde_json::to_value(project.input()).unwrap();
    assert_eq!(input, case_input("files-brief-pair"));
    // The documents alone are those of the cases sent directly.
    assert_eq!(
        input["documents"][0]["document"],
        case_input("brief-valid")["documents"][0]["document"]
    );
    assert_eq!(
        input["documents"][1]["document"],
        case_input("brief-invalid")["documents"][1]["document"]
    );
}

#[test]
fn source_carries_the_file_and_the_line_of_the_faulty_value() {
    let text = "# A brief.\nkind: brief\nschema_version: 1\nid: BRIEF-0001\ntitle: Time from the terminal\nstatus: approved\norigin: light_requirements\nowner: \"@example-owner\"\nproblem: Hours are lost.\ngoals:\n  - Record time\n  - 42\nusers:\n  - Freelancers\nscope_in:\n  - Time entries\n";
    let project = Project::new(
        "source",
        &[("plan/briefs/BRIEF-0001.yaml", text.as_bytes())],
    );
    let envelope = project.envelope();
    let error = &envelope["error"];
    assert_eq!(error["pointer"], "/goals/1", "{envelope}");
    assert_eq!(error["reason"], "type", "{envelope}");
    assert_eq!(
        error["source"],
        json!({"file": "plan/briefs/BRIEF-0001.yaml", "line": 12})
    );
}

#[test]
fn no_on_and_a_date_stay_strings_in_the_document() {
    let project = Project::new(
        "strings",
        &[(
            "plan/items/x.yaml",
            b"kind: task\nanswer: no\nswitch: on\ndate: 2026-11-17\n",
        )],
    );
    let input = project.input();
    let document = &input.documents[0].document.as_ref().unwrap().0;
    assert_eq!(document["answer"], "no");
    assert_eq!(document["switch"], "on");
    assert_eq!(document["date"], "2026-11-17");
}

#[test]
fn a_duplicate_key_and_a_malformed_file_are_reported_with_syntax_and_their_line() {
    let project = Project::new(
        "unreadable",
        &[
            (
                "plan/briefs/BRIEF-0001.yaml",
                b"kind: brief\nid: BRIEF-0001\nid: BRIEF-0002\n",
            ),
            (
                "plan/briefs/BRIEF-0002.yaml",
                b"kind: brief\ntitle: [unclosed\n",
            ),
        ],
    );
    let input = project.input();
    for (entry, line) in input.documents.iter().zip([3, 3]) {
        assert_eq!(entry.format, EntryFormat::Unreadable);
        assert_eq!(entry.error.as_ref().unwrap().line.get().get(), line);
        assert!(entry.document.is_none() && entry.lines.is_none());
    }
    let envelope = project.envelope();
    let errors = envelope["error"]["errors"].as_array().unwrap();
    assert_eq!(errors.len(), 2, "{envelope}");
    for (i, (error, file)) in errors
        .iter()
        .zip(["plan/briefs/BRIEF-0001.yaml", "plan/briefs/BRIEF-0002.yaml"])
        .enumerate()
    {
        assert_eq!(error["code"], "artifact_schema");
        assert_eq!(error["reason"], "syntax");
        assert_eq!(error["pointer"], "");
        assert_eq!(error["document"], i);
        assert_eq!(error["source"], json!({"file": file, "line": 3}));
    }
    assert!(errors[0]["message"].as_str().unwrap().contains("`id`"));
}

#[test]
fn an_alias_bomb_is_refused_not_loaded() {
    let text = crate::yaml::tests::alias_bomb();
    let project = Project::new("bomb", &[("plan/briefs/BRIEF-0001.yaml", text.as_bytes())]);
    let entry = &project.input().documents[0];
    assert_eq!(entry.format, EntryFormat::Unreadable);
    assert!(entry.document.is_none());
    let message = &entry.error.as_ref().unwrap().message;
    assert!(message.contains("aliases"), "{message}");
}

#[test]
fn an_oversized_file_is_refused_not_loaded() {
    let size = usize::try_from(MAX_FILE_BYTES).unwrap();
    let at_bound = format!("kind: brief\nnote: {}\n", "x".repeat(size - 19));
    assert_eq!(at_bound.len(), size);
    let beyond = format!("{at_bound} ");
    let project = Project::new(
        "oversized",
        &[
            ("plan/items/a.yaml", at_bound.as_bytes()),
            ("plan/items/b.yaml", beyond.as_bytes()),
            ("specs/0001-x/spec.md", beyond.as_bytes()),
        ],
    );
    let input = project.input();
    assert_eq!(input.documents[0].format, EntryFormat::Document);
    for entry in &input.documents[1..] {
        assert_eq!(entry.format, EntryFormat::Unreadable);
        assert!(entry.text.is_none() && entry.document.is_none());
        let error = entry.error.as_ref().unwrap();
        assert_eq!(error.line.get().get(), 1);
        assert!(
            error.message.contains(&MAX_FILE_BYTES.to_string()),
            "{}",
            error.message
        );
    }
    let envelope = project.envelope();
    let syntax: Vec<&Value> = envelope["error"]["errors"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["reason"] == "syntax")
        .collect();
    assert_eq!(syntax.len(), 2, "{envelope}");
    assert_eq!(syntax[0]["document"], 1);
}

#[test]
fn a_file_that_is_not_utf8_is_unreadable_at_the_line_of_the_first_bad_byte() {
    let file = StoredFile {
        path: ProjectPath::new("plan/items/a.yaml"),
        stored: Stored::Markdown,
    };
    let read = entry(file, Content::Bytes(b"a: 1\nb: 2\nc: \xff\n".to_vec()));
    assert_eq!(read.format, EntryFormat::Unreadable);
    let error = read.error.unwrap();
    assert_eq!(error.line.get().get(), 3);
    assert!(error.message.contains("UTF-8"));
    assert_eq!(
        read.origin.unwrap().file.unwrap().as_str(),
        "plan/items/a.yaml"
    );
}

#[test]
fn the_segments_of_a_named_path_are_those_of_the_platform() {
    assert_eq!(
        segments(Path::new("plan/briefs/a.yaml")),
        ["plan", "briefs", "a.yaml"]
    );
    assert_eq!(segments(Path::new("./a/../b.md")), [".", "a", "..", "b.md"]);
    #[cfg(unix)]
    assert_eq!(segments(Path::new("/etc/a.yaml")), ["", "etc", "a.yaml"]);
}
