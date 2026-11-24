//! Tests of reading a project into the canonical input of `validate` (SPEC-0001 R12; plan,
//! D1; tasks, T6). They write temporary projects and read them back: the I/O opt-out of the
//! crate root covers them, since the runtime performs I/O.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use wayloop_core::artifacts::locations::{MAX_DOCUMENT_BYTES, MAX_FILE_BYTES, Stored, StoredFile};
use wayloop_core::{Command, Context, run};
use wayloop_schemas::validate::{ArtifactFolder, EntryFormat, ValidateInput};
use wayloop_schemas::value::ProjectPath;

use super::{
    Content, LoadError, discover, discovered_input, entry, list, load, load_within, segments,
};
use crate::test_support::TempProject;
use wayloop_core::artifacts::locations::Refused;

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

/// A temporary project, read as `wayloop validate` reads it.
struct Project(TempProject);

impl Project {
    fn new(name: &str, files: &[(&str, &[u8])]) -> Self {
        Self(TempProject::new(name, files))
    }

    fn root(&self) -> &Path {
        self.0.root()
    }

    /// The input of every stored artifact of the project, as `wayloop validate` reads it.
    fn input(&self) -> ValidateInput {
        discovered_input(self.root()).unwrap()
    }

    /// The envelope of the entry point for the input of the project.
    fn envelope(&self) -> Value {
        let input = serde_json::to_value(self.input()).unwrap();
        let envelope = run(Command::Validate, &input, &Context::default());
        serde_json::from_str(&wayloop_core::envelope_json(&envelope).unwrap()).unwrap()
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
    let discovery = discover(project.root()).unwrap();
    assert_eq!(discovery.locations, ArtifactFolder::ALL);
    let found = discovery.files;
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
    let input = load(project.root(), found).unwrap();
    assert_eq!(input.documents[4].format, EntryFormat::Markdown);
    assert_eq!(input.documents[4].text.as_deref(), Some("# SPEC-0001"));
    let empty = Project::new("discover-empty", &[]);
    // SPEC-0001 R26: no folder of a location exists, which the core reports as a warning.
    let nothing = discover(empty.root()).unwrap();
    assert_eq!(nothing.files, []);
    assert_eq!(nothing.locations, []);
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
        list(project.root(), "a", depth, &mut found).unwrap();
        found.sort();
        assert_eq!(found, expected, "depth {depth}");
    }
}

const VALID_BRIEF: &str = "kind: brief\nschema_version: 1\nid: BRIEF-0001\ntitle: Time from the terminal\nstatus: approved\norigin: light_requirements\nowner: \"@example-owner\"\nproblem: Hours are lost.\ngoals:\n  - Record time\nusers:\n  - Freelancers\nscope_in:\n  - Time entries\n";

#[cfg(unix)]
#[test]
fn discovery_follows_no_symbolic_link() {
    use std::os::unix::fs::symlink;
    let outside = Project::new(
        "outside",
        &[
            ("briefs/BRIEF-0001.yaml", VALID_BRIEF.as_bytes()),
            ("specs/0001-x/spec.md", b"# SPEC-0001"),
            ("BRIEF-0009.yaml", VALID_BRIEF.as_bytes()),
        ],
    );
    // A linked location folder, a linked parent of one, and a linked file.
    let project = Project::new("links", &[("plan/items/TASK-0001.yaml", b"kind: task\n")]);
    fs::create_dir_all(project.root().join("plan")).unwrap();
    symlink(
        outside.root().join("briefs"),
        project.root().join("plan/briefs"),
    )
    .unwrap();
    symlink(outside.root().join("specs"), project.root().join("specs")).unwrap();
    symlink(
        outside.root().join("BRIEF-0009.yaml"),
        project.root().join("plan/items/BRIEF-0009.yaml"),
    )
    .unwrap();
    let discovery = discover(project.root()).unwrap();
    assert_eq!(
        paths(&discovery.files),
        [("plan/items/TASK-0001.yaml", Stored::Yaml)]
    );
    // A folder reached through a link is not a location of the project.
    assert_eq!(discovery.locations, [ArtifactFolder::PlanItems]);
    let linked_parent = Project::new("linked-parent", &[]);
    fs::create_dir_all(outside.root().join("plan/briefs")).unwrap();
    fs::write(
        outside.root().join("plan/briefs/BRIEF-0001.yaml"),
        VALID_BRIEF,
    )
    .unwrap();
    symlink(
        outside.root().join("plan"),
        linked_parent.root().join("plan"),
    )
    .unwrap();
    let discovery = discover(linked_parent.root()).unwrap();
    assert_eq!(discovery.files, []);
    assert_eq!(discovery.locations, []);
}

