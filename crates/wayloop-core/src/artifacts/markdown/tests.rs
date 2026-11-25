//! Tests of the normalisation of a living specification from Markdown (SPEC-0001 T14: R6, R12;
//! AC13, AC36). The specification of this repository normalises with no error in the tests of
//! `wayloop-runtime`, which read files; the contract cases are in `tests/contract/validate/v1/`.

use core::num::NonZeroU64;

use serde_json::{Value, json};
use wayloop_schemas::examples;
use wayloop_schemas::value::{Line, ProjectPath};

use super::{VERIFICATION_WORDS, normalise};
use crate::artifacts::codes;
use crate::artifacts::input::{self, Input};
use crate::artifacts::kinds;
use crate::artifacts::tests::found;
use crate::{Error, Pointer, Reason, Source};

/// The file of the specification of the examples.
const FILE: &str = "specs/0001-time-entries/spec.md";

/// The living specification of the examples of SPEC-0001, written in Markdown: its normalised
/// form is `examples::specification()`.
pub(crate) const EXAMPLE: &str = "# SPEC-0001: Time entries

| | |
| --- | --- |
| Status | Implementing |
| Milestone | M1 · First release |
| Owner | @example-owner |
| Related | COUNCIL-0001 |
| Commit scope | core |

## Problem
Freelancers need to record time without leaving the terminal.

## Scope
**In scope:** Recording and listing time entries
**Out of scope:** Export (a later specification)

## User stories
- STORY-0001
- As a freelance developer, I want to see the total of a day, so that I check my timesheet.

## Requirements (EARS)
- **R1** *Record*: WHEN the user runs the record command, THE TOOL SHALL append one entry to the log.
- **R2**: THE TOOL SHALL store durations in minutes.
- **R3**: IF the duration is not positive, THEN THE TOOL SHALL refuse the entry.
- **R4** (removed): THE TOOL SHALL ask for a project when none is given.

## Acceptance criteria
- **AC1** (R1, R2): Given an empty log, when an entry of 90 minutes is recorded, then the log holds one entry of 90 minutes. Verification: TDD.
- **AC2** (R3): Given any duration of zero or less, when recorded, then it is refused. Verification: property test.
- **AC3** (R1): Given the help of the record command, when read by a new user, then it explains duration and project. Verification: judge. Rubric: Score 1 to 5; 4 or more when duration format, project and one example are all stated.

## Interfaces
- timesheet record <duration> <project>

## Open questions
- [x] **Q1. Is a project required for every entry?**

## Notes
The log format follows COUNCIL-0001 D1.
";

/// `EXAMPLE` with `from` replaced by `to`, which must be in it.
fn example_with(from: &str, to: &str) -> String {
    assert!(EXAMPLE.contains(from), "{from}");
    EXAMPLE.replacen(from, to, 1)
}

/// The line, from 1, of the first line of `text` that starts with `start`.
fn line_of(text: &str, start: &str) -> Line {
    let index = text.lines().position(|l| l.starts_with(start)).unwrap();
    Line::new(NonZeroU64::new(u64::try_from(index).unwrap() + 1).unwrap())
}

fn file() -> ProjectPath {
    ProjectPath::new(FILE)
}

/// The document normalised from `text`, read from [`FILE`], and the errors of normalisation.
fn normalised(text: &str) -> (Value, Vec<Error>) {
    let normalised = normalise(text, Some(&file()));
    (Value::Object(normalised.document), normalised.errors)
}

/// The set of one Markdown entry with `text`, read from [`FILE`].
fn markdown_input(text: &str) -> Input {
    let input = json!({"documents": [
        {"format": "markdown", "text": text, "origin": {"file": FILE}}
    ]});
    assert_eq!(input::rules(&input), []);
    input::read(&input).unwrap()
}

/// The errors of validating `text`, read from [`FILE`], with the types whose schema is built.
fn validated(text: &str) -> Vec<Error> {
    let validation = crate::artifacts::validate(&markdown_input(text), &kinds::built().unwrap());
    assert_eq!(validation.failures, []);
    validation.errors
}

fn at_line(line: Line) -> Source {
    Source::File {
        file: file(),
        line: Some(line),
    }
}

#[test]
fn the_markdown_of_the_example_normalises_into_its_document() {
    let (document, errors) = normalised(EXAMPLE);
    assert_eq!(errors, []);
    assert_eq!(document, examples::specification());
    assert_eq!(validated(EXAMPLE), []);
}

