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