#[cfg(unix)]
#[test]
fn a_named_file_reached_through_a_link_outside_the_project_is_refused() {
    use std::os::unix::fs::symlink;
    let outside = Project::new(
        "creds",
        &[("creds.yaml", b"secret: x\n"), ("dir/a.yaml", b"a: 1\n")],
    );
    let project = Project::new("named-links", &[("plan/briefs/ok.yaml", b"a: 1\n")]);
    symlink(
        outside.root().join("creds.yaml"),
        project.root().join("plan/briefs/x.yaml"),
    )
    .unwrap();
    symlink(
        outside.root().join("dir"),
        project.root().join("plan/linked"),
    )
    .unwrap();
    for named in ["plan/briefs/x.yaml", "plan/linked/a.yaml"] {
        let files = vec![StoredFile {
            path: ProjectPath::new(named),
            stored: Stored::Yaml,
        }];
        match load(project.root(), files) {
            Err(LoadError::Refused {
                path: Some(path),
                refused: Refused::OutsideTheProject,
            }) => assert_eq!(path.as_str(), named),
            other => panic!("{named}: {other:?}"),
        }
    }
    // A link that stays inside the project is read.
    symlink(
        project.root().join("plan/briefs/ok.yaml"),
        project.root().join("plan/briefs/same.yaml"),
    )
    .unwrap();
    let inside = vec![StoredFile {
        path: ProjectPath::new("plan/briefs/same.yaml"),
        stored: Stored::Yaml,
    }];
    let input = load(project.root(), inside).unwrap();
    assert_eq!(input.documents[0].format, EntryFormat::Document);
    // A link inside the project to a file of another extension is refused, as a named path
    // of that extension is.
    fs::write(project.root().join(".env"), b"SECRET=x\n").unwrap();
    symlink(
        project.root().join(".env"),
        project.root().join("plan/briefs/e.yaml"),
    )
    .unwrap();
    let disguised = vec![StoredFile {
        path: ProjectPath::new("plan/briefs/e.yaml"),
        stored: Stored::Yaml,
    }];
    match load(project.root(), disguised) {
        Err(LoadError::Refused {
            path: Some(path),
            refused: Refused::NotAnArtifactFile,
        }) => assert_eq!(path.as_str(), "plan/briefs/e.yaml"),
        other => panic!("{other:?}"),
    }
}

#[cfg(unix)]
#[test]
fn a_special_file_is_an_error_of_the_environment_not_a_blocking_read() {
    let project = Project::new("fifo", &[]);
    fs::create_dir_all(project.root().join("plan/briefs")).unwrap();
    let fifo = project.root().join("plan/briefs/BRIEF-0001.yaml");
    let made = std::process::Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .unwrap();
    assert!(made.success());
    let files = vec![StoredFile {
        path: ProjectPath::new("plan/briefs/BRIEF-0001.yaml"),
        stored: Stored::Yaml,
    }];
    match load(project.root(), files) {
        Err(LoadError::Read(error)) => {
            assert_eq!(error.error().kind(), std::io::ErrorKind::InvalidInput);
            assert!(error.to_string().contains("not a regular file"), "{error}");
        }
        other => panic!("{other:?}"),
    }
}

#[cfg(unix)]
#[test]
fn a_location_below_a_file_is_an_error_of_the_environment() {
    let project = Project::new("plan-is-a-file", &[("plan", b"a file")]);
    let Err(LoadError::Read(error)) = discover(project.root()) else {
        panic!("a read error");
    };
    assert!(error.path().ends_with("plan/briefs"), "{error}");
}