#[test]
fn ac13_markdown_requirement_without_id() {
    let text = example_with("- **R2**: THE TOOL", "- THE TOOL");
    let errors = validated(&text);
    assert_eq!(
        found(&errors),
        [(
            codes::ARTIFACT_SCHEMA,
            "/requirements/1/id".into(),
            Reason::Missing
        )]
    );
    let line = line_of(&text, "- THE TOOL SHALL store");
    assert_eq!(errors[0].location(), Some(&at_line(line)));
    assert_eq!(errors[0].document(), Some(0));
}

#[test]
fn ac36_translated_and_extra_sections_are_prose_and_verification_words_are_mapped() {
    let text = example_with("## Requirements (EARS)", "## Requisiti");
    let text = text.replacen(
        "Verification: property test.",
        "Verification: property test for production, TDD for validation.",
        1,
    );
    let (document, errors) = normalised(&text);
    assert_eq!(errors, []);
    let headings: Vec<&str> = document["extra_sections"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["heading"].as_str().unwrap())
        .collect();
    assert_eq!(headings, ["Requisiti", "Notes"]);
    assert!(
        document["extra_sections"][0]["text"]
            .as_str()
            .unwrap()
            .starts_with("- **R1** *Record*: WHEN")
    );
    assert_eq!(document.get("requirements"), None);
    assert_eq!(
        document["acceptance_criteria"][1]["verification"],
        json!(["property", "tdd"])
    );
    assert_eq!(
        found(&validated(&text)),
        [(
            codes::ARTIFACT_SCHEMA,
            "/requirements".into(),
            Reason::Missing
        )]
    );
}

#[test]
fn continuation_lines_of_a_list_item_are_joined() {
    let text = example_with(
        "- **R2**: THE TOOL SHALL store durations in minutes.",
        "- **R2**: THE TOOL SHALL store\n  durations\n\n  in minutes.",
    );
    let text = text.replacen(
        "- timesheet record <duration> <project>",
        "- timesheet record\n<duration> <project>",
        1,
    );
    let (document, errors) = normalised(&text);
    assert_eq!(errors, []);
    assert_eq!(document, examples::specification());
}

#[test]
fn header_rows_are_split_at_commas_and_semicolons() {
    let text = example_with(
        "| Related | COUNCIL-0001 |",
        "| Related | #3, #4; ADR 0002, 0003; SPEC-0002 |\n| Superseded by | SPEC-0002, SPEC-0003 |\n| Language | it |",
    );
    let (document, errors) = normalised(&text);
    assert_eq!(errors, []);
    assert_eq!(
        document["related"],
        json!(["#3", "#4", "ADR 0002", "0003", "SPEC-0002"])
    );
    assert_eq!(document["superseded_by"], json!(["SPEC-0002", "SPEC-0003"]));
    assert_eq!(document["language"], json!("it"));
    assert_eq!(document["status"], json!("implementing"));
    assert_eq!(document["milestone"], json!("M1"));
}

#[test]
fn verification_text_with_no_mapped_word_is_refused_with_enum() {
    let text = example_with("Verification: TDD.", "Verification: manual review.");
    let errors = validated(&text);
    assert_eq!(
        found(&errors),
        [(
            codes::ARTIFACT_SCHEMA,
            "/acceptance_criteria/0/verification".into(),
            Reason::Enum
        )]
    );
    let words: Vec<Value> = VERIFICATION_WORDS.iter().map(|(w, _)| json!(w)).collect();
    assert_eq!(errors[0].expected(), Some(&words[..]));
    assert_eq!(
        errors[0].location(),
        Some(&at_line(line_of(&text, "- **AC1**")))
    );
}

#[test]
fn every_verification_word_maps_to_its_value_in_the_order_written() {
    let text = example_with(
        "Verification: TDD.",
        "Verification: none, judge, benchmark, contract test, Property, END-TO-END, BDD scenario, tdd.",
    );
    let (document, errors) = normalised(&text);
    assert_eq!(errors, []);
    assert_eq!(
        document["acceptance_criteria"][0]["verification"],
        json!([
            "none",
            "judge",
            "benchmark",
            "contract",
            "property",
            "end_to_end",
            "bdd",
            "tdd"
        ])
    );
}

