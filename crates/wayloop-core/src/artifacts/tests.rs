//! Tests of the artifact model and of `validate` (SPEC-0001 T5: R1, R5, R6, R12, R14, R16;
//! AC1, AC8, AC9, AC25; T7: R1, R5, R6, R9; AC1, AC9, AC35; T8: R1, R5, R6; AC1, AC35; T9: R1,
//! R22; AC1, AC31; T10: R1, R16; AC1; T11: R2, R14; AC1, AC6). The contract cases of the
//! command are in `tests/contract/validate/v1/` and run in the tests of `wayloop-cli`; these
//! tests cover the parts the cases cannot reach.

// The helpers take JSON values by value so that a test reads as the document it builds.
#![allow(clippy::needless_pass_by_value)]

use core::num::{NonZeroU32, NonZeroU64};

use serde_json::{Map, Value, json};
use wayloop_schemas::artifact::ArtifactKind;
use wayloop_schemas::contract::{
    Area, BRIEF, COMMIT, CONTRACTS, Contract, Embedded, HANDOVER, PULL_REQUEST, Part, VALIDATE,
    reread,
};
use wayloop_schemas::validate::{ArtifactFolder, InputKind};
use wayloop_schemas::value::{IssueNumber, Line, MilestoneNumber, ProjectPath};
use wayloop_schemas::{NotDoneReason, Related, Source};

use super::input::{self, Input};
use super::kinds::{self, Checked, Kind, Migration};
use super::locations::{LOCATIONS, Location};
use super::{checks, codes, run_with, validate_with};
use crate::command::COMMAND_INPUT;
use crate::envelope::without_nulls;
use crate::schema::tests::oracle;
use crate::{
    Code, Command, Context, Error, INTERNAL_ERROR, IssueStep, Pointer, Reason, run, run_json,
};

// Helpers -------------------------------------------------------------------------------------

/// The types whose schema is built.
fn built() -> Vec<Kind> {
    kinds::built().unwrap()
}

/// Validates `input` with the types whose schema is built.
pub(super) fn validate(input: &Input) -> super::Validation {
    super::validate(input, &built())
}

/// The input schema of `validate`, loaded.
fn input_schema() -> crate::schema::Schema {
    kinds::load(VALIDATE.embedded(Part::Schema, VALIDATE.version).unwrap()).unwrap()
}

use wayloop_schemas::examples::{
    change, commit, council_report, epic, gate, handover, handover_input, milestone, pull_request,
    specification, story, task,
};

/// A valid brief with only its required members, smaller than the brief of the examples.
pub(super) fn brief() -> Value {
    json!({
        "kind": "brief",
        "schema_version": 1,
        "id": "BRIEF-0001",
        "title": "Freelancers record their working time from the terminal",
        "status": "approved",
        "origin": "light_requirements",
        "owner": "@example-owner",
        "problem": "Freelancers lose billable hours because recording time breaks their flow.",
        "goals": ["Record a time entry with one command"],
        "users": ["Freelance developers"],
        "scope_in": ["Time entries and monthly export"]
    })
}

/// The superseded living specification of the examples, replaced by `SPEC-0001`.
pub(super) fn superseded_specification() -> Value {
    let specification = with(specification(), "id", json!("SPEC-0003"));
    let specification = with(specification, "status", json!("superseded"));
    with(specification, "superseded_by", json!(["SPEC-0001"]))
}

/// The valid documents of the examples, one consistent project in which every link resolves
/// (SPEC-0001, Examples).
pub(super) fn project() -> Vec<Value> {
    vec![
        brief(),
        council_report(),
        specification(),
        gate(),
        milestone(),
        epic(),
        story(),
        task(),
        superseded_specification(),
        change(),
        handover(),
    ]
}

/// The identifier an entry carries: the `id` of its document, or the identifier of the title
/// of a living specification sent as Markdown.
fn carried(entry: &Value) -> Option<String> {
    if let Some(id) = entry["document"]["id"].as_str() {
        return Some(id.to_owned());
    }
    let title = entry["text"].as_str()?.lines().next()?.strip_prefix("# ")?;
    Some(title.split(':').next()?.to_owned())
}

/// `entries`, then the targets of their links: the documents of the [`project`] that carry
/// none of their identifiers. The entries keep their indexes, and every error of the set is
/// theirs, so that a test of one rule sees no error of a link it does not test (R3).
pub(super) fn linked(entries: Vec<Value>) -> Vec<Value> {
    let ids: Vec<String> = entries.iter().filter_map(carried).collect();
    let targets = project()
        .into_iter()
        .filter(|d| !ids.iter().any(|id| d["id"] == id.as_str()))
        .map(entry);
    entries.into_iter().chain(targets).collect()
}

pub(super) fn with(mut document: Value, key: &str, value: Value) -> Value {
    document[key] = value;
    document
}

/// `document` with the title `Title <n>`, so that it shares no title with the examples (R8).
pub(super) fn titled(document: Value, n: u32) -> Value {
    with(document, "title", json!(format!("Title {n}")))
}

pub(super) fn entry(document: Value) -> Value {
    json!({"format": "document", "document": document})
}

/// The input of the set `documents`, read after it passed the input schema and its rules.
pub(super) fn read(documents: Vec<Value>) -> Input {
    read_input(json!({"documents": documents}))
}

/// The canonical `input`, read after it passed the input schema and its rules.
pub(super) fn read_input(input: Value) -> Input {
    assert_eq!(input_schema().validate(&input, COMMAND_INPUT), []);
    assert_eq!(input::rules(&input), []);
    input::read(&input).unwrap()
}

/// The [`project`] with `extra` after it, validated; `extra` starts at the index
/// `project().len()`.
pub(super) fn validated(extra: Vec<Value>) -> super::Validation {
    let entries = project().into_iter().chain(extra).map(entry).collect();
    validate(&read(entries))
}

/// The only error of `validation`, which has no failure and no warning.
pub(super) fn the_only_error(validation: &super::Validation) -> &Error {
    assert_eq!(validation.failures, []);
    assert_eq!(validation.warnings, []);
    let [error] = &validation.errors[..] else {
        panic!("{:?}", validation.errors);
    };
    error
}

pub(super) fn found(errors: &[Error]) -> Vec<(Code, String, Reason)> {
    errors
        .iter()
        .map(|e| (e.code().clone(), e.pointer().to_string(), e.reason()))
        .collect()
}

pub(super) fn at(pointer: &str) -> Pointer {
    Pointer::parse(pointer).unwrap()
}

fn line(n: u64) -> Line {
    Line::new(NonZeroU64::new(n).unwrap())
}

// The registry against the files ------------------------------------------------------------

#[test]
fn the_built_kinds_are_the_documents_of_the_registry_with_every_version() {
    // ADR 0030 G5: the kinds of the core are built from the registry: the artifacts, the inputs
    // of the area `inputs` (T10), then the inputs that are a further file of an artifact (T9).
    let built = built();
    let contracts = |area: Area| CONTRACTS.iter().filter(move |c| c.area == area);
    let (inputs, artifacts): (Vec<Kind>, Vec<Kind>) =
        built.into_iter().partition(|k| k.input.is_some());
    let artifact_names: Vec<&str> = contracts(Area::Artifacts).map(|c| c.name).collect();
    assert_eq!(
        artifacts.iter().map(|k| k.id).collect::<Vec<_>>(),
        artifact_names
    );
    assert!(artifacts.iter().all(|k| k.code == codes::ARTIFACT_SCHEMA));
    let named: Vec<(Option<InputKind>, &str, Code)> = inputs
        .iter()
        .map(|k| (k.input, k.id, k.code.clone()))
        .collect();
    assert_eq!(
        named,
        [
            (Some(InputKind::Commit), "commit", codes::ARTIFACT_SCHEMA),
            (
                Some(InputKind::PullRequest),
                "pull_request",
                codes::ARTIFACT_SCHEMA
            ),
            (
                Some(InputKind::HandoverInput),
                "handover_input",
                codes::HANDOVER_INPUT
            ),
        ]
    );
    // Each kind holds the file of its part for every version of its contract.
    let files = artifacts
        .iter()
        .zip(contracts(Area::Artifacts))
        .chain(inputs.iter().zip(contracts(Area::Inputs)))
        .map(|(kind, contract)| (kind, contract, Part::Schema))
        .chain(inputs.last().map(|kind| (kind, &HANDOVER, Part::Input)));
    for (kind, contract, part) in files {
        let paths: Vec<&str> = kind.versions.iter().map(|e| e.path).collect();
        let expected: Vec<String> = (1..=contract.version.get())
            .map(|v| contract.path(part, NonZeroU32::new(v).unwrap()))
            .collect();
        assert_eq!(paths, expected, "{}", kind.id);
        assert_eq!(kind.migrations.len() + 1, kind.versions.len());
    }
}

#[test]
fn every_input_kind_is_built_exactly_once() {
    // T10: every input has its schema; none is left out as not built.
    let built = built();
    for input in InputKind::ALL {
        let count = built.iter().filter(|k| k.input == Some(input)).count();
        assert_eq!(count, 1, "{input}");
    }
}

#[test]
fn every_built_schema_loads_and_the_command_schemas_too() {
    let built = built();
    let loaded = kinds::load_all(&built).unwrap();
    assert_eq!(loaded.len(), built.len());
    for part in [Part::Schema, Part::Output] {
        kinds::load(VALIDATE.embedded(part, VALIDATE.version).unwrap()).unwrap();
    }
}

#[test]
fn every_kind_and_input_has_a_schema_check_named_after_it() {
    let names = ArtifactKind::ALL
        .iter()
        .map(|k| k.id())
        .chain(InputKind::ALL.iter().map(|k| k.id()));
    for kind in names {
        assert_eq!(kinds::schema_check(kind).id(), format!("schema_{kind}"));
    }
    assert_eq!(kinds::kinds().len(), ArtifactKind::ALL.len());
}

#[test]
fn the_kinds_and_their_checks_are_snake_case_and_unique() {
    let mut ids: Vec<String> = ArtifactKind::ALL
        .iter()
        .map(|k| k.id().to_owned())
        .chain(InputKind::ALL.iter().map(|k| k.id().to_owned()))
        .collect();
    ids.extend(checks::NOT_BUILT.iter().map(|c| c.id().to_owned()));
    ids.extend(
        [
            checks::DOCUMENT_KIND,
            checks::SCHEMA_VERSION,
            checks::IDENTIFIER_FORMAT,
            checks::IDENTIFIER_DUPLICATE,
            checks::IDENTIFIER_LOCATION,
            checks::TITLE_FORMAT,
            checks::TITLE_DUPLICATE,
            checks::BRANCH_NAME,
            checks::COMMIT_HEADER,
            checks::MARKDOWN,
            checks::LINKS,
            checks::PARTS,
            checks::ADR_CITATIONS,
            checks::ARTIFACT_LOCATIONS,
        ]
        .iter()
        .map(|c| c.id().to_owned()),
    );
    let count = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), count);
    for id in ids {
        assert!(
            id.bytes()
                .all(|b| b.is_ascii_lowercase() || b == b'_' || b.is_ascii_digit()),
            "{id}"
        );
    }
}

// The brief schema against the oracle -----------------------------------------------------

/// Asserts that the oracle and the interpreter agree on the verdict of every document against
/// the schema `v1` of `contract`.
fn oracle_agrees(contract: &Contract, documents: impl IntoIterator<Item = Value>) {
    oracle_agrees_on(contract, Part::Schema, documents);
}

/// [`oracle_agrees`] against the file `part` of the contract.
fn oracle_agrees_on(contract: &Contract, part: Part, documents: impl IntoIterator<Item = Value>) {
    let snapshot = contract.embedded(part, NonZeroU32::MIN).unwrap();
    let schema: Value = serde_json::from_str(snapshot.text).unwrap();
    let loaded = kinds::load(snapshot).unwrap();
    let oracle = oracle(&schema);
    for document in documents {
        let errors = loaded.validate(&document, codes::ARTIFACT_SCHEMA);
        assert_eq!(
            oracle.is_valid(&without_nulls(&document)),
            errors.is_empty(),
            "{document}: {errors:?}"
        );
    }
}

#[test]
fn the_oracle_agrees_with_the_interpreter_on_briefs() {
    let documents = [
        brief(),
        with(brief(), "language", json!("it")),
        with(brief(), "language", Value::Null),
        with(brief(), "status", json!("implementing")),
        with(brief(), "status", json!("superseded")),
        with(
            with(brief(), "status", json!("superseded")),
            "superseded_by",
            json!(["BRIEF-0002"]),
        ),
        with(brief(), "superseded_by", json!([])),
        with(brief(), "problema", json!("x")),
        with(brief(), "id", json!("BRIEF-1")),
        with(brief(), "id", json!(1)),
        with(brief(), "owner", json!("example-owner")),
        with(brief(), "goals", json!([])),
        with(brief(), "goals", json!([""])),
        with(brief(), "title", json!("")),
        with(brief(), "title", json!("x".repeat(121))),
        with(brief(), "schema_version", json!(1.0)),
        with(brief(), "schema_version", json!(2)),
        with(brief(), "origin", json!("idea")),
        with(brief(), "scope_out", json!([])),
        with(brief(), "kind", json!("task")),
    ];
    oracle_agrees(&BRIEF, documents);
}

// Gates, milestones, epics, stories and tasks (T7: R1, R5, R6, R9; AC1, AC9, AC35) ----------

/// `document` without the member `key`.
fn without(mut document: Value, key: &str) -> Value {
    document.as_object_mut().unwrap().remove(key);
    document
}