#[cfg(unix)]
#[test]
fn a_location_that_cannot_be_listed_is_an_error_of_the_environment() {
    let project = Project::new("not-a-folder", &[("plan/briefs", b"a file")]);
    let Err(LoadError::Read(error)) = discover(project.root()) else {
        panic!("a read error");
    };
    assert!(error.path().ends_with("plan/briefs"), "{error}");
    assert!(error.to_string().contains("plan/briefs"), "{error}");
    assert!(std::error::Error::source(&error).is_some());
    let missing = load(
        project.root(),
        vec![StoredFile {
            path: ProjectPath::new("plan/items/TASK-0001.yaml"),
            stored: Stored::Yaml,
        }],
    )
    .unwrap_err();
    let LoadError::Read(missing) = missing else {
        panic!("{missing:?}");
    };
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
fn the_examples_of_the_plan_items_load_into_the_documents_of_their_cases() {
    // T7: the cases of gates, milestones, epics, stories and tasks send the examples
    // directly; each document of a case is the example read as `wayloop validate` reads it, so
    // that the examples and the cases cannot drift.
    let valid = [
        "brief",
        "council_report",
        "specification",
        "gate",
        "milestone",
        "epic",
        "story",
        "task",
    ]
    .map(|kind| format!("{kind}.valid.yaml"));
    let invalid = [
        ("gate-invalid", "gate.invalid.yaml"),
        ("task-invalid", "task.invalid.yaml"),
        ("milestone-invalid", "milestone.invalid.yaml"),
    ];
    let names: Vec<&str> = valid
        .iter()
        .map(String::as_str)
        .chain(invalid.iter().map(|(_, name)| *name))
        .collect();
    let mut read = examples_read(&names);
    let checked = read.split_off(valid.len());
    let set = read;
    assert_eq!(case_documents("plan-valid"), set);
    for ((case, _), document) in invalid.into_iter().zip(checked) {
        let mut expected = set.clone();
        expected.push(document);
        assert_eq!(case_documents(case), expected, "{case}");
    }
    // Every other case of a plan item sends the valid set first, then the document it checks.
    for case in [
        "story-italian-prose",
        "story-key-in-italian",
        "story-approved-without-specification",
        "task-judge-without-rubric",
        "epic-implementing-without-issue",
        "gate-done-without-issue",
    ] {
        let given = case_documents(case);
        assert_eq!(given.get(..set.len()), Some(&set[..]), "{case}");
        assert_eq!(given.len(), set.len() + 1, "{case}");
    }
}

#[test]
fn the_examples_of_the_schemas_crate_are_those_of_the_valid_case() {
    // `wayloop_schemas::examples` holds a deliberate copy of the documents of `plan-valid`
    // for the tests of the core, which embed no file outside their crate; this keeps it equal.
    use wayloop_schemas::examples;
    let documents = case_documents("plan-valid");
    let of_kind = |kind: &str| {
        let found = documents.iter().find(|d| d["kind"] == kind);
        found.unwrap_or_else(|| panic!("no {kind} in plan-valid"))
    };
    let copies = [
        ("gate", examples::gate()),
        ("milestone", examples::milestone()),
        ("epic", examples::epic()),
        ("story", examples::story()),
        ("task", examples::task()),
    ];
    for (kind, copy) in copies {
        assert_eq!(&copy, of_kind(kind), "{kind}");
    }
}

/// The examples of SPEC-0001 called `names`, each read as `wayloop validate` reads a file, in
/// the order of `names`.
fn examples_read(names: &[&str]) -> Vec<Value> {
    let texts: Vec<(String, String)> = names
        .iter()
        .map(|name| (format!("plan/items/{name}"), example(name)))
        .collect();
    let files: Vec<(&str, &[u8])> = texts
        .iter()
        .map(|(path, text)| (path.as_str(), text.as_bytes()))
        .collect();
    let input = serde_json::to_value(Project::new("read-examples", &files).input()).unwrap();
    let entries = input["documents"].as_array().unwrap();
    texts
        .iter()
        .map(|(path, _)| {
            let entry = entries
                .iter()
                .find(|e| e["origin"]["file"] == path.as_str());
            entry.unwrap_or_else(|| panic!("{path} was not read"))["document"].clone()
        })
        .collect()
}

/// The entries of the input of the contract case `case`.
fn case_entries(case: &str) -> Vec<Value> {
    case_input(case)["documents"].as_array().unwrap().clone()
}

/// The entry of a contract case that sends the hand-over input `document` directly.
fn handover_input_entry(document: &Value) -> Value {
    json!({
        "format": "document", "input_kind": "handover_input", "document": document
    })
}

/// The documents of the entries of the contract case `case`, in their order.
fn case_documents(case: &str) -> Vec<Value> {
    let entries = case_entries(case);
    entries.into_iter().map(|e| e["document"].clone()).collect()
}

#[test]
fn the_examples_of_the_hand_over_load_into_the_documents_of_their_cases() {
    // T9: the cases of the hand-over send the valid set of `plan-valid`, then the valid record
    // and the valid input, then the document each checks; each is the example read as
    // `wayloop validate` reads it, so that the examples and the cases cannot drift.
    let read = examples_read(&[
        "handover.valid.yaml",
        "handover.input.valid.yaml",
        "handover.invalid.yaml",
        "handover.input.invalid.yaml",
    ]);
    let record = json!({"format": "document", "document": read[0]});
    let mut set = case_entries("plan-valid");
    set.extend([record, handover_input_entry(&read[1])]);
    assert_eq!(case_entries("handover-valid"), set);
    let mut kebab = read[1].clone();
    kebab["step"] = json!("counter-review");
    let checked = [
        (
            "handover-invalid",
            json!({"format": "document", "document": read[2]}),
        ),
        ("handover-input-invalid", handover_input_entry(&read[3])),
        (
            "handover-input-step-kebab-case",
            handover_input_entry(&kebab),
        ),
    ];
    for (case, entry) in checked {
        let mut expected = set.clone();
        expected.push(entry);
        assert_eq!(case_entries(case), expected, "{case}");
    }
    // The deliberate copies of `wayloop_schemas::examples` are these examples too.
    assert_eq!(wayloop_schemas::examples::handover(), read[0]);
    assert_eq!(wayloop_schemas::examples::handover_input(), read[1]);
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
    let size = MAX_FILE_BYTES;
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
    let (read, built) = entry(
        file,
        Content::Bytes(b"a: 1\nb: 2\nc: \xff\n".to_vec()),
        usize::MAX,
    );
    assert_eq!(built, 0);
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

#[test]
fn the_set_is_bounded_by_the_bytes_read_and_built() {
    let brief = VALID_BRIEF.as_bytes();
    let project = Project::new(
        "set-bound",
        &[
            ("plan/briefs/BRIEF-0001.yaml", brief),
            ("plan/briefs/BRIEF-0002.yaml", brief),
        ],
    );
    let files = discover(project.root()).unwrap().files;
    let whole = load(project.root(), files.clone()).unwrap();
    assert!(
        whole
            .documents
            .iter()
            .all(|d| d.format == EntryFormat::Document)
    );
    // Exactly the bytes of the first file and of its document: the first is loaded, the
    // second is not; one byte less and the document of the first does not fit either.
    let built = crate::yaml::parse(VALID_BRIEF, MAX_DOCUMENT_BYTES)
        .unwrap()
        .bytes;
    let budget = brief.len() + built;
    let bounded = load_within(project.root(), files.clone(), budget).unwrap();
    let formats: Vec<EntryFormat> = bounded.documents.iter().map(|d| d.format).collect();
    assert_eq!(formats, [EntryFormat::Document, EntryFormat::Unreadable]);
    let message = &bounded.documents[1].error.as_ref().unwrap().message;
    assert!(
        message.contains(&budget.to_string()) && message.contains("set"),
        "{message}"
    );
    let short = load_within(project.root(), files.clone(), budget - 1).unwrap();
    assert_eq!(short.documents[0].format, EntryFormat::Unreadable);
    // A file of exactly the bytes left is read; it is its document that does not fit.
    let exact = brief.len();
    let read_only = load_within(project.root(), files, exact).unwrap();
    let message = &read_only.documents[0].error.as_ref().unwrap().message;
    assert!(message.contains("the document would take"), "{message}");
}

#[cfg(unix)]
#[test]
fn a_name_that_is_not_utf8_is_skipped() {
    use std::os::unix::ffi::OsStrExt as _;
    let project = Project::new("names", &[("plan/items/TASK-0001.yaml", b"a: 1\n")]);
    let name = std::ffi::OsStr::from_bytes(b"bad-\xff.yaml");
    if fs::write(project.root().join("plan/items").join(name), b"a: 1\n").is_err() {
        return; // The file system refuses such names: nothing to skip.
    }
    let found = discover(project.root()).unwrap().files;
    assert_eq!(paths(&found), [("plan/items/TASK-0001.yaml", Stored::Yaml)]);
}

#[test]
fn a_document_beyond_its_bytes_is_unreadable_and_the_envelope_is_written() {
    let big = "x".repeat(500_000);
    let aliases = vec!["*a"; 80_000].join(",");
    let text = format!("kind: brief\na: &a {big}\nb: [{aliases}]\n");
    let project = Project::new(
        "memory",
        &[("plan/briefs/BRIEF-0001.yaml", text.as_bytes())],
    );
    let entry = &project.input().documents[0];
    assert_eq!(entry.format, EntryFormat::Unreadable);
    let message = &entry.error.as_ref().unwrap().message;
    assert!(
        message.contains(&MAX_DOCUMENT_BYTES.to_string()),
        "{message}"
    );
    let envelope = project.envelope();
    assert_eq!(envelope["error"]["reason"], "syntax", "{envelope}");
}