#[test]
fn headings_inside_a_fenced_block_are_prose() {
    let text = example_with(
        "The log format follows COUNCIL-0001 D1.",
        "```text\n## Problem\n```",
    );
    let (document, errors) = normalised(&text);
    assert_eq!(errors, []);
    assert_eq!(
        document["extra_sections"],
        json!([{"heading": "Notes", "text": "```text\n## Problem\n```"}])
    );
}

#[test]
fn an_unknown_header_row_is_refused_at_its_line() {
    let text = example_with(
        "| Commit scope | core |",
        "| Commit scope | core |\n| Stato | Bozza |",
    );
    let errors = validated(&text);
    assert_eq!(
        found(&errors),
        [(
            codes::ARTIFACT_SCHEMA,
            "/Stato".into(),
            Reason::UnknownField
        )]
    );
    assert_eq!(
        errors[0].location(),
        Some(&at_line(line_of(&text, "| Stato")))
    );
}

#[test]
fn a_repeated_header_row_is_refused() {
    let text = example_with(
        "| Commit scope | core |",
        "| Commit scope | core |\n| Owner | @other |",
    );
    let (document, errors) = normalised(&text);
    assert_eq!(
        found(&errors),
        [(
            codes::ARTIFACT_SCHEMA,
            "/Owner".into(),
            Reason::UnknownField
        )]
    );
    assert_eq!(document["owner"], json!("@example-owner"));
}

#[test]
fn a_repeated_section_of_the_template_is_refused_at_the_line_of_its_second_heading() {
    // Decided by the maintainer on 2026-11-25: refused with `duplicate`, not kept as prose.
    let text = example_with(
        "## Notes",
        "## Requirements (EARS)\n- **R5**: THE TOOL SHALL list entries.\n\n## Notes",
    );
    let errors = validated(&text);
    assert_eq!(
        found(&errors),
        [(
            codes::ARTIFACT_SCHEMA,
            "/requirements".into(),
            Reason::Duplicate
        )]
    );
    let second = text
        .lines()
        .enumerate()
        .filter(|(_, l)| *l == "## Requirements (EARS)")
        .nth(1)
        .map(|(i, _)| Line::new(NonZeroU64::new(u64::try_from(i).unwrap() + 1).unwrap()))
        .unwrap();
    assert_eq!(errors[0].location(), Some(&at_line(second)));
    // The content of the repeated section is not read, and nothing goes to the prose sections.
    let (document, _) = normalised(&text);
    assert_eq!(document["requirements"].as_array().unwrap().len(), 4);
    assert_eq!(
        document["extra_sections"],
        json!([{"heading": "Notes", "text": "The log format follows COUNCIL-0001 D1."}])
    );
}

#[test]
fn text_outside_the_title_the_header_and_the_sections_is_refused_at_its_line() {
    let text = example_with("| | |", "Stray text.\n\n| | |");
    let errors = validated(&text);
    assert_eq!(
        found(&errors),
        [(codes::ARTIFACT_SCHEMA, String::new(), Reason::Syntax)]
    );
    assert_eq!(
        errors[0].location(),
        Some(&at_line(line_of(&text, "Stray")))
    );
}

#[test]
fn values_are_located_at_their_lines() {
    let text = example_with("| Status | Implementing |", "| Status | Published |");
    let errors = validated(&text);
    assert_eq!(
        found(&errors),
        [(codes::ARTIFACT_SCHEMA, "/status".into(), Reason::Enum)]
    );
    assert_eq!(
        errors[0].location(),
        Some(&at_line(line_of(&text, "| Status")))
    );
}

#[test]
fn a_specification_outside_its_folder_has_no_capability() {
    let normalised = normalise(EXAMPLE, Some(&ProjectPath::new("docs/spec.md")));
    assert_eq!(normalised.document.get("capability"), None);
    let sent = normalise(EXAMPLE, None);
    assert_eq!(sent.document.get("capability"), None);
    assert_eq!(sent.errors, []);
    let input = json!({"documents": [{"format": "markdown", "text": EXAMPLE}]});
    let input = input::read(&input).unwrap();
    let validation = crate::artifacts::validate(&input, &kinds::built().unwrap());
    assert_eq!(
        found(&validation.errors),
        [(
            codes::ARTIFACT_SCHEMA,
            "/capability".into(),
            Reason::Missing
        )]
    );
    // Sent directly: no source.
    assert_eq!(validation.errors[0].location(), None);
}