/// The errors of `document`, validated with the targets of its links, by code, pointer and
/// reason; the errors of the targets, which a changed `document` may cause, are not its own.
fn errors_of(document: Value) -> Vec<(Code, String, Reason)> {
    let validation = validate(&read(linked(vec![entry(document)])));
    assert_eq!(validation.failures, []);
    let own: Vec<Error> = validation
        .errors
        .into_iter()
        .filter(|e| e.document() == Some(0))
        .collect();
    found(&own)
}

fn schema_error(pointer: &str, reason: Reason) -> (Code, String, Reason) {
    (codes::ARTIFACT_SCHEMA, pointer.to_owned(), reason)
}

/// Asserts that `documents` pass as one set, that the schema of each kind ran, and that each is
/// read into its contract type and written back as given (ADR 0030 G1).
fn assert_valid_and_reread(documents: &[Value]) {
    let validation = validate(&read(linked(
        documents.iter().cloned().map(entry).collect(),
    )));
    assert_eq!(validation.errors, []);
    assert_eq!(validation.failures, []);
    let checks: Vec<&str> = validation
        .checks
        .iter()
        .map(wayloop_schemas::Check::id)
        .collect();
    let built = built();
    for document in documents {
        let kind = document["kind"].as_str().unwrap();
        assert!(
            checks.contains(&format!("schema_{kind}").as_str()),
            "{kind}"
        );
        let reread = built.iter().find(|k| k.id == kind).unwrap().reread;
        assert_eq!(&reread(document).unwrap(), document, "{kind}");
    }
}

#[test]
fn the_valid_plan_items_of_the_examples_pass_and_read_into_their_types() {
    assert_valid_and_reread(&[gate(), milestone(), epic(), story(), task()]);
}

#[test]
fn a_gate_in_status_done_requires_the_person_who_approved_it() {
    // SPEC-0001, Gate issue: `approved_by` when `status` is `done` (reason `requires`).
    let validation = validate(&read(linked(vec![entry(without(gate(), "approved_by"))])));
    let [error] = &validation.errors[..] else {
        panic!("{:?}", validation.errors);
    };
    assert_eq!(
        (error.pointer().as_str(), error.reason()),
        ("/approved_by", Reason::Requires)
    );
    assert_eq!(error.pointers(), Some(&[at("/status")][..]));
    // The issue is required when `status` is `done` (decided by the maintainer on 2026-11-23),
    // and optional before the gate is done or once it is superseded.
    let validation = validate(&read(linked(vec![entry(without(gate(), "issue"))])));
    let [error] = &validation.errors[..] else {
        panic!("{:?}", validation.errors);
    };
    assert_eq!(
        (error.pointer().as_str(), error.reason()),
        ("/issue", Reason::Requires)
    );
    assert_eq!(error.pointers(), Some(&[at("/status")][..]));
    let draft = without(
        without(with(gate(), "status", json!("draft")), "approved_by"),
        "issue",
    );
    assert_eq!(errors_of(draft.clone()), []);
    let superseded = with(
        with(draft, "status", json!("superseded")),
        "superseded_by",
        json!(["GATE-0002"]),
    );
    assert_eq!(errors_of(superseded), []);
    // A gate has no status approved: it is done when the subject is approved.
    let approved = with(gate(), "status", json!("approved"));
    let validation = validate(&read(linked(vec![entry(approved)])));
    assert_eq!(
        found(&validation.errors),
        [schema_error("/status", Reason::Enum)]
    );
    assert_eq!(
        validation.errors[0].expected(),
        Some(&[json!("draft"), json!("done"), json!("superseded")][..])
    );
}

#[test]
fn the_subject_of_a_gate_is_a_brief_or_a_council_report() {
    let council = with(gate(), "subject", json!("COUNCIL-0001"));
    assert_eq!(errors_of(council), []);
    for (subject, fix) in [
        ("TASK-0001", None),
        ("council 1", Some("COUNCIL-0001")),
        ("BRIEF-1", Some("BRIEF-0001")),
    ] {
        let validation = validate(&read(linked(vec![entry(with(
            gate(),
            "subject",
            json!(subject),
        ))])));
        assert_eq!(
            found(&validation.errors),
            [schema_error("/subject", Reason::Syntax)],
            "{subject}"
        );
        assert_eq!(
            validation.errors[0].fix(),
            fix.map(|f| json!(f)).as_ref(),
            "{subject}"
        );
    }
}

#[test]
fn the_tracker_number_is_required_once_a_plan_item_is_implementing_or_done() {
    // Milestone: `tracker_number`; epic, story and task: `issue`; when `status` is
    // `implementing` or `done`, never before.
    for (document, member) in [
        (milestone(), "tracker_number"),
        (epic(), "issue"),
        (story(), "issue"),
        (task(), "issue"),
    ] {
        for status in ["implementing", "done"] {
            let item = without(with(document.clone(), "status", json!(status)), member);
            assert_eq!(
                errors_of(item),
                [schema_error(&format!("/{member}"), Reason::Requires)],
                "{member} {status}"
            );
        }
        for status in ["draft", "approved"] {
            let item = without(with(document.clone(), "status", json!(status)), member);
            assert_eq!(errors_of(item), [], "{member} {status}");
        }
    }
}

#[test]
fn a_story_requires_its_specification_from_approved_on() {
    let draft = without(
        without(with(story(), "status", json!("draft")), "specification"),
        "issue",
    );
    assert_eq!(errors_of(draft.clone()), []);
    for status in ["approved", "implementing", "done"] {
        let item = with(draft.clone(), "status", json!(status));
        let item = if status == "approved" {
            item
        } else {
            with(item, "issue", json!(11))
        };
        assert_eq!(
            errors_of(item),
            [schema_error("/specification", Reason::Requires)],
            "{status}"
        );
    }
    // A specification given is at least one identifier, in any status.
    assert_eq!(
        errors_of(with(draft, "specification", json!([]))),
        [schema_error("/specification", Reason::Length)]
    );
}

#[test]
fn a_task_verified_by_a_judge_requires_a_rubric() {
    // AC35, for the task: `rubric` when `verification` contains `judge`.
    let judged = with(task(), "verification", json!(["tdd", "judge"]));
    let validation = validate(&read(linked(vec![entry(judged.clone())])));
    let [error] = &validation.errors[..] else {
        panic!("{:?}", validation.errors);
    };
    assert_eq!(
        (error.pointer().as_str(), error.reason()),
        ("/rubric", Reason::Requires)
    );
    assert_eq!(error.pointers(), Some(&[at("/verification")][..]));
    let rubric = with(judged, "rubric", json!("Score 1 to 5."));
    assert_eq!(errors_of(rubric), []);
    assert_eq!(
        errors_of(with(task(), "verification", json!([]))),
        [schema_error("/verification", Reason::Length)]
    );
    assert_eq!(
        errors_of(with(task(), "verification", json!(["unit"]))),
        [schema_error("/verification/0", Reason::Enum)]
    );
    // A text is not a list: its error of type, and no rubric required (review of T7).
    assert_eq!(
        errors_of(with(task(), "verification", json!("tdd"))),
        [schema_error("/verification", Reason::Type)]
    );
    assert_eq!(
        errors_of(with(task(), "verification", json!("judge"))),
        [schema_error("/verification", Reason::Type)]
    );
}

#[test]
fn a_task_without_its_epic_or_its_milestone_gives_one_error_each() {
    // Q2 of the plan: one error, `missing`, and no error of a link.
    assert_eq!(
        errors_of(without(task(), "epic")),
        [schema_error("/epic", Reason::Missing)]
    );
    assert_eq!(
        errors_of(without(without(task(), "epic"), "milestone")),
        [
            schema_error("/epic", Reason::Missing),
            schema_error("/milestone", Reason::Missing)
        ]
    );
    // A story needs its epic and its milestone too, and an epic its milestone.
    assert_eq!(
        errors_of(without(story(), "epic")),
        [schema_error("/epic", Reason::Missing)]
    );
    assert_eq!(
        errors_of(without(epic(), "milestone")),
        [schema_error("/milestone", Reason::Missing)]
    );
}

#[test]
fn blocked_is_never_a_status_of_a_plan_item() {
    // SPEC-0001, Common rules: `blocked` is an issue-cycle state of SPEC-0002 only.
    for document in [milestone(), epic(), story(), task()] {
        let validation = validate(&read(linked(vec![entry(with(
            document,
            "status",
            json!("blocked"),
        ))])));
        assert_eq!(
            found(&validation.errors),
            [schema_error("/status", Reason::Enum)]
        );
        assert_eq!(
            validation.errors[0].expected(),
            Some(
                &[
                    json!("draft"),
                    json!("approved"),
                    json!("implementing"),
                    json!("done"),
                    json!("superseded")
                ][..]
            )
        );
    }
}

#[test]
fn the_complexity_of_a_task_is_from_1_to_10() {
    for complexity in [1, 10] {
        assert_eq!(errors_of(with(task(), "complexity", json!(complexity))), []);
    }
    for complexity in [0, 11] {
        assert_eq!(
            errors_of(with(task(), "complexity", json!(complexity))),
            [schema_error("/complexity", Reason::Range)],
            "{complexity}"
        );
    }
    // Read by value: 3.0 is the integer 3, and reads into the type of the task.
    assert_eq!(errors_of(with(task(), "complexity", json!(3.0))), []);
}

#[test]
fn the_parts_and_links_of_a_plan_item_have_the_forms_of_their_types() {
    let cases = [
        (with(milestone(), "due", json!("31/03/2027")), "/due"),
        (
            with(milestone(), "council_report", json!("COUNCIL-1")),
            "/council_report",
        ),
        (
            with(epic(), "milestone", json!("Milestone 1")),
            "/milestone",
        ),
        (
            with(
                story(),
                "acceptance_criteria",
                json!([{"id": "1", "given": "a", "when": "b", "then": "c"}]),
            ),
            "/acceptance_criteria/0/id",
        ),
        (with(task(), "story", json!("STORY-1")), "/story"),
        (
            with(task(), "depends_on", json!(["TASK-1"])),
            "/depends_on/0",
        ),
        (
            with(task(), "expanded_from", json!("EPIC-0001")),
            "/expanded_from",
        ),
        (
            with(
                task(),
                "specification",
                json!([{"spec": "SPEC-0001", "requirements": ["AC1"]}]),
            ),
            "/specification/0/requirements/0",
        ),
    ];
    for (document, pointer) in cases {
        assert_eq!(
            errors_of(document),
            [schema_error(pointer, Reason::Syntax)],
            "{pointer}"
        );
    }
    // An area is a name of the policy (#267): an empty one, or one out of its form, is
    // refused by the format, with a fix when lower case gives the form.
    assert_eq!(
        errors_of(with(epic(), "area", json!(""))),
        [schema_error("/area", Reason::Syntax)]
    );
    assert_eq!(
        errors_of(with(epic(), "area", json!("area:core"))),
        [schema_error("/area", Reason::Syntax)]
    );
}

#[test]
fn the_oracle_agrees_with_the_interpreter_on_plan_items() {
    let variations = |document: Value| {
        vec![
            document.clone(),
            with(document.clone(), "status", json!("superseded")),
            with(document.clone(), "status", json!("draft")),
            with(document.clone(), "titolo", json!("x")),
            with(document.clone(), "language", json!("it")),
            with(document.clone(), "issue", json!(0)),
            without(document.clone(), "title"),
            with(document, "superseded_by", json!([])),
        ]
    };
    let documents = [
        (gate(), without(gate(), "approved_by")),
        (gate(), without(gate(), "issue")),
        (milestone(), without(milestone(), "tracker_number")),
        (epic(), without(epic(), "issue")),
        (story(), without(story(), "specification")),
        (task(), with(task(), "verification", json!(["judge"]))),
        (task(), with(task(), "verification", json!("judge"))),
    ];
    for (document, refused) in documents {
        let kind = document["kind"].as_str().unwrap().to_owned();
        let contract = CONTRACTS.iter().find(|c| c.name == kind).unwrap();
        let mut all = variations(document);
        all.push(refused);
        oracle_agrees(contract, all);
    }
}

// Hand-over records and inputs (T9: R1, R22; AC1, AC31) --------------------------------------

/// The entry of the input `document` of the kind `input_kind`.
pub(super) fn input_entry(document: Value, input_kind: InputKind) -> Value {
    json!({"format": "document", "input_kind": input_kind.id(), "document": document})
}

/// The errors of validating the input `document` of the kind `input_kind` alone.
fn input_errors(document: Value, input_kind: InputKind) -> Vec<(Code, String, Reason)> {
    let validation = validate(&read(linked(vec![input_entry(document, input_kind)])));
    assert_eq!(validation.failures, []);
    found(&validation.errors)
}

/// Asserts that `validation` has no error and no failure, and that each of `ran` ran.
pub(super) fn passed_with(validation: &super::Validation, ran: &[&str]) {
    assert_eq!(validation.errors, []);
    assert_eq!(validation.failures, []);
    let checks: Vec<&str> = validation
        .checks
        .iter()
        .map(wayloop_schemas::Check::id)
        .collect();
    for check in ran {
        assert!(checks.contains(check), "{check}");
        assert!(
            validation.not_done.iter().all(|n| n.check().id() != *check),
            "{check}"
        );
    }
}

