//! The valid plan items of the examples of SPEC-0001, for the tests of the crates that read
//! them. A deliberate copy of the documents of the contract case `plan-valid`: a crate does not
//! embed a file outside its folder, so the test of `wayloop-runtime`
//! `the_examples_of_the_schemas_crate_are_those_of_the_valid_case` keeps them equal.

use serde_json::{Value, json};

/// The valid gate issue of the examples.
#[must_use]
pub fn gate() -> Value {
    json!({
        "kind": "gate", "schema_version": 1, "id": "GATE-0001",
        "title": "Approve the brief on terminal time tracking", "status": "done",
        "subject": "BRIEF-0001", "issue": 2, "approved_by": "@example-owner"
    })
}

/// The valid milestone of the examples.
#[must_use]
pub fn milestone() -> Value {
    json!({
        "kind": "milestone", "schema_version": 1, "id": "M1", "title": "Time entries",
        "status": "implementing",
        "gate": "A user records, lists and exports the entries of a month offline.",
        "council_report": "COUNCIL-0001", "due": "2027-03-31", "tracker_number": 1
    })
}

/// The valid epic of the examples.
#[must_use]
pub fn epic() -> Value {
    json!({
        "kind": "epic", "schema_version": 1, "id": "EPIC-0001",
        "title": "Time entries are recorded from the terminal", "status": "implementing",
        "milestone": "M1",
        "outcome": "A user records a time entry with one command and lists the entries of a day.",
        "area": "core", "issue": 10
    })
}

/// The valid story of the examples.
#[must_use]
pub fn story() -> Value {
    json!({
        "kind": "story", "schema_version": 1, "id": "STORY-0001",
        "title": "Record a time entry with one command", "status": "implementing",
        "epic": "EPIC-0001", "milestone": "M1",
        "as_a": "freelance developer", "i_want": "to record a time entry with one command",
        "so_that": "I do not leave the terminal",
        "acceptance_criteria": [{
            "id": "AC1", "given": "an empty log",
            "when": "I run the record command with a duration and a project",
            "then": "the log holds one entry with that duration and project"
        }],
        "specification": ["SPEC-0001"], "issue": 11
    })
}

/// The valid task of the examples.
#[must_use]
pub fn task() -> Value {
    json!({
        "kind": "task", "schema_version": 1, "id": "TASK-0001",
        "title": "timesheet record appends one entry to the log", "status": "implementing",
        "epic": "EPIC-0001", "milestone": "M1", "story": "STORY-0001",
        "specification": [{"spec": "SPEC-0001", "requirements": ["R1", "R2"]}],
        "verification": ["tdd"], "area": "core", "depends_on": [], "complexity": 3,
        "parallel": false, "issue": 12
    })
}