#[test]
fn template_forms_of_items_without_their_parts_reach_the_schema() {
    // An open question without a checkbox, a criterion without its requirements, a title
    // without its identifier: each is reported at its member.
    let text = example_with(
        "- [x] **Q1. Is a project required for every entry?**",
        "- Is a project required for every entry?",
    );
    let text = text.replacen("- **AC2** (R3): Given", "- **AC2**: Given", 1);
    let text = text.replacen("# SPEC-0001: Time entries", "# Time entries", 1);
    let found = found(&validated(&text));
    let pointers: Vec<&str> = found.iter().map(|(_, p, _)| p.as_str()).collect();
    assert_eq!(
        pointers,
        [
            "/acceptance_criteria/1/requirements",
            "/open_questions/0/resolved",
            "/id"
        ]
    );
}

#[test]
fn the_normalised_root_and_its_values_have_lines() {
    let normalised = normalise(EXAMPLE, Some(&file()));
    let line = |pointer: &str| {
        normalised
            .lines
            .get(&Pointer::parse(pointer).unwrap())
            .copied()
    };
    assert_eq!(line(""), Some(line_of(EXAMPLE, "")));
    assert_eq!(line("/title"), Some(line_of(EXAMPLE, "# SPEC")));
    assert_eq!(line("/owner"), Some(line_of(EXAMPLE, "| Owner")));
    assert_eq!(line("/related/0"), Some(line_of(EXAMPLE, "| Related")));
    assert_eq!(line("/problem"), Some(line_of(EXAMPLE, "## Problem")));
    assert_eq!(line("/requirements/3"), Some(line_of(EXAMPLE, "- **R4**")));
    assert_eq!(
        line("/scope_out/0"),
        Some(line_of(EXAMPLE, "**Out of scope:**"))
    );
    assert_eq!(
        line("/extra_sections/0"),
        Some(line_of(EXAMPLE, "## Notes"))
    );
    assert_eq!(line("/capability"), None);
}

#[test]
fn text_of_the_scope_before_its_paragraphs_is_refused_and_an_empty_row_gives_no_member() {
    let text = example_with("**In scope:**", "Before.\n**In scope:**");
    let text = text.replacen("| Owner | @example-owner |", "| Owner | |", 1);
    let (document, errors) = normalised(&text);
    assert_eq!(
        found(&errors),
        [(codes::ARTIFACT_SCHEMA, String::new(), Reason::Syntax)]
    );
    assert_eq!(
        errors[0].location(),
        Some(&at_line(line_of(&text, "Before.")))
    );
    assert_eq!(document.get("owner"), None);
    assert_eq!(
        document["scope_in"],
        json!(["Recording and listing time entries"])
    );
}