/// Asserts that the input `document` of the kind `input_kind`, whose version the engine does
/// not know, is refused with the known versions, `1` only (R16).
fn version_unknown(document: Value, input_kind: InputKind) {
    let validation = validate(&read(linked(vec![input_entry(document, input_kind)])));
    let error = validation.errors.first().unwrap();
    assert_eq!(
        (error.code(), error.pointer().as_str(), error.reason()),
        (
            &codes::SCHEMA_VERSION_UNKNOWN,
            "/schema_version",
            Reason::Enum
        )
    );
    assert_eq!(error.expected(), Some(&[json!(1)][..]));
}

/// The errors of validating the hand-over input `document` alone.
fn input_errors_of(document: Value) -> Vec<(Code, String, Reason)> {
    input_errors(document, InputKind::HandoverInput)
}

/// The accepted values of the member `step` in the file `part` of the hand-over contract, its
/// reference followed.
fn step_values(part: Part) -> Vec<String> {
    let snapshot = HANDOVER.embedded(part, HANDOVER.version).unwrap();
    let schema: Value = serde_json::from_str(snapshot.text).unwrap();
    let mut step = &schema["properties"]["step"];
    if let Some(reference) = step["$ref"].as_str() {
        step = schema.pointer(reference.trim_start_matches('#')).unwrap();
    }
    step["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn the_step_of_both_hand_over_schemas_is_the_issue_step_of_the_core() {
    // SPEC-0001, Hand-over record: the `step` values are the identifiers of `IssueStep` (#228).
    let steps: Vec<&str> = IssueStep::ALL.iter().map(|s| s.id()).collect();
    for part in [Part::Schema, Part::Input] {
        assert_eq!(step_values(part), steps, "{part}");
    }
}

#[test]
fn the_valid_hand_over_record_and_input_pass_and_read_into_their_types() {
    let validation = validate(&read(vec![
        entry(handover()),
        input_entry(handover_input(), InputKind::HandoverInput),
    ]));
    passed_with(&validation, &["schema_handover", "schema_handover_input"]);
    let built = built();
    let reread_of = |id: &str| built.iter().find(|k| k.id == id).unwrap().reread;
    assert_eq!(reread_of("handover")(&handover()).unwrap(), handover());
    assert_eq!(
        reread_of("handover_input")(&handover_input()).unwrap(),
        handover_input()
    );
    // The record is summarised by its kind and identifier, the input by its input kind only.
    assert_eq!(
        serde_json::to_value(&validation.documents).unwrap(),
        json!([
            {"kind": "handover", "id": "HANDOVER-12-1"},
            {"input_kind": "handover_input"}
        ])
    );
}

#[test]
fn the_summary_of_an_input_never_holds_a_kind_an_identifier_or_a_status() {
    let mut document = with(handover_input(), "id", json!("HANDOVER-12-1"));
    document = with(document, "status", json!("done"));
    let validation = validate(&read(vec![input_entry(document, InputKind::HandoverInput)]));
    assert_eq!(
        serde_json::to_value(&validation.documents).unwrap(),
        json!([{"input_kind": "handover_input"}])
    );
}

#[test]
fn ac31_an_engine_field_in_a_hand_over_input_is_refused_with_unknown_field() {
    // SPEC-0001 R22: every member the engine sets, given by the agent, is refused at its place
    // with the code of the hand-over input, before any engine field is filled.
    let record = handover();
    for field in [
        "kind",
        "id",
        "issue",
        "task",
        "session",
        "holder",
        "executor",
        "branch",
        "recorded_at",
        "tokens",
    ] {
        let document = with(handover_input(), field, record[field].clone());
        assert_eq!(
            input_errors_of(document),
            [(
                codes::HANDOVER_INPUT,
                format!("/{field}"),
                Reason::UnknownField
            )],
            "{field}"
        );
    }
}

#[test]
fn every_error_of_a_hand_over_input_has_its_code_and_the_record_that_of_artifacts() {
    // SPEC-0001, Errors: `handover_input` with `unknown_field` and the reasons of
    // `artifact_schema`; the record is an artifact and keeps `artifact_schema`.
    let document = with(
        without(handover_input(), "next_step"),
        "step",
        json!("counter-review"),
    );
    assert_eq!(
        input_errors_of(document),
        [
            (codes::HANDOVER_INPUT, "/step".to_owned(), Reason::Enum),
            (
                codes::HANDOVER_INPUT,
                "/next_step".to_owned(),
                Reason::Missing
            ),
        ]
    );
    assert_eq!(
        input_errors_of(with(
            handover_input(),
            "rulings",
            json!([{"decision": "x", "reason": "y"}])
        )),
        [(
            codes::HANDOVER_INPUT,
            "/rulings/0/cost_if_wrong".to_owned(),
            Reason::Missing
        )]
    );
    assert_eq!(
        errors_of(without(handover(), "next_step")),
        [schema_error("/next_step", Reason::Missing)]
    );
    assert_eq!(
        errors_of(with(handover(), "id", json!("HANDOVER-012-1"))),
        [(codes::IDENTIFIER_FORMAT, "/id".to_owned(), Reason::Syntax)]
    );
    // R16: an input of a version the engine does not know is refused with its known versions.
    version_unknown(
        with(handover_input(), "schema_version", json!(2)),
        InputKind::HandoverInput,
    );
}

#[test]
fn the_files_touched_of_a_hand_over_are_project_paths_and_its_prose_may_name_its_language() {
    // SPEC-0001, Hand-over record (decided by the maintainer on 2026-11-24): `files_touched`
    // holds project paths, so a path that leaves the project or spells `\` is refused at its
    // place, in the input and in the record; `language` is accepted in both.
    for path in ["../x", "src\\x"] {
        let paths = json!(["src/record.rs", path]);
        assert_eq!(
            input_errors_of(with(handover_input(), "files_touched", paths.clone())),
            [(
                codes::HANDOVER_INPUT,
                "/files_touched/1".to_owned(),
                Reason::Syntax
            )],
            "{path}"
        );
        assert_eq!(
            errors_of(with(handover(), "files_touched", paths)),
            [schema_error("/files_touched/1", Reason::Syntax)],
            "{path}"
        );
    }
    assert_eq!(
        input_errors_of(with(handover_input(), "language", json!("it"))),
        []
    );
    assert_eq!(errors_of(with(handover(), "language", json!("it"))), []);
}

#[test]
fn an_artifact_document_never_selects_the_schema_of_an_input() {
    // `kind` selects among artifacts only: the spelling of an input kind is not one.
    let document = with(handover_input(), "kind", json!("handover_input"));
    let validation = validate(&read(linked(vec![entry(document)])));
    let error = validation.errors.first().unwrap();
    assert_eq!(
        (error.code(), error.pointer().as_str(), error.reason()),
        (&codes::ARTIFACT_SCHEMA, "/kind", Reason::Enum)
    );
}

#[test]
fn the_oracle_agrees_with_the_interpreter_on_hand_overs() {
    let variations = |document: Value| {
        vec![
            document.clone(),
            with(document.clone(), "step", json!("pull-request")),
            with(document.clone(), "files_touched", json!([""])),
            with(document.clone(), "files_touched", json!(["../x"])),
            with(document.clone(), "files_touched", json!(["src\\x"])),
            with(document.clone(), "language", json!("it")),
            with(document.clone(), "language", json!("italiano")),
            with(document.clone(), "rulings", json!([{"decision": "x"}])),
            with(document.clone(), "tokens", json!(10)),
            with(document.clone(), "schema_version", json!(2)),
            without(document, "next_step"),
        ]
    };
    let mut records = variations(handover());
    records.extend([
        with(
            handover(),
            "recorded_at",
            json!("2027-01-15T17:42:00+01:00"),
        ),
        with(handover(), "tokens", json!(0)),
        with(handover(), "id", json!("HANDOVER-12")),
        without(handover(), "task"),
        with(handover(), "status", json!("done")),
    ]);
    oracle_agrees(&HANDOVER, records);
    oracle_agrees_on(&HANDOVER, Part::Input, variations(handover_input()));
}

// Council reports, living specifications and change deltas (T8: R1, R5, R6; AC1, AC35) -------

#[test]
fn the_valid_council_report_specification_and_change_pass_and_read_into_their_types() {
    // Written back as given: a removed requirement keeps its status, and an active one, whose
    // status is absent, is not given one.
    assert_valid_and_reread(&[council_report(), specification(), change()]);
}

/// The error of `document` alone, which must be the only one, with its `expected`.
fn only_error(document: Value) -> Error {
    let validation = validate(&read(linked(vec![entry(document)])));
    let [error] = &validation.errors[..] else {
        panic!("{:?}", validation.errors);
    };
    error.clone()
}

#[test]
fn each_kind_accepts_only_its_own_statuses() {
    // R5: a living specification has six statuses; `published` is none of them
    // (`specification.invalid.yaml`). A council report has those of a brief, a change delta
    // those of a plan item; `blocked` is never a status.
    let six = [
        "draft",
        "approved",
        "surveyed",
        "implementing",
        "done",
        "superseded",
    ];
    let five = ["draft", "approved", "implementing", "done", "superseded"];
    let three = ["draft", "approved", "superseded"];
    let cases: [(Value, &str, &[&str]); 5] = [
        (specification(), "published", &six),
        (specification(), "blocked", &six),
        (council_report(), "implementing", &three),
        (change(), "surveyed", &five),
        (change(), "blocked", &five),
    ];
    for (document, status, expected) in cases {
        let error = only_error(with(document, "status", json!(status)));
        assert_eq!(
            (error.code(), error.pointer().as_str(), error.reason()),
            (&codes::ARTIFACT_SCHEMA, "/status", Reason::Enum),
            "{status}"
        );
        let expected: Vec<Value> = expected.iter().map(|s| json!(s)).collect();
        assert_eq!(error.expected(), Some(&expected[..]), "{status}");
    }
    // A specification written by the survey starts in `surveyed` (SPEC-0011 R4).
    assert_eq!(
        errors_of(with(specification(), "status", json!("surveyed"))),
        []
    );
    for document in [council_report(), specification(), change()] {
        let superseded = with(document, "status", json!("superseded"));
        assert_eq!(
            errors_of(superseded),
            [schema_error("/superseded_by", Reason::Requires)]
        );
    }
}

#[test]
fn an_acceptance_criterion_verified_by_a_judge_requires_a_rubric() {
    // AC35, for the living specification: `rubric` when `verification` contains `judge`.
    let mut document = specification();
    document["acceptance_criteria"][2]
        .as_object_mut()
        .unwrap()
        .remove("rubric");
    let error = only_error(document.clone());
    assert_eq!(
        (error.pointer().as_str(), error.reason()),
        ("/acceptance_criteria/2/rubric", Reason::Requires)
    );
    assert_eq!(
        error.pointers(),
        Some(&[at("/acceptance_criteria/2/verification")][..])
    );
    document["acceptance_criteria"][2]["verification"] = json!(["end_to_end"]);
    assert_eq!(errors_of(document.clone()), []);
    // A criterion cites at least one requirement and one technique, each in its form.
    let criterion = |member: &str, value: Value| {
        let mut changed = specification();
        changed["acceptance_criteria"][0][member] = value;
        errors_of(changed)
    };
    let at_first = |member: &str| format!("/acceptance_criteria/0/{member}");
    assert_eq!(
        criterion("requirements", json!([])),
        [schema_error(&at_first("requirements"), Reason::Length)]
    );
    assert_eq!(
        criterion("requirements", json!(["AC1"])),
        [schema_error(
            "/acceptance_criteria/0/requirements/0",
            Reason::Syntax
        )]
    );
    assert_eq!(
        criterion("verification", json!([])),
        [schema_error(&at_first("verification"), Reason::Length)]
    );
    assert_eq!(
        criterion("verification", json!(["unit"])),
        [schema_error(
            "/acceptance_criteria/0/verification/0",
            Reason::Enum
        )]
    );
    // A text is not a list: its error of type, and no rubric required.
    assert_eq!(
        criterion("verification", json!("judge")),
        [schema_error(&at_first("verification"), Reason::Type)]
    );
}

#[test]
fn the_operations_of_a_change_require_their_members_by_op() {
    // SPEC-0001, Change delta: `requirement` for `modified`, `removed` and `renamed`; `text`
    // for `added` and `modified`; `name` for `renamed`. Each missing member is refused with
    // `requires` and the pointer of `op`.
    let operation = |op: Value| errors_of(with(change(), "operations", json!([op])));
    let requires = |member: &str| {
        let error = schema_error(&format!("/operations/0/{member}"), Reason::Requires);
        vec![error]
    };
    assert_eq!(
        operation(json!({"op": "modified", "text": "T"})),
        requires("requirement")
    );
    assert_eq!(
        operation(json!({"op": "modified", "requirement": "R1"})),
        requires("text")
    );
    assert_eq!(operation(json!({"op": "removed"})), requires("requirement"));
    assert_eq!(
        operation(json!({"op": "renamed", "name": "N"})),
        requires("requirement")
    );
    assert_eq!(
        operation(json!({"op": "renamed", "requirement": "R1"})),
        requires("name")
    );
    assert_eq!(
        operation(json!({"op": "added", "name": "N"})),
        requires("text")
    );
    let error = only_error(with(change(), "operations", json!([{"op": "added"}])));
    assert_eq!(error.pointers(), Some(&[at("/operations/0/op")][..]));
    // What each op needs, and the optional `name` of `added` and `modified`, is accepted.
    for op in [
        json!({"op": "added", "text": "T"}),
        json!({"op": "added", "name": "N", "text": "T"}),
        json!({"op": "modified", "requirement": "R2", "name": "N", "text": "T"}),
        json!({"op": "removed", "requirement": "R2"}),
        json!({"op": "renamed", "requirement": "R2", "name": "N"}),
    ] {
        assert_eq!(operation(op.clone()), [], "{op}");
    }
    assert_eq!(
        operation(json!({"op": "deleted", "requirement": "R2"})),
        [schema_error("/operations/0/op", Reason::Enum)]
    );
    assert_eq!(
        operation(json!({"requirement": "R2"})),
        [schema_error("/operations/0/op", Reason::Missing)]
    );
    assert_eq!(
        errors_of(with(change(), "operations", json!([]))),
        [schema_error("/operations", Reason::Length)]
    );
}

#[test]
fn a_council_report_holds_its_parts_in_their_forms() {
    let part = |member: &str, value: Value| errors_of(with(council_report(), member, value));
    assert_eq!(
        part("mode", json!("one-mind")),
        [schema_error("/mode", Reason::Enum)]
    );
    assert_eq!(
        part("roles", json!([])),
        [schema_error("/roles", Reason::Length)]
    );
    assert_eq!(
        part("decisions", json!([])),
        [schema_error("/decisions", Reason::Length)]
    );
    assert_eq!(
        part(
            "decisions",
            json!([{"id": "1", "statement": "S", "reason": "R"}])
        ),
        [schema_error("/decisions/0/id", Reason::Syntax)]
    );
    assert_eq!(
        part(
            "dissent",
            json!([{"role": "critic", "position": "P", "decision": "Q1"}])
        ),
        [schema_error("/dissent/0/decision", Reason::Syntax)]
    );
    // The lists that may be empty are given, as `[]`.
    for member in ["rejected_alternatives", "dissent", "risks", "questions"] {
        assert_eq!(part(member, json!([])), [], "{member}");
        assert_eq!(
            errors_of(without(council_report(), member)),
            [schema_error(&format!("/{member}"), Reason::Missing)],
            "{member}"
        );
    }
    // A question has at least two options; its answer is absent until decided.
    let mut question = council_report()["questions"][0].clone();
    question.as_object_mut().unwrap().remove("answer");
    assert_eq!(part("questions", json!([question.clone()])), []);
    question["options"] = json!(["No"]);
    assert_eq!(
        part("questions", json!([question])),
        [schema_error("/questions/0/options", Reason::Length)]
    );
    assert_eq!(
        part("brief", json!("brief 1")),
        [schema_error("/brief", Reason::Syntax)]
    );
}

#[test]
fn the_members_of_a_living_specification_have_their_forms() {
    let member = |name: &str, value: Value| errors_of(with(specification(), name, value));
    // The capability is lower-case words joined by single hyphens, with a fix when one can be
    // derived.
    for capability in ["time-entries", "csv-log", "rfc8785", "a"] {
        assert_eq!(member("capability", json!(capability)), [], "{capability}");
    }
    for (capability, fix) in [
        ("Time entries", Some("time-entries")),
        ("time_entries", Some("time-entries")),
        ("time--entries", Some("time-entries")),
        ("-time", Some("time")),
        ("time-", Some("time")),
        ("", None),
        ("--", None),
    ] {
        let error = only_error(with(specification(), "capability", json!(capability)));
        assert_eq!(
            (error.pointer().as_str(), error.reason()),
            ("/capability", Reason::Syntax),
            "{capability}"
        );
        assert_eq!(error.fix(), fix.map(|f| json!(f)).as_ref(), "{capability}");
    }
    assert_eq!(
        member("owner", json!("example-owner")),
        [schema_error("/owner", Reason::Syntax)]
    );
    assert_eq!(
        member("commit_scope", json!("")),
        [schema_error("/commit_scope", Reason::Syntax)]
    );
    assert_eq!(
        member("stories", json!(["STORY-1"])),
        [schema_error("/stories/0", Reason::Syntax)]
    );
    assert_eq!(
        member("scope_in", json!([])),
        [schema_error("/scope_in", Reason::Length)]
    );
    assert_eq!(
        member("requirements", json!([])),
        [schema_error("/requirements", Reason::Length)]
    );
    // A requirement is `active` or `removed`; an open question may have no identifier, and
    // states whether it is resolved.
    // R4 of the example, which no criterion names, so that its status alone is tested.
    let requirement = |status: &str| {
        let mut requirements = specification()["requirements"].clone();
        requirements[3]["status"] = json!(status);
        member("requirements", requirements)
    };
    assert_eq!(requirement("active"), []);
    assert_eq!(requirement("removed"), []);
    assert_eq!(
        requirement("deleted"),
        [schema_error("/requirements/3/status", Reason::Enum)]
    );
    assert_eq!(
        member("open_questions", json!([{"text": "T", "resolved": false}])),
        []
    );
    assert_eq!(
        member("open_questions", json!([{"id": "Q1", "text": "T"}])),
        [schema_error("/open_questions/0/resolved", Reason::Missing)]
    );
    // An extra section needs a heading that is not empty; its text is absent when the section
    // has no content (decided by the maintainer on 2026-11-24).
    assert_eq!(member("extra_sections", json!([{"heading": "Notes"}])), []);
    assert_eq!(
        member("extra_sections", json!([{"heading": "", "text": "T"}])),
        [schema_error("/extra_sections/0/heading", Reason::Length)]
    );
    assert_eq!(
        member("extra_sections", json!([{"text": "T"}])),
        [schema_error("/extra_sections/0/heading", Reason::Missing)]
    );
    // R6: keys are English; an Italian key in place of the English one is unknown, with the
    // English key, which is then missing, as the fix.
    let italian = with(without(specification(), "problem"), "problema", json!("x"));
    let validation = validate(&read(linked(vec![entry(italian)])));
    assert_eq!(
        found(&validation.errors),
        [
            schema_error("/problema", Reason::UnknownField),
            schema_error("/problem", Reason::Missing)
        ]
    );
    assert_eq!(validation.errors[0].fix(), Some(&json!("problem")));
}

#[test]
fn the_oracle_agrees_with_the_interpreter_on_council_reports_specifications_and_changes() {
    let mut no_rubric = specification();
    no_rubric["acceptance_criteria"][2]
        .as_object_mut()
        .unwrap()
        .remove("rubric");
    let mut text_verification = specification();
    text_verification["acceptance_criteria"][2]["verification"] = json!("judge");
    let documents = [
        (
            council_report(),
            with(council_report(), "mode", json!("one-mind")),
        ),
        (specification(), no_rubric),
        (specification(), text_verification),
        (
            specification(),
            with(specification(), "capability", json!("A b")),
        ),
        (
            change(),
            with(
                change(),
                "operations",
                json!([{"op": "renamed", "requirement": "R1"}]),
            ),
        ),
        (
            change(),
            with(change(), "operations", json!([{"op": "removed"}])),
        ),
    ];
    for (document, refused) in documents {
        let kind = document["kind"].as_str().unwrap().to_owned();
        let contract = CONTRACTS.iter().find(|c| c.name == kind).unwrap();
        oracle_agrees(
            contract,
            [
                document.clone(),
                with(document.clone(), "status", json!("published")),
                with(document.clone(), "titolo", json!("x")),
                without(document, "title"),
                refused,
            ],
        );
    }
}

// Versions and migrations (R16, AC25) --------------------------------------------------------

const SAMPLE_V1: Embedded = embedded_test(
    "test/sample/v1.json",
    r#"{
        "type": "object", "additionalProperties": false,
        "required": ["kind", "schema_version", "name"],
        "properties": {
            "kind": {"const": "sample"},
            "schema_version": {"type": "integer"},
            "id": {"type": "string", "format": "task-id"},
            "name": {"type": "string", "minLength": 1}
        }
    }"#,
);