#[test]
fn kind_and_schema_version_are_listed_as_defaults_of_their_document() {
    // Decided by the maintainer on 2026-11-25: the Markdown form does not carry them, so the
    // envelope lists both in `meta.defaults_applied` with the index of the document, after the
    // default of the input itself, in the order of the documents, then in the byte order of their pointers.
    let input = json!({"documents": [
        {"format": "document", "document": examples::specification()},
        {"format": "markdown", "text": EXAMPLE, "origin": {"file": FILE}},
        {"format": "markdown", "text": EXAMPLE, "origin": {"file": FILE}}
    ]});
    let envelope = crate::run(crate::Command::Validate, &input, &crate::Context::default());
    let written = serde_json::to_value(&envelope).unwrap();
    let defaults: Vec<(Option<u64>, String, Value, String)> = written["meta"]["defaults_applied"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| {
            (
                d["document"].as_u64(),
                d["pointer"].as_str().unwrap().to_owned(),
                d["value"].clone(),
                d["reason"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    let implied = super::IMPLIED_BY_FORM.id().to_owned();
    assert_eq!(
        defaults,
        [
            (
                None,
                String::new(),
                json!(1),
                crate::CURRENT_COMMAND_VERSION.id().to_owned()
            ),
            (
                Some(1),
                "/kind".into(),
                json!("specification"),
                implied.clone()
            ),
            (Some(1), "/schema_version".into(), json!(1), implied.clone()),
            (
                Some(2),
                "/kind".into(),
                json!("specification"),
                implied.clone()
            ),
            (Some(2), "/schema_version".into(), json!(1), implied),
        ]
    );
}

#[test]
fn a_repeated_scope_marker_is_refused_at_its_member_and_line() {
    // Review B1, decided by the maintainer on 2026-11-25.
    let text = example_with(
        "**Out of scope:** Export (a later specification)",
        "**Out of scope:** Export (a later specification)\n**In scope:** Again\n**Out of scope:** Twice",
    );
    let (document, errors) = normalised(&text);
    assert_eq!(
        found(&errors),
        [
            (
                codes::ARTIFACT_SCHEMA,
                "/scope_in".into(),
                Reason::Duplicate
            ),
            (
                codes::ARTIFACT_SCHEMA,
                "/scope_out".into(),
                Reason::Duplicate
            )
        ]
    );
    assert_eq!(
        errors[0].location(),
        Some(&at_line(line_of(&text, "**In scope:** Again")))
    );
    assert_eq!(
        errors[1].location(),
        Some(&at_line(line_of(&text, "**Out of scope:** Twice")))
    );
    // The repeated paragraphs are not read.
    assert_eq!(
        document["scope_in"],
        json!(["Recording and listing time entries"])
    );
    assert_eq!(
        document["scope_out"],
        json!(["Export (a later specification)"])
    );
}

#[test]
fn a_fence_closes_only_on_its_character_with_a_run_at_least_as_long() {
    // Review B2: a shorter run, or the other character, inside a block is text.
    let block = "````text\n```\n~~~\n## Problem\n````";
    let text = example_with("The log format follows COUNCIL-0001 D1.", block);
    let (document, errors) = normalised(&text);
    assert_eq!(errors, []);
    assert_eq!(
        document["extra_sections"],
        json!([{"heading": "Notes", "text": block}])
    );
    assert_eq!(
        document["problem"],
        json!(examples::specification()["problem"])
    );
}

#[test]
fn a_title_inside_a_fenced_block_is_not_the_title() {
    // Review B3: the fenced lines before the title are text outside the form.
    let text = format!("```\n# SPEC-0009: Fake\n```\n{EXAMPLE}");
    let (document, errors) = normalised(&text);
    assert_eq!(document["id"], json!("SPEC-0001"));
    assert_eq!(document["title"], json!("Time entries"));
    let syntax = (codes::ARTIFACT_SCHEMA, String::new(), Reason::Syntax);
    assert_eq!(found(&errors), [syntax.clone(), syntax.clone(), syntax]);
    assert_eq!(
        errors[1].location(),
        Some(&at_line(line_of(&text, "# SPEC-0009")))
    );
}

#[test]
fn a_header_row_is_skipped_only_when_both_cells_are_rules_or_empty() {
    // Review B4: an empty label with a value is a row the form does not know.
    let text = example_with("| --- | --- |", "| --- | --- |\n| :-: | |\n| | @someone |");
    let errors = validated(&text);
    assert_eq!(
        found(&errors),
        [(codes::ARTIFACT_SCHEMA, "/".into(), Reason::UnknownField)]
    );
    assert_eq!(
        errors[0].location(),
        Some(&at_line(line_of(&text, "| | @someone")))
    );
}

#[test]
fn any_markdown_file_is_read_as_a_living_specification_without_capability_outside_its_folder() {
    // S5, confirmed by the maintainer on 2026-11-25: a Markdown file named explicitly, here
    // notes, is read as a living specification and refused as one.
    let input = json!({"documents": [
        {"format": "markdown", "text": "# Notes\n", "origin": {"file": "docs/notes.md"}}
    ]});
    let validation =
        crate::artifacts::validate(&input::read(&input).unwrap(), &kinds::built().unwrap());
    assert_eq!(validation.failures, []);
    assert_eq!(
        validation.documents[0].kind,
        Some(wayloop_schemas::artifact::ArtifactKind::Specification)
    );
    let pointers: Vec<String> = validation
        .errors
        .iter()
        .map(|e| e.pointer().to_string())
        .collect();
    assert!(pointers.contains(&"/capability".to_owned()), "{pointers:?}");
    assert!(pointers.contains(&"/id".to_owned()), "{pointers:?}");
}

#[test]
fn a_repeated_scope_or_user_stories_section_is_refused_at_its_first_member() {
    // S6: the first of the two members of the section.
    for (heading, pointer) in [("## Scope", "/scope_in"), ("## User stories", "/stories")] {
        let text = example_with("## Notes", &format!("{heading}\n- Again\n\n## Notes"));
        assert_eq!(
            found(&validated(&text)),
            [(codes::ARTIFACT_SCHEMA, pointer.into(), Reason::Duplicate)],
            "{heading}"
        );
    }
}

#[test]
fn the_first_section_is_still_checked_when_its_heading_repeats() {
    // S6: the repeated heading is refused, and the first section keeps its own errors.
    let text = example_with("- **R2**: THE TOOL", "- THE TOOL");
    let text = text.replacen(
        "## Notes",
        "## Requirements (EARS)\n- **R5**: Again.\n\n## Notes",
        1,
    );
    assert_eq!(
        found(&validated(&text)),
        [
            (
                codes::ARTIFACT_SCHEMA,
                "/requirements".into(),
                Reason::Duplicate
            ),
            (
                codes::ARTIFACT_SCHEMA,
                "/requirements/1/id".into(),
                Reason::Missing
            )
        ]
    );
}

#[test]
fn text_after_the_verification_words_that_is_not_a_rubric_is_added_to_the_text() {
    // S6, decided by the maintainer on 2026-11-25: nothing written is dropped.
    let text = example_with(
        "Verification: TDD.",
        "Verification: TDD. See the log format.",
    );
    let (document, errors) = normalised(&text);
    assert_eq!(errors, []);
    assert_eq!(
        document["acceptance_criteria"][0]["text"],
        json!(
            "Given an empty log, when an entry of 90 minutes is recorded, then the log holds \
             one entry of 90 minutes. See the log format."
        )
    );
}

#[test]
fn two_living_specifications_with_one_identifier_are_located_at_their_title_lines() {
    // The identifier stage reads the normalised entry, so its errors carry the line.
    let other = "specs/0002-time-entries/spec.md";
    let input = json!({"documents": [
        {"format": "markdown", "text": EXAMPLE, "origin": {"file": FILE}},
        {"format": "markdown", "text": EXAMPLE, "origin": {"file": other}}
    ]});
    let validation =
        crate::artifacts::validate(&input::read(&input).unwrap(), &kinds::built().unwrap());
    let title = line_of(EXAMPLE, "# SPEC-0001");
    assert_eq!(
        found(&validation.errors),
        [
            (codes::IDENTIFIER_DUPLICATE, "/id".into(), Reason::Duplicate),
            (codes::IDENTIFIER_DUPLICATE, "/id".into(), Reason::Duplicate)
        ]
    );
    assert_eq!(validation.errors[0].location(), Some(&at_line(title)));
    let related = validation.errors[0].related().unwrap();
    assert_eq!(
        serde_json::to_value(related).unwrap(),
        json!([{"document": 1, "pointer": "/id", "file": other, "line": title.get()}])
    );
    // The folder of the second names SPEC-0002: a warning at the line of the title.
    assert_eq!(validation.warnings[0].pointer().to_string(), "/id");
    assert_eq!(
        validation.warnings[0].location(),
        Some(&Source::File {
            file: ProjectPath::new(other),
            line: Some(title)
        })
    );
}

#[test]
fn a_closing_fence_is_followed_only_by_spaces() {
    // Review round 2, S1, as in the CommonMark specification: inside an open block, a run followed by an info string
    // is text; a run followed by spaces closes the block.
    let block = "```\n```rust\n## Problem\n```   ";
    let text = example_with("The log format follows COUNCIL-0001 D1.", block);
    let (document, errors) = normalised(&text);
    assert_eq!(errors, []);
    assert_eq!(
        document["extra_sections"],
        json!([{"heading": "Notes", "text": block}])
    );
    // After the block closes, the next heading is read again.
    let after = example_with(
        "The log format follows COUNCIL-0001 D1.",
        "```\n```  \n## Extra",
    );
    let (document, _) = normalised(&after);
    assert_eq!(document["extra_sections"][1]["heading"], json!("Extra"));
}

#[test]
fn a_header_row_without_a_label_says_so() {
    let text = example_with("| --- | --- |", "| --- | --- |\n| | @someone |");
    let (_, errors) = normalised(&text);
    assert_eq!(errors[0].message(), "A header row has no label.");
}