const SAMPLE_V2: Embedded = embedded_test(
    "test/sample/v2.json",
    r#"{
        "type": "object", "additionalProperties": false,
        "required": ["kind", "schema_version", "title"],
        "properties": {
            "kind": {"const": "sample"},
            "schema_version": {"type": "integer", "const": 2},
            "id": {"type": "string", "format": "task-id"},
            "title": {"type": "string", "minLength": 1}
        }
    }"#,
);

/// Version 1 to 2 of the sample: `name` is renamed `title`.
fn rename(mut document: Map<String, Value>) -> Map<String, Value> {
    if let Some(name) = document.remove("name") {
        document.insert("title".into(), name);
    }
    document.insert("schema_version".into(), json!(2));
    document
}

/// A migration that forgets to set the version: the current schema refuses its result.
fn broken(mut document: Map<String, Value>) -> Map<String, Value> {
    document.remove("name");
    document
}

const RENAME: [Migration; 1] = [rename];
const BROKEN: [Migration; 1] = [broken];

/// A test snapshot at `path` with `text`.
const fn embedded_test(path: &'static str, text: &'static str) -> Embedded {
    Embedded { path, text }
}

/// A test-only type with two versions and the migration between them.
fn sample_kind() -> Kind {
    Kind {
        id: "sample",
        input: None,
        code: codes::ARTIFACT_SCHEMA,
        versions: vec![SAMPLE_V1, SAMPLE_V2],
        migrations: &RENAME,
        // A test type is read as a JSON value.
        reread: reread::<Value>,
    }
}

fn sample() -> kinds::Loaded {
    kinds::load_all(&[sample_kind()]).unwrap().remove(0)
}

fn object(value: Value) -> Map<String, Value> {
    value.as_object().unwrap().clone()
}

fn check_sample(document: Value) -> Checked {
    sample().check(&object(document))
}

#[test]
fn a_document_of_an_earlier_version_is_validated_then_migrated() {
    // AC25, second half, on a type with two versions.
    assert_eq!(
        check_sample(json!({"kind": "sample", "schema_version": 1, "name": "Old"})),
        Checked::Valid(object(
            json!({"kind": "sample", "schema_version": 2, "title": "Old"})
        ))
    );
    let current = json!({"kind": "sample", "schema_version": 2, "title": "New"});
    assert_eq!(
        check_sample(current.clone()),
        Checked::Valid(object(current))
    );
}

#[test]
fn a_document_of_an_earlier_version_is_checked_against_its_own_schema() {
    let Checked::Refused(errors) =
        check_sample(json!({"kind": "sample", "schema_version": 1, "title": "x"}))
    else {
        panic!("refused expected");
    };
    assert_eq!(
        found(&errors),
        [
            (
                codes::ARTIFACT_SCHEMA,
                "/title".into(),
                Reason::UnknownField
            ),
            (codes::ARTIFACT_SCHEMA, "/name".into(), Reason::Missing),
        ]
    );
}

#[test]
fn an_unknown_version_is_refused_with_enum_and_the_known_versions() {
    // AC25, first half: R16 for every value that is not a known version.
    for version in [json!(99), json!(0), json!(-1), json!(3), json!(3.0)] {
        let document = json!({"kind": "sample", "schema_version": version, "name": "x"});
        let Checked::Refused(errors) = check_sample(document) else {
            panic!("refused expected for {version}");
        };
        assert_eq!(errors.len(), 1, "{version}");
        let error = &errors[0];
        assert_eq!(*error.code(), codes::SCHEMA_VERSION_UNKNOWN);
        assert_eq!(error.pointer(), &at("/schema_version"));
        assert_eq!(error.reason(), Reason::Enum);
        assert_eq!(error.expected(), Some(&[json!(1), json!(2)][..]));
    }
    // An integer written with a fraction of zero is that integer (plan of SPEC-0001, Numbers).
    assert!(matches!(
        check_sample(json!({"kind": "sample", "schema_version": 1.0, "name": "x"})),
        Checked::Valid(_)
    ));
}

#[test]
fn a_missing_or_mistyped_version_is_checked_against_the_current_schema_in_one_pass() {
    // Audit S3: every error of the document, in one pass (ADR 0009 C3), against the current
    // version, here v2, which knows `title` and not `name`.
    let unknown = (
        codes::ARTIFACT_SCHEMA,
        "/name".to_owned(),
        Reason::UnknownField,
    );
    let title = (codes::ARTIFACT_SCHEMA, "/title".to_owned(), Reason::Missing);
    for (version, reason) in [
        (None, Reason::Missing),
        (Some(Value::Null), Reason::Missing),
        (Some(json!("1")), Reason::Type),
        (Some(json!(1.5)), Reason::Type),
    ] {
        let mut document = json!({"kind": "sample", "name": "x"});
        if let Some(version) = version {
            document["schema_version"] = version;
        }
        let Checked::Refused(errors) = check_sample(document) else {
            panic!("refused expected");
        };
        let version = (codes::ARTIFACT_SCHEMA, "/schema_version".to_owned(), reason);
        assert_eq!(found(&errors), [unknown.clone(), version, title.clone()]);
    }
}

#[test]
fn every_version_of_every_built_schema_requires_an_integer_schema_version() {
    // The rule above relies on it: the current schema reports a missing or mistyped version.
    for kind in built() {
        for snapshot in &kind.versions {
            let schema = kinds::load(*snapshot).unwrap();
            let missing = schema.validate(&json!({}), codes::ARTIFACT_SCHEMA);
            assert!(
                found(&missing).contains(&(
                    codes::ARTIFACT_SCHEMA,
                    "/schema_version".into(),
                    Reason::Missing
                )),
                "{}",
                snapshot.path
            );
            let mistyped = schema.validate(&json!({"schema_version": "1"}), codes::ARTIFACT_SCHEMA);
            assert!(
                found(&mistyped).contains(&(
                    codes::ARTIFACT_SCHEMA,
                    "/schema_version".into(),
                    Reason::Type
                )),
                "{}",
                snapshot.path
            );
        }
    }
}

#[test]
fn a_version_beyond_the_safe_integers_is_refused_with_range() {
    // Review R2: the bound of the plan of SPEC-0001 (Numbers) holds for the version too.
    for version in [
        json!(9_007_199_254_740_992_u64),
        json!(-9_007_199_254_740_992_i64),
        json!(1e300),
    ] {
        let Checked::Refused(errors) =
            check_sample(json!({"kind": "sample", "schema_version": version, "name": "x"}))
        else {
            panic!("refused expected");
        };
        assert_eq!(
            found(&errors),
            [(
                codes::ARTIFACT_SCHEMA,
                "/schema_version".into(),
                Reason::Range
            )],
            "{version}"
        );
    }
    assert!(matches!(
        check_sample(json!({
            "kind": "sample", "schema_version": 9_007_199_254_740_991_u64, "name": "x"
        })),
        Checked::Refused(errors) if *errors[0].code() == codes::SCHEMA_VERSION_UNKNOWN
    ));
}

#[test]
fn a_migration_that_gives_an_invalid_document_is_a_failure_of_the_engine() {
    let kind = Kind {
        migrations: &BROKEN,
        ..sample_kind()
    };
    let loaded = kinds::load_all(core::slice::from_ref(&kind))
        .unwrap()
        .remove(0);
    let document = object(json!({"kind": "sample", "schema_version": 1, "name": "x"}));
    let Checked::Failed(failure) = loaded.check(&document) else {
        panic!("failure expected");
    };
    assert_eq!(*failure.code(), INTERNAL_ERROR);
    assert_eq!(failure.reason(), Reason::Internal);
    // Through validate and run: the result is a failure, exit code 5, whatever else is found.
    let input = json!({"documents": [
        entry(json!({"kind": "sample", "schema_version": 1, "name": "x"})),
        entry(json!({"kind": "ticket"})),
    ]});
    assert_eq!(input::rules(&input), []);
    let read = input::read(&input).unwrap();
    let loaded = kinds::load_all(core::slice::from_ref(&kind)).unwrap();
    let validation = validate_with(&read, &loaded);
    assert_eq!(validation.failures.len(), 1);
    assert_eq!(validation.errors.len(), 1);
    let (outcome, diagnostics, _) = run_with(&input, &[kind]);
    assert_eq!(outcome.exit_code(), crate::ExitCode::InternalError);
    assert_eq!(outcome.report().unwrap().first().reason(), Reason::Internal);
    assert_eq!(diagnostics, wayloop_schemas::Diagnostics::default());
}

#[test]
fn a_type_whose_schemas_cannot_be_loaded_is_a_failure_of_the_engine() {
    const NOT_JSON: Embedded = embedded_test("test/broken/v1.json", "{");
    const OUTSIDE: Embedded = embedded_test("test/broken/v1.json", r#"{"pattern": "x"}"#);
    for kind in [
        Kind {
            id: "broken",
            input: None,
            code: codes::ARTIFACT_SCHEMA,
            versions: vec![NOT_JSON],
            migrations: &[],
            reread: reread::<Value>,
        },
        Kind {
            id: "broken",
            input: None,
            code: codes::ARTIFACT_SCHEMA,
            versions: vec![OUTSIDE],
            migrations: &[],
            reread: reread::<Value>,
        },
        Kind {
            id: "broken",
            input: None,
            code: codes::ARTIFACT_SCHEMA,
            versions: vec![SAMPLE_V1, SAMPLE_V2],
            migrations: &[],
            reread: reread::<Value>,
        },
        Kind {
            id: "broken",
            input: None,
            code: codes::ARTIFACT_SCHEMA,
            versions: vec![],
            migrations: &[],
            reread: reread::<Value>,
        },
    ] {
        let failure = kinds::load_all(core::slice::from_ref(&kind)).unwrap_err();
        assert_eq!(failure.reason(), Reason::Internal, "{failure}");
        let (outcome, _, _) = run_with(&json!({"documents": []}), &[kind]);
        assert_eq!(outcome.exit_code(), crate::ExitCode::InternalError);
    }
}

// The identifier stage (R14) ---------------------------------------------------------------

#[test]
fn a_format_error_of_the_id_has_its_own_code_and_keeps_its_fix() {
    let validation = validate(&read(linked(vec![entry(with(
        brief(),
        "id",
        json!("brief_12"),
    ))])));
    let [error] = &validation.errors[..] else {
        panic!("{:?}", validation.errors);
    };
    assert_eq!(*error.code(), codes::IDENTIFIER_FORMAT);
    assert_eq!(error.reason(), Reason::Syntax);
    assert_eq!(error.fix(), Some(&json!("BRIEF-0012")));
    // Another fault of the id keeps the code of the schema.
    let validation = validate(&read(linked(vec![entry(with(brief(), "id", json!(12)))])));
    assert_eq!(
        found(&validation.errors),
        [(codes::ARTIFACT_SCHEMA, "/id".into(), Reason::Type)]
    );
}

#[test]
fn the_sequence_of_an_identifier_has_four_digits_up_to_9999_then_no_padding() {
    // SPEC-0001, Identifiers (R14): the canonical sequence is `[0-9]{4}|[1-9][0-9]{4,}`.
    for (id, fix) in [("TASK-107", "TASK-0107"), ("TASK-00107", "TASK-0107")] {
        let task = titled(with(task(), "id", json!(id)), 1);
        let validation = validate(&read(linked(vec![entry(task)])));
        let [error] = &validation.errors[..] else {
            panic!("{id}: {:?}", validation.errors);
        };
        assert_eq!(
            (error.code(), error.pointer(), error.reason(), error.fix()),
            (
                &codes::IDENTIFIER_FORMAT,
                &at("/id"),
                Reason::Syntax,
                Some(&json!(fix))
            ),
            "{id}"
        );
        assert_eq!(error.expected(), None);
    }
    let accepted = [
        titled(with(task(), "id", json!("TASK-10000")), 1),
        with(handover(), "id", json!("HANDOVER-12-3")),
    ];
    let validation = validate(&read(linked(accepted.into_iter().map(entry).collect())));
    assert_eq!(validation.errors, []);
    assert_eq!(validation.failures, []);
}

#[test]
fn ac1_the_invalid_milestone_is_refused_at_its_id_with_the_fix_m2_and_no_expected() {
    let document = with(milestone(), "id", json!("Milestone 2"));
    let validation = validate(&read(linked(vec![entry(document)])));
    let [error] = &validation.errors[..] else {
        panic!("{:?}", validation.errors);
    };
    assert_eq!(
        (error.code(), error.pointer(), error.reason()),
        (&codes::IDENTIFIER_FORMAT, &at("/id"), Reason::Syntax)
    );
    assert_eq!(error.fix(), Some(&json!("M2")));
    assert_eq!(error.expected(), None);
}

/// The entry of `document` read from the file `file`, with the line of its `/id`.
pub(super) fn stored(document: Value, file: &str, id_line: u64) -> Value {
    json!({
        "format": "document", "document": document,
        "origin": {"file": file}, "lines": {"/id": id_line}
    })
}

fn stored_at(file: &str, id_line: u64) -> Source {
    Source::File {
        file: ProjectPath::new(file),
        line: Some(line(id_line)),
    }
}

#[test]
fn ac6_two_artifacts_with_one_identifier_are_both_refused_each_naming_the_other() {
    let validation = validate(&read(linked(vec![
        stored(task(), "plan/items/TASK-0001.yaml", 3),
        entry(brief()),
        entry(titled(task(), 2)),
    ])));
    assert_eq!(
        found(&validation.errors),
        [
            (codes::IDENTIFIER_DUPLICATE, "/id".into(), Reason::Duplicate),
            (codes::IDENTIFIER_DUPLICATE, "/id".into(), Reason::Duplicate),
        ]
    );
    let [first, second] = &validation.errors[..] else {
        panic!("{:?}", validation.errors);
    };
    assert_eq!((first.document(), second.document()), (Some(0), Some(2)));
    assert_eq!(
        first.location(),
        Some(&stored_at("plan/items/TASK-0001.yaml", 3))
    );
    assert_eq!(second.location(), None);
    // Each names the other, with its source when it was stored.
    assert_eq!(
        first.related(),
        Some(&[Related::new(2, at("/id"), None)][..])
    );
    assert_eq!(
        second.related(),
        Some(
            &[Related::new(
                0,
                at("/id"),
                Some(stored_at("plan/items/TASK-0001.yaml", 3))
            )][..]
        )
    );
    assert_eq!(validation.warnings, []);
}

#[test]
fn a_refused_identifier_stays_out_of_the_index_and_errors_stay_in_document_order() {
    // The brief carries the identifier too, but its `id` is refused; the duplicate errors of
    // the three tasks come by document, around the format error of the brief. No error copies
    // a list that grows with the input (decided by the maintainer on 2026-11-25): the first
    // carrier names the second, and each later carrier names the first.
    let validation = validate(&read(linked(vec![
        entry(task()),
        entry(with(brief(), "id", json!("TASK-0001"))),
        entry(titled(task(), 2)),
        entry(titled(task(), 3)),
    ])));
    let documents: Vec<(Option<u64>, Code)> = validation
        .errors
        .iter()
        .map(|e| (e.document(), e.code().clone()))
        .collect();
    assert_eq!(
        documents,
        [
            (Some(0), codes::IDENTIFIER_DUPLICATE),
            (Some(1), codes::IDENTIFIER_FORMAT),
            (Some(2), codes::IDENTIFIER_DUPLICATE),
            (Some(3), codes::IDENTIFIER_DUPLICATE),
        ]
    );
    let named: Vec<Vec<u64>> = validation
        .errors
        .iter()
        .filter(|e| e.code() == &codes::IDENTIFIER_DUPLICATE)
        .map(|e| e.related().unwrap().iter().map(Related::document).collect())
        .collect();
    assert_eq!(named, [vec![2], vec![0], vec![0]]);
}

#[test]
fn duplicates_are_found_in_the_same_pass_as_the_other_errors_of_their_documents() {
    // ADR 0009 C3: a document with an error elsewhere than at its `id` is still indexed, so
    // both carriers are refused with `identifier_duplicate`, after the schema error of the
    // first.
    let validation = validate(&read(linked(vec![
        entry(with(task(), "zz", json!(1))),
        entry(titled(task(), 1)),
    ])));
    let found: Vec<(Option<u64>, Code, String)> = validation
        .errors
        .iter()
        .map(|e| (e.document(), e.code().clone(), e.pointer().to_string()))
        .collect();
    assert_eq!(
        found,
        [
            (Some(0), codes::ARTIFACT_SCHEMA, "/zz".to_owned()),
            (Some(0), codes::IDENTIFIER_DUPLICATE, "/id".to_owned()),
            (Some(1), codes::IDENTIFIER_DUPLICATE, "/id".to_owned()),
        ]
    );
    assert_eq!(
        validation.errors[1].related(),
        Some(&[Related::new(1, at("/id"), None)][..])
    );
}

#[test]
fn a_file_name_or_folder_number_that_names_another_identifier_gives_a_warning() {
    let specification = specification();
    let validation = validate(&read(linked(vec![
        stored(brief(), "plan/briefs/BRIEF-0002.yaml", 4),
        stored(task(), "plan/items/TASK-0001.yaml", 3),
        stored(specification.clone(), "specs/0002-time-entries/spec.md", 1),
        stored(
            with(brief(), "id", json!("BRIEF-0005")),
            "notes/BRIEF-0009.yaml",
            2,
        ),
        entry(titled(with(task(), "id", json!("TASK-0002")), 2)),
    ])));
    assert_eq!(validation.errors, []);
    assert_eq!(
        found(&validation.warnings),
        [
            (codes::IDENTIFIER_LOCATION, "/id".into(), Reason::Conflict),
            (codes::IDENTIFIER_LOCATION, "/id".into(), Reason::Conflict),
        ]
    );
    let [brief_warning, spec_warning] = &validation.warnings[..] else {
        panic!("{:?}", validation.warnings);
    };
    assert_eq!(brief_warning.document(), Some(0));
    assert_eq!(brief_warning.fix(), Some(&json!("BRIEF-0001")));
    assert_eq!(
        brief_warning.location(),
        Some(&stored_at("plan/briefs/BRIEF-0002.yaml", 4))
    );
    assert_eq!(spec_warning.document(), Some(2));
    assert_eq!(spec_warning.fix(), Some(&specification["id"]));
    // The same specification in the folder of its number gives none.
    let validation = validate(&read(linked(vec![stored(
        specification,
        "specs/0001-time-entries/spec.md",
        1,
    )])));
    assert_eq!(validation.warnings, []);
}

// The set ----------------------------------------------------------------------------------

#[test]
fn a_project_without_any_location_gives_a_warning_and_the_check_runs() {
    // AC37: discovery outside the root of a project finds no folder and no file.
    for (locations, warned) in [(json!([]), true), (json!(["specs"]), false)] {
        let input = json!({"documents": [], "locations": locations});
        assert_eq!(input_schema().validate(&input, COMMAND_INPUT), []);
        let validation = validate(&input::read(&input).unwrap());
        assert_eq!(validation.errors, []);
        assert!(validation.checks.contains(&checks::ARTIFACT_LOCATIONS));
        assert!(
            validation
                .not_done
                .iter()
                .all(|n| *n.check() != checks::ARTIFACT_LOCATIONS)
        );
        let warnings = found(&validation.warnings);
        if warned {
            assert_eq!(
                warnings,
                [(
                    codes::ARTIFACT_LOCATION_MISSING,
                    "/locations".into(),
                    Reason::Missing
                )]
            );
        } else {
            assert_eq!(warnings, []);
        }
    }
    // A folder outside the artifact model is refused by the schema.
    let input = json!({"documents": [], "locations": ["plan"]});
    let errors = input_schema().validate(&input, COMMAND_INPUT);
    assert_eq!(
        found(&errors),
        [(COMMAND_INPUT, "/locations/0".into(), Reason::Enum)]
    );
}

#[test]
fn the_folders_of_the_contract_are_those_of_the_locations() {
    let folders: Vec<ArtifactFolder> = LOCATIONS.iter().map(Location::folder).collect();
    assert_eq!(folders, ArtifactFolder::ALL);
}

#[test]
fn checks_not_built_are_listed_once_with_their_reason() {
    // Every type has its schema built: a missing one is a failure of the engine, never a check
    // not built (decided by the maintainer on 2026-11-25).
    let types = built();
    let input = read(linked(vec![
        json!({"format": "markdown", "text": super::markdown::tests::EXAMPLE,
               "origin": {"file": "specs/0001-time-entries/spec.md"}}),
        entry(brief()),
    ]));
    let validation = super::validate(&input, &types);
    assert_eq!(validation.errors, []);
    // The valid brief is read into its contract type (ADR 0030 G1), not left as a value.
    assert_eq!(validation.failures, []);
    // Each check not done is listed once, in the order of its identifier.
    let mut expected_not_done: Vec<String> = checks::NOT_BUILT
        .iter()
        .chain([&checks::ARTIFACT_LOCATIONS, &checks::ADR_CITATIONS])
        .map(|c| c.id().to_owned())
        .collect();
    expected_not_done.sort();
    let not_done: Vec<&str> = validation.not_done.iter().map(|n| n.check().id()).collect();
    assert_eq!(not_done, expected_not_done);
    // The locations were not given: the documents were not discovered (AC37); nor were the
    // ADRs of the project. Every other check is not built.
    for not in &validation.not_done {
        let given = [&checks::ARTIFACT_LOCATIONS, &checks::ADR_CITATIONS];
        let expected = if given.contains(&not.check()) {
            NotDoneReason::InputAbsent
        } else {
            NotDoneReason::NotBuilt
        };
        assert_eq!(not.reason(), expected, "{}", not.check().id());
    }
    // One check identifier per schema (S6): the schemas of the built types and inputs ran, in
    // their order.
    let schemas = types.iter().map(|k| kinds::schema_check(k.id));
    let mut expected = vec![
        checks::MARKDOWN,
        checks::DOCUMENT_KIND,
        checks::SCHEMA_VERSION,
    ];
    expected.extend(schemas);
    expected.extend([
        checks::IDENTIFIER_FORMAT,
        checks::IDENTIFIER_DUPLICATE,
        checks::IDENTIFIER_LOCATION,
        checks::LINKS,
        checks::PARTS,
        checks::TITLE_FORMAT,
        checks::TITLE_DUPLICATE,
        checks::BRANCH_NAME,
        checks::COMMIT_HEADER,
    ]);
    assert_eq!(validation.checks, expected);
    let summaries: Vec<(Option<&str>, Option<&str>, Option<&str>)> = validation
        .documents
        .iter()
        .map(|d| {
            (
                d.kind.map(wayloop_schemas::artifact::ArtifactKind::id),
                d.input_kind.map(wayloop_schemas::validate::InputKind::id),
                d.id.as_deref(),
            )
        })
        .collect();
    // The two documents of the test come first; the targets of their links follow.
    assert_eq!(
        summaries.get(..2),
        Some(
            &[
                (Some("specification"), None, Some("SPEC-0001")),
                (Some("brief"), None, Some("BRIEF-0001")),
            ][..]
        )
    );
    assert_eq!(validation.warnings, []);
    assert!(
        validation
            .not_done
            .iter()
            .all(|n| n.check() != &checks::LINKS)
    );
}

#[test]
fn errors_come_in_the_order_of_the_documents_then_of_their_values() {
    let input = read(linked(vec![
        entry(with(with(brief(), "status", json!("done")), "zz", json!(1))),
        entry(json!({"kind": 7})),
        entry(json!({"kind": "ticket"})),
        entry(with(
            with(brief(), "id", json!("BRIEF-0002")),
            "schema_version",
            json!(2),
        )),
    ]));
    let validation = validate(&input);
    assert_eq!(
        found(&validation.errors),
        [
            (codes::ARTIFACT_SCHEMA, "/status".into(), Reason::Enum),
            (codes::ARTIFACT_SCHEMA, "/zz".into(), Reason::UnknownField),
            (codes::ARTIFACT_SCHEMA, "/kind".into(), Reason::Type),
            (codes::ARTIFACT_SCHEMA, "/kind".into(), Reason::Enum),
            (
                codes::SCHEMA_VERSION_UNKNOWN,
                "/schema_version".into(),
                Reason::Enum
            ),
        ]
    );
    assert_eq!(
        validation.errors[3].expected().map(<[Value]>::len),
        Some(ArtifactKind::ALL.len())
    );
    // Same input, same result.
    assert_eq!(validate(&input), validation);
}

// Sources (R12) ----------------------------------------------------------------------------

fn sources(origin: Value, lines: Option<Value>, document: Value) -> Vec<Option<Source>> {
    let mut entry = json!({"format": "document", "document": document, "origin": origin});
    if let Some(lines) = lines {
        entry["lines"] = lines;
    }
    validate(&read(linked(vec![entry])))
        .errors
        .iter()
        .map(|e| e.location().cloned())
        .collect()
}

#[test]
fn the_source_of_an_error_is_the_line_of_its_value_or_of_the_nearest_value_holding_it() {
    let file = |line: Option<u64>| {
        Some(Source::File {
            file: ProjectPath::new("plan/briefs/BRIEF-0001.yaml"),
            line: line.map(self::line),
        })
    };
    let document = with(
        with(brief(), "goals", json!(["", "Export"])),
        "problem",
        Value::Null,
    );
    let origin = json!({"file": "plan/briefs/BRIEF-0001.yaml"});
    // /goals/0 has no line of its own: the line of /goals; /problem is missing: the root.
    assert_eq!(
        sources(
            origin.clone(),
            Some(json!({"": 2, "/goals": 9, "/goals/1": 11})),
            document.clone()
        ),
        [file(Some(9)), file(Some(2))]
    );
    // No line given for the value nor for a value holding it: no line, never a guess.
    assert_eq!(
        sources(origin.clone(), None, document.clone()),
        [file(None), file(None)]
    );
    assert_eq!(
        sources(origin, Some(json!({"/goals/1": 11})), document.clone()),
        [file(None), file(None)]
    );
    let issue = Some(Source::Issue {
        issue: IssueNumber::new(line(12).get()),
    });
    assert_eq!(
        sources(json!({"issue": 12}), None, document.clone()),
        [issue.clone(), issue]
    );
    let milestone = Some(Source::Milestone {
        milestone: MilestoneNumber::new(line(3).get()),
    });
    assert_eq!(
        sources(json!({"milestone": 3.0}), None, document),
        [milestone.clone(), milestone]
    );
}

#[test]
fn an_unreadable_entry_is_a_syntax_error_at_the_line_of_the_parser() {
    let validation = validate(&read(linked(vec![
        entry(brief()),
        json!({"format": "unreadable", "origin": {"file": "plan/briefs/BRIEF-0002.yaml"},
               "error": {"line": 4, "message": "found a tab"}}),
    ])));
    let [error] = &validation.errors[..] else {
        panic!("{:?}", validation.errors);
    };
    assert_eq!(*error.code(), codes::ARTIFACT_SCHEMA);
    assert_eq!(error.pointer(), &Pointer::root());
    assert_eq!(error.reason(), Reason::Syntax);
    assert_eq!(
        error.location(),
        Some(&Source::File {
            file: ProjectPath::new("plan/briefs/BRIEF-0002.yaml"),
            line: Some(line(4))
        })
    );
    assert_eq!(error.document(), Some(1));
    assert!(error.message().contains("found a tab"));
}

#[test]
fn an_unreadable_entry_needs_a_file_origin() {
    // Only a file can fail to parse (review R1).
    let input = json!({"documents": [
        {"format": "unreadable", "error": {"line": 1, "message": "empty"}},
        {"format": "unreadable", "origin": {"issue": 5}, "error": {"line": 4, "message": "m"}},
    ]});
    assert_eq!(
        refused_input(&input),
        [
            (
                "/documents/0/origin".into(),
                Reason::Requires,
                Some(vec!["/documents/0/format".into()])
            ),
            (
                "/documents/1/origin".into(),
                Reason::Conflict,
                Some(vec!["/documents/1/format".into()])
            ),
        ]
    );
}

#[test]
fn every_error_of_a_document_carries_its_index_and_errors_of_the_input_none() {
    // SPEC-0004 R2 as amended: the pointer is relative to the document named by `document`.
    let validation = validate(&read(linked(vec![
        entry(brief()),
        entry(json!({"kind": "ticket"})),
        entry(with(
            with(brief(), "id", json!("BRIEF-0002")),
            "status",
            json!("done"),
        )),
    ])));
    let documents: Vec<Option<u64>> = validation.errors.iter().map(Error::document).collect();
    assert_eq!(documents, [Some(1), Some(2)]);
    let envelope = run(
        Command::Validate,
        &json!({"documents": [], "adrs": [0]}),
        &Context::default(),
    );
    let Some(report) = envelope.outcome().report() else {
        panic!("refused expected");
    };
    assert_eq!(report.first().document(), None);
}

// Rules of the input beyond its schema ----------------------------------------------------

fn refused_input(input: &Value) -> Vec<(String, Reason, Option<Vec<String>>)> {
    let schema = input_schema();
    assert_eq!(schema.validate(input, COMMAND_INPUT), [], "{input}");
    let errors = input::rules(input);
    errors
        .iter()
        .inspect(|e| assert_eq!(*e.code(), COMMAND_INPUT))
        .map(|e| {
            (
                e.pointer().to_string(),
                e.reason(),
                e.pointers()
                    .map(|p| p.iter().map(ToString::to_string).collect()),
            )
        })
        .collect()
}

#[test]
fn a_member_of_another_format_is_a_conflict_with_the_format() {
    let input = json!({"documents": [
        {"format": "document", "document": {}, "text": "x", "error": {"line": 1, "message": "m"}},
        {"format": "markdown", "text": "x", "lines": {}, "input_kind": "commit"},
        {
            "format": "unreadable", "error": {"line": 1, "message": "m"}, "document": {},
            "text": null
        },
    ]});
    let format = |i: usize| Some(vec![format!("/documents/{i}/format")]);
    assert_eq!(
        refused_input(&input),
        [
            ("/documents/0/error".into(), Reason::Conflict, format(0)),
            ("/documents/0/text".into(), Reason::Conflict, format(0)),
            (
                "/documents/1/input_kind".into(),
                Reason::Conflict,
                format(1)
            ),
            ("/documents/1/lines".into(), Reason::Conflict, format(1)),
            ("/documents/2/document".into(), Reason::Conflict, format(2)),
            // An unreadable entry needs a file origin (review R1).
            (
                "/documents/2/origin".into(),
                Reason::Requires,
                Some(vec!["/documents/2/format".into()])
            ),
        ]
    );
}

#[test]
fn an_origin_gives_exactly_one_place() {
    let input = json!({"documents": [
        {"format": "markdown", "text": "x", "origin": {}},
        {"format": "markdown", "text": "x", "origin": {"file": null}},
        {"format": "markdown", "text": "x", "origin": {"issue": 1, "file": "a", "milestone": 2}},
        {"format": "markdown", "text": "x", "origin": {"issue": 1, "file": null}},
    ]});
    assert_eq!(
        refused_input(&input),
        [
            ("/documents/0/origin".into(), Reason::Missing, None),
            ("/documents/1/origin".into(), Reason::Missing, None),
            (
                "/documents/2/origin".into(),
                Reason::Conflict,
                Some(vec![
                    "/documents/2/origin/file".into(),
                    "/documents/2/origin/issue".into(),
                    "/documents/2/origin/milestone".into(),
                ])
            ),
        ]
    );
}

#[test]
fn lines_need_a_file_origin() {
    let input = json!({"documents": [
        {"format": "document", "document": {}, "lines": {"": 1}},
        {"format": "document", "document": {}, "lines": {"": 1}, "origin": {"issue": 3}},
        {"format": "document", "document": {}, "lines": {"": 1}, "origin": {}},
        {"format": "document", "document": {}, "lines": {"": 1}, "origin": {"file": "a.yaml"}},
    ]});
    assert_eq!(
        refused_input(&input),
        [
            (
                "/documents/0/origin".into(),
                Reason::Requires,
                Some(vec!["/documents/0/lines".into()])
            ),
            (
                "/documents/1/origin".into(),
                Reason::Conflict,
                Some(vec!["/documents/1/lines".into()])
            ),
            ("/documents/2/origin".into(), Reason::Missing, None),
        ]
    );
}

#[test]
fn the_schema_keys_lines_by_pointers_and_takes_line_numbers_from_1() {
    // #307: the schema states the rules of `lines`, which the core no longer repeats.
    let input = json!({"documents": [
        {"format": "document", "document": {}, "origin": {"file": "a.yaml"},
         "lines": {"x": 1, "/a": "2", "/b": 0, "/c": 2.5, "/d": 9_007_199_254_740_992_u64,
                   "/e": null, "/f": 3.0, "/~2": 1, "": 1, "/~0~1": 2}},
    ]});
    let errors = input_schema().validate(&input, COMMAND_INPUT);
    let found: Vec<(String, Reason)> = errors
        .iter()
        .map(|e| (e.pointer().to_string(), e.reason()))
        .collect();
    assert_eq!(
        found,
        [
            ("/documents/0/lines/~1a".into(), Reason::Type),
            ("/documents/0/lines/~1b".into(), Reason::Range),
            ("/documents/0/lines/~1c".into(), Reason::Type),
            ("/documents/0/lines/~1d".into(), Reason::Range),
            ("/documents/0/lines/~1~02".into(), Reason::Syntax),
            ("/documents/0/lines/x".into(), Reason::Syntax),
        ]
    );
    let names = errors.iter().filter(|e| e.reason() == Reason::Syntax);
    for error in names {
        assert_eq!(
            error.message(),
            "The name of the member is not in the form of the format `json-pointer`."
        );
    }
    assert_eq!(input::rules(&input), []);
    // Contract audit of #307: an external validator that checks formats reads each line the
    // same way, beyond the safe-integer bound of the core and the nulls of ADR 0009 C2, which
    // the oracle does not share.
    let text = VALIDATE
        .embedded(Part::Schema, VALIDATE.version)
        .unwrap()
        .text;
    let oracle = oracle(&serde_json::from_str(text).unwrap());
    for (key, line) in [
        ("x", json!(1)),
        ("/a", json!("2")),
        ("/b", json!(0)),
        ("/c", json!(2.5)),
        ("/f", json!(3.0)),
        ("/~2", json!(1)),
        ("", json!(1)),
        ("/~0~1", json!(2)),
    ] {
        let one = json!({"documents": [{"format": "document", "document": {},
            "origin": {"file": "a.yaml"}, "lines": {key: line}}]});
        let core = input_schema().validate(&one, COMMAND_INPUT).is_empty();
        assert_eq!(oracle.is_valid(&one), core, "{key}: {line}");
    }
}

// The entry point --------------------------------------------------------------------------

#[test]
fn the_entry_point_reports_the_dry_run_of_the_context_and_hashes_the_input() {
    let input = json!({"documents": [entry(brief())]});
    let envelope = run(
        Command::Validate,
        &input,
        &Context::default().with_dry_run(true),
    );
    assert!(envelope.meta().dry_run());
    assert_eq!(envelope.command(), "validate");
    assert_eq!(envelope.meta().command_version().get(), 1);
    assert_eq!(
        envelope.meta().input_hash(),
        &crate::envelope::input_hash(&input)
    );
    let text = serde_json::to_vec(&input).unwrap();
    assert_eq!(
        run_json(
            Command::Validate,
            &text,
            &Context::default().with_dry_run(true)
        ),
        envelope
    );
}

#[test]
fn a_refused_input_runs_no_check_of_the_documents() {
    let input = json!({"documents": [entry(json!({"kind": "ticket"}))], "adrs": [-1]});
    let envelope = run(Command::Validate, &input, &Context::default());
    let Some(report) = envelope.outcome().report() else {
        panic!("refused expected");
    };
    assert_eq!(
        found(&report.errors().cloned().collect::<Vec<_>>()),
        [(COMMAND_INPUT, "/adrs/0".into(), Reason::Range)]
    );
    assert_eq!(envelope.diagnostics().checks(), []);
    let input = json!({"documents": [{"format": "markdown", "text": "x", "origin": {}}]});
    let envelope = run(Command::Validate, &input, &Context::default());
    assert_eq!(envelope.exit_code().code(), 1);
    assert_eq!(envelope.diagnostics().not_done(), []);
}

#[test]
fn an_input_that_is_not_an_object_is_refused_with_type_at_the_root() {
    for input in [json!([]), json!(null), json!("documents")] {
        let envelope = run(Command::Validate, &input, &Context::default());
        let Some(report) = envelope.outcome().report() else {
            panic!("refused expected");
        };
        assert_eq!(report.first().pointer(), &Pointer::root());
        assert_eq!(report.first().reason(), Reason::Type);
    }
}

#[test]
fn identifier_errors_come_after_the_schema_errors_of_their_document() {
    // Review R4: by document, then by stage (schema, then identifier), then in the order of
    // the schema pass, members in the byte order of their keys (`/aa` before `/zz`).
    let document = with(
        with(with(brief(), "id", json!("brief-7")), "zz", json!(1)),
        "aa",
        json!(1),
    );
    let validation = validate(&read(linked(vec![entry(document)])));
    assert_eq!(
        found(&validation.errors),
        [
            (codes::ARTIFACT_SCHEMA, "/aa".into(), Reason::UnknownField),
            (codes::ARTIFACT_SCHEMA, "/zz".into(), Reason::UnknownField),
            (codes::IDENTIFIER_FORMAT, "/id".into(), Reason::Syntax),
        ]
    );
}

#[test]
fn without_a_requested_version_the_current_one_runs_and_is_listed_as_a_default() {
    // ADR 0028 as amended on 2026-11-20.
    let envelope = run(
        Command::Validate,
        &json!({"documents": []}),
        &Context::default(),
    );
    assert_eq!(envelope.exit_code().code(), 0);
    let [default] = envelope.meta().defaults_applied() else {
        panic!("{:?}", envelope.meta().defaults_applied());
    };
    assert_eq!(default.reason().id(), "current_command_version");
    assert_eq!(default.value(), &json!(1));
    assert_eq!(default.pointer(), &Pointer::root());
    let one = core::num::NonZeroU32::MIN;
    let requested = run(
        Command::Validate,
        &json!({"documents": []}),
        &Context::default().with_command_version(one),
    );
    assert_eq!(requested.meta().defaults_applied(), []);
    assert_eq!(requested.meta().command_version(), one);
}

#[test]
fn an_unknown_requested_version_is_refused_with_the_known_versions() {
    let two = core::num::NonZeroU32::new(2).unwrap();
    let context = Context::default().with_command_version(two);
    // With text that is not JSON, the syntax error follows the version in the same report.
    for (envelope, count) in [
        (
            run(Command::Validate, &json!({"documents": []}), &context),
            1,
        ),
        (run_json(Command::Validate, b"{", &context), 2),
    ] {
        let Some(report) = envelope.outcome().report() else {
            panic!("refused expected");
        };
        let error = report.first();
        assert_eq!(*error.code(), COMMAND_INPUT);
        assert_eq!(error.reason(), Reason::Enum);
        assert_eq!(error.pointer(), &Pointer::root());
        assert_eq!(error.expected(), Some(&[json!(1)][..]));
        let reasons: Vec<Reason> = report.errors().map(Error::reason).collect();
        assert_eq!(reasons, [Reason::Enum, Reason::Syntax][..count]);
        assert_eq!(envelope.meta().command_version().get(), 1);
        assert_eq!(envelope.meta().defaults_applied(), []);
    }
    // Text that is not JSON with no version requested: the default is still listed.
    let envelope = run_json(Command::Validate, b"{", &Context::default());
    assert_eq!(envelope.meta().defaults_applied().len(), 1);
}

#[test]
fn usage_and_unreadable_input_envelopes_carry_the_error_and_their_exit_code() {
    let usage = crate::usage_error("nope", "Unknown command.");
    assert_eq!(usage.command(), "nope");
    assert_eq!(usage.exit_code().code(), 2);
    let Some(report) = usage.outcome().report() else {
        panic!("usage expected");
    };
    assert_eq!(*report.first().code(), crate::USAGE);
    assert_eq!(report.first().reason(), Reason::Syntax);
    assert_eq!(report.first().message(), "Unknown command.");
    assert_eq!(usage.meta().command_version().get(), 1);
    // SHA-256 of no bytes.
    assert_eq!(
        usage.meta().input_hash().as_str(),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(crate::usage_error("validate", "x").command(), "validate");
    for (reason, message) in [
        (Reason::Missing, "The input does not exist."),
        (Reason::State, "The input cannot be read."),
    ] {
        let envelope = crate::input_unreadable(Command::Validate, reason);
        assert_eq!(envelope.command(), "validate");
        assert_eq!(envelope.exit_code().code(), 3);
        let Some(report) = envelope.outcome().report() else {
            panic!("environment expected");
        };
        assert_eq!(*report.first().code(), crate::INPUT_UNREADABLE);
        assert_eq!(report.first().reason(), reason);
        assert_eq!(report.first().message(), message);
    }
    // Another reason is not a reason of the code: a failure of the engine, not a guess.
    let envelope = crate::input_unreadable(Command::Validate, Reason::Syntax);
    assert_eq!(envelope.exit_code().code(), 5);
    let Some(report) = envelope.outcome().report() else {
        panic!("failure expected");
    };
    assert_eq!(*report.first().code(), crate::INTERNAL_ERROR);
    assert!(report.first().message().contains("syntax"), "{report:?}");
}

#[test]
fn a_file_origin_outside_the_project_is_refused() {
    // Audit B2: no absolute or escaping path.
    for file in [
        "/etc/passwd",
        "../outside.yaml",
        "plan\\briefs\\a.yaml",
        "C:/x.yaml",
    ] {
        let input = json!({"documents": [
            {"format": "markdown", "text": "x", "origin": {"file": file}}
        ]});
        let envelope = run(Command::Validate, &input, &Context::default());
        let Some(report) = envelope.outcome().report() else {
            panic!("refused expected for {file}");
        };
        assert_eq!(
            found(&report.errors().cloned().collect::<Vec<_>>()),
            [(
                COMMAND_INPUT,
                "/documents/0/origin/file".into(),
                Reason::Syntax
            )],
            "{file}"
        );
    }
}

#[test]
fn every_artifact_type_has_its_schema_built() {
    // After T7 to T10 every type of SPEC-0001 has a schema; a type declared without one would
    // make every run with a document of that type a failure of the engine.
    let built = kinds::built().unwrap();
    for kind in ArtifactKind::ALL {
        let found = built
            .iter()
            .filter(|k| k.input.is_none() && k.id == kind.id());
        assert_eq!(found.count(), 1, "{}", kind.id());
    }
}

// Pull request and commit inputs (T10: R1, R16; AC1, structural part) -----------------------

#[test]
fn valid_inputs_pass_and_their_schemas_run() {
    let validation = validate(&read(vec![
        input_entry(pull_request(), InputKind::PullRequest),
        input_entry(commit(), InputKind::Commit),
        // A commit of one line has no body, so it marks no breaking change (R11).
        input_entry(
            with(
                without(commit(), "body"),
                "header",
                json!("feat(core): store durations in seconds"),
            ),
            InputKind::Commit,
        ),
    ]));
    passed_with(
        &validation,
        &[
            "schema_commit",
            "schema_pull_request",
            "branch_name",
            "commit_header",
        ],
    );
    // The links of a pull request are a later check (R10).
    assert!(
        validation
            .not_done
            .iter()
            .any(|n| n.check() == &checks::PULL_REQUEST_LINK)
    );
    let kinds: Vec<Option<InputKind>> = validation.documents.iter().map(|d| d.input_kind).collect();
    assert_eq!(
        kinds,
        [
            Some(InputKind::PullRequest),
            Some(InputKind::Commit),
            Some(InputKind::Commit)
        ]
    );
    assert!(validation.documents.iter().all(|d| d.kind.is_none()));
}

#[test]
fn the_oracle_agrees_with_the_interpreter_on_inputs() {
    oracle_agrees(
        &PULL_REQUEST,
        [
            pull_request(),
            with(pull_request(), "body", json!("")),
            with(pull_request(), "title", json!("")),
            without(pull_request(), "branch"),
            without(pull_request(), "schema_version"),
            with(pull_request(), "labels", json!(["ok", 7])),
            with(pull_request(), "labels", json!([""])),
            with(pull_request(), "kind", json!("pull_request")),
            commit(),
        ],
    );
    oracle_agrees(
        &COMMIT,
        [
            commit(),
            with(commit(), "body", json!("Refs: #12")),
            with(commit(), "body", json!("")),
            with(commit(), "body", json!(1)),
            with(commit(), "header", json!("")),
            with(commit(), "schema_version", json!("1")),
            without(commit(), "header"),
            pull_request(),
        ],
    );
}

#[test]
fn input_kind_selects_the_schema() {
    // The pull request sent as a commit: its members other than the body are unknown to the
    // commit, which misses its header. Present members in byte order, then absent ones in the
    // order of `required`.
    assert_eq!(
        input_errors(pull_request(), InputKind::Commit),
        [
            schema_error("/branch", Reason::UnknownField),
            schema_error("/labels", Reason::UnknownField),
            schema_error("/title", Reason::UnknownField),
            schema_error("/header", Reason::Missing),
        ]
    );
    // The commit of one line sent as a pull request.
    assert_eq!(
        input_errors(without(commit(), "body"), InputKind::PullRequest),
        [
            schema_error("/header", Reason::UnknownField),
            schema_error("/title", Reason::Missing),
            schema_error("/body", Reason::Missing),
            schema_error("/branch", Reason::Missing),
            schema_error("/labels", Reason::Missing),
        ]
    );
}

#[test]
fn a_missing_branch_or_header_is_missing() {
    assert_eq!(
        input_errors(without(pull_request(), "branch"), InputKind::PullRequest),
        [schema_error("/branch", Reason::Missing)]
    );
    assert_eq!(
        input_errors(without(commit(), "header"), InputKind::Commit),
        [schema_error("/header", Reason::Missing)]
    );
    // A member set to null is not given (ADR 0009 C2).
    assert_eq!(
        input_errors(with(commit(), "header", Value::Null), InputKind::Commit),
        [schema_error("/header", Reason::Missing)]
    );
}

#[test]
fn the_texts_of_an_input_are_not_empty_except_the_body_of_a_pull_request() {
    // Decided by the maintainer on 2026-11-24 (SPEC-0001, Inputs).
    for member in ["title", "branch"] {
        assert_eq!(
            input_errors(
                with(pull_request(), member, json!("")),
                InputKind::PullRequest
            ),
            [schema_error(&format!("/{member}"), Reason::Length)]
        );
    }
    for member in ["header", "body"] {
        assert_eq!(
            input_errors(with(commit(), member, json!("")), InputKind::Commit),
            [schema_error(&format!("/{member}"), Reason::Length)]
        );
    }
    assert_eq!(
        input_errors(
            with(pull_request(), "body", json!("")),
            InputKind::PullRequest
        ),
        []
    );
    // A label is a non-empty text; the list may be empty (decided by the maintainer on
    // 2026-11-25).
    assert_eq!(
        input_errors(
            with(pull_request(), "labels", json!([""])),
            InputKind::PullRequest
        ),
        [schema_error("/labels/0", Reason::Length)]
    );
}

#[test]
fn the_members_of_an_input_have_their_types() {
    assert_eq!(
        input_errors(
            with(pull_request(), "labels", json!(["ok", 7])),
            InputKind::PullRequest
        ),
        [schema_error("/labels/1", Reason::Type)]
    );
    assert_eq!(
        input_errors(with(commit(), "body", json!(["x"])), InputKind::Commit),
        [schema_error("/body", Reason::Type)]
    );
}

#[test]
fn an_input_carries_its_schema_version_and_no_kind() {
    // Decided by the maintainer on 2026-11-24: every input carries `schema_version` (R16).
    for (document, input_kind) in [
        (pull_request(), InputKind::PullRequest),
        (commit(), InputKind::Commit),
    ] {
        assert_eq!(
            input_errors(without(document.clone(), "schema_version"), input_kind),
            [schema_error("/schema_version", Reason::Missing)]
        );
        // An input has no kind, identifier or status (SPEC-0001, Inputs).
        assert_eq!(
            input_errors(with(document.clone(), "kind", json!("commit")), input_kind),
            [schema_error("/kind", Reason::UnknownField)]
        );
        // R16: a version the engine does not know is refused with the known versions.
        version_unknown(with(document, "schema_version", json!(99)), input_kind);
    }
}

#[test]
fn an_input_without_input_kind_and_without_kind_is_missing_at_kind() {
    for document in [pull_request(), commit()] {
        assert_eq!(
            errors_of(document),
            [schema_error("/kind", Reason::Missing)]
        );
    }
}

#[test]
fn a_kind_naming_an_input_is_not_an_artifact_type() {
    // `kind` selects artifacts only: the name of an input there is outside the list.
    for name in ["commit", "pull_request"] {
        let validation = validate(&read(linked(vec![entry(with(
            commit(),
            "kind",
            json!(name),
        ))])));
        assert_eq!(
            found(&validation.errors),
            [schema_error("/kind", Reason::Enum)]
        );
        let expected: Vec<Value> = kinds::kinds().into_iter().map(Value::from).collect();
        assert_eq!(validation.errors[0].expected(), Some(&expected[..]));
    }
}

#[test]
fn a_type_without_its_schema_is_a_failure_of_the_engine() {
    // Every artifact type and input kind is built; a set of types without one is a defect, never
    // a check left out as not built (decided by the maintainer on 2026-11-25).
    let cases = [
        (
            "task",
            entry(task()),
            "The artifact type task has no schema loaded, so the check schema_task cannot run.",
        ),
        (
            "commit",
            input_entry(commit(), InputKind::Commit),
            "The input kind commit has no schema loaded, so the check schema_commit cannot run.",
        ),
    ];
    for (id, document, message) in cases {
        let mut types = built();
        types.retain(|k| k.id != id);
        assert_eq!(types.len(), built().len() - 1, "{id}");
        let validation = super::validate(&read(linked(vec![document])), &types);
        assert_eq!(validation.errors, [], "{id}");
        assert_eq!(validation.failures.len(), 1, "{id}");
        assert_eq!(validation.failures[0].reason(), Reason::Internal, "{id}");
        assert_eq!(validation.failures[0].message(), message);
        let check = kinds::schema_check(id);
        assert!(
            validation.not_done.iter().all(|n| n.check() != &check),
            "{id}"
        );
    }
}

// Titles of epics and tasks (T16: R7, R8; AC1, AC4, AC10) ------------------------------------

/// Validates the set `documents` with the subset of the policy `policy`.
fn validate_with_policy(documents: Vec<Value>, policy: Value) -> super::Validation {
    validate(&read_input(
        json!({"documents": linked(documents), "policy": policy}),
    ))
}

/// The subset of the policy of `examples/policy.yaml`: its types are those of the default
/// policy; its scopes and areas are fewer, as the example gives them.
fn policy() -> Value {
    json!({
        "types": crate::policy::TYPES,
        "scopes": ["core", "cli", "docs", "specs", "ci"],
        "areas": ["core", "cli", "docs"]
    })
}

#[test]
fn ac1_the_invalid_epic_is_refused_for_its_bracket_tag_with_the_title_as_fix() {
    // `epic.invalid.yaml`: the epic of the examples with another identifier and a tag.
    let invalid = with(
        with(epic(), "id", json!("EPIC-0002")),
        "title",
        json!("[EPIC] Monthly export of time entries"),
    );
    let validation = validate(&read(linked(vec![entry(epic()), entry(invalid)])));
    assert_eq!(
        found(&validation.errors),
        [(codes::TITLE_FORMAT, "/title".into(), Reason::Syntax)]
    );
    let error = &validation.errors[0];
    assert_eq!(error.document(), Some(1));
    assert_eq!(error.fix(), Some(&json!("Monthly export of time entries")));
}

#[test]
fn ac4_each_broken_title_is_reported_with_its_pointer_and_fix() {
    // A prefix, a bracket tag and a full stop, on epics and tasks, each at `/title` with its
    // document, its fix and the line it was read from.
    let task_at = |n: u32, title: &str| {
        let document = with(task(), "id", json!(format!("TASK-{n:04}")));
        with(document, "title", json!(title))
    };
    let validation = validate_with_policy(
        vec![
            entry(with(epic(), "title", json!("feat(core): Record entries"))),
            entry(task_at(2, "Entries are listed [WIP]")),
            json!({
                "format": "document", "document": task_at(3, "Entries are exported."),
                "origin": {"file": "plan/items/TASK-0003.yaml"}, "lines": {"/title": 4}
            }),
        ],
        policy(),
    );
    assert_eq!(
        found(&validation.errors),
        vec![(codes::TITLE_FORMAT, "/title".to_owned(), Reason::Syntax); 3]
    );
    let documents: Vec<(Option<u64>, Option<&Value>)> = validation
        .errors
        .iter()
        .map(|e| (e.document(), e.fix()))
        .collect();
    assert_eq!(
        documents,
        [
            (Some(0), Some(&json!("Record entries"))),
            (Some(1), Some(&json!("Entries are listed"))),
            (Some(2), Some(&json!("Entries are exported"))),
        ]
    );
    assert_eq!(
        validation.errors[2].location(),
        Some(&stored_at("plan/items/TASK-0003.yaml", 4))
    );
}

#[test]
fn ac10_a_task_title_that_starts_with_a_command_name_passes() {
    let title = "wayloop init creates a project structure that validates";
    let validation =
        validate_with_policy(vec![entry(with(task(), "title", json!(title)))], policy());
    passed_with(&validation, &["title_format", "title_duplicate"]);
}

#[test]
fn ac4_titles_equal_after_normalisation_are_refused_across_epics_and_tasks() {
    // The epic and the task share a title after NFC, case folding and white space; each is
    // refused at `/title` and names the other.
    let validation = validate(&read(linked(vec![
        entry(with(
            epic(),
            "title",
            json!("Export of the cafe\u{301} entries"),
        )),
        stored(
            with(task(), "title", json!(" EXPORT of the  café entries")),
            "plan/items/TASK-0001.yaml",
            3,
        ),
    ])));
    assert_eq!(
        found(&validation.errors),
        vec![
            (
                codes::TITLE_DUPLICATE,
                "/title".to_owned(),
                Reason::Duplicate
            );
            2
        ]
    );
    let [first, second] = &validation.errors[..] else {
        panic!("{:?}", validation.errors);
    };
    assert_eq!((first.document(), second.document()), (Some(0), Some(1)));
    // The title of the task has no line of its own: its source names the file only.
    let task_file = Source::File {
        file: ProjectPath::new("plan/items/TASK-0001.yaml"),
        line: None,
    };
    assert_eq!(second.location(), Some(&task_file));
    assert_eq!(
        first.related(),
        Some(&[Related::new(1, at("/title"), Some(task_file))][..])
    );
    assert_eq!(
        second.related(),
        Some(&[Related::new(0, at("/title"), None)][..])
    );
    assert_eq!(first.fix(), None);
}

#[test]
fn a_superseded_title_is_not_compared_and_other_kinds_are_not_checked() {
    // The superseded task keeps the title of the live one; the brief and the milestone are
    // not epics or tasks, so their titles are neither checked nor compared.
    let superseded = with(
        with(titled(task(), 9), "status", json!("superseded")),
        "superseded_by",
        json!(["TASK-0001"]),
    );
    let superseded = with(
        with(superseded, "id", json!("TASK-0009")),
        "title",
        task()["title"].clone(),
    );
    let validation = validate(&read(linked(vec![
        entry(task()),
        entry(superseded),
        entry(with(brief(), "title", json!("[BRIEF] Time entries."))),
        entry(with(milestone(), "title", task()["title"].clone())),
    ])));
    assert_eq!(validation.errors, []);
}

#[test]
fn a_title_refused_by_its_schema_is_left_out_of_the_title_stage() {
    // A title that is too long is an error of the schema only: no error of form or duplicate
    // is added at the same pointer.
    let long = format!("{}.", "x".repeat(130));
    let validation = validate(&read(linked(vec![
        entry(with(task(), "title", json!(long))),
        entry(with(titled(epic(), 1), "title", json!(long))),
    ])));
    let codes: Vec<Code> = validation.errors.iter().map(|e| e.code().clone()).collect();
    assert_eq!(codes, [codes::ARTIFACT_SCHEMA, codes::ARTIFACT_SCHEMA]);
}

#[test]
fn a_duplicate_names_the_other_document_in_its_message() {
    let validation = validate(&read(linked(vec![
        entry(task()),
        entry(epic()),
        entry(task()),
    ])));
    let messages: Vec<&str> = validation.errors.iter().map(Error::message).collect();
    assert!(
        messages[0].contains("the document 2 of the input"),
        "{messages:?}"
    );
    assert!(
        messages
            .last()
            .unwrap()
            .contains("the document 0 of the input"),
        "{messages:?}"
    );
}

#[test]
fn without_a_policy_the_default_policy_gives_the_types_and_is_reported_once() {
    // R21: the prefix is a type of the default policy, and the default is listed at `/policy`
    // before the defaults of the documents, only when a check reads it.
    let markdown = json!({"format": "markdown", "text": super::markdown::tests::EXAMPLE,
                          "origin": {"file": "specs/0001-time-entries/spec.md"}});
    let validation = validate(&read(linked(vec![
        markdown.clone(),
        entry(with(epic(), "title", json!("chore: Time entries"))),
    ])));
    assert_eq!(
        found(&validation.errors),
        [(codes::TITLE_FORMAT, "/title".into(), Reason::Syntax)]
    );
    let defaults: Vec<(Option<u64>, &str, &str)> = validation
        .defaults_applied
        .iter()
        .map(|d| (d.document(), d.pointer().as_str(), d.reason().id()))
        .collect();
    assert_eq!(
        defaults,
        [
            (None, "/policy", "default_policy"),
            (Some(0), "/kind", "implied_by_form"),
            (Some(0), "/schema_version", "implied_by_form"),
        ]
    );
    // No epic or task, so not their targets either: no check reads the policy, so no default is
    // reported.
    let validation = validate(&read(vec![markdown, entry(brief())]));
    assert!(
        validation
            .defaults_applied
            .iter()
            .all(|d| d.pointer().as_str() != "/policy")
    );
    // A policy given is read instead, and no default applies.
    let validation = validate_with_policy(
        vec![entry(with(epic(), "title", json!("chore: Time entries")))],
        json!({"types": ["feat"], "scopes": ["core"], "areas": ["core"]}),
    );
    assert_eq!(validation.errors, []);
    assert_eq!(validation.defaults_applied, []);
}

#[test]
fn a_title_longer_than_the_schema_allows_is_not_read_by_the_title_stage() {
    // A document of an unknown version skips its schema, so its title is not limited there:
    // the title stage reads no title beyond the limit of the schema, and the set is checked in
    // a bounded time.
    let long = ".".repeat(100_000);
    let validation = validate(&read(linked(vec![entry(with(
        with(epic(), "schema_version", json!(9)),
        "title",
        json!(long),
    ))])));
    let codes: Vec<Code> = validation.errors.iter().map(|e| e.code().clone()).collect();
    assert_eq!(codes, [codes::SCHEMA_VERSION_UNKNOWN]);
    // At the limit, the title is read.
    let at_limit = format!(
        "{}.",
        "x".repeat(wayloop_schemas::contract::rules::TITLE_MAX_CHARS - 1)
    );
    let validation = validate(&read(linked(vec![entry(with(
        epic(),
        "title",
        json!(at_limit),
    ))])));
    let codes: Vec<Code> = validation.errors.iter().map(|e| e.code().clone()).collect();
    assert_eq!(codes, [codes::TITLE_FORMAT]);
}
