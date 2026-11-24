//! Valid documents of the examples of SPEC-0001, for the tests of the crates that read them: the
//! council report, the living specification and the change delta, the gate issue and the plan
//! items, the hand-over record and its input, the pull request and the commit. A deliberate copy
//! of the documents of the examples and of the contract cases `design-valid`, `handover-valid`
//! and `inputs-valid`: a crate does not embed a file outside its folder, so three tests of
//! `wayloop-runtime` keep them equal:
//! `the_examples_of_the_schemas_crate_are_those_of_the_valid_case` for the design artifacts and
//! the plan items, `the_examples_of_the_hand_over_load_into_the_documents_of_their_cases` for
//! the hand-over record and input, and
//! `the_examples_of_the_pull_request_and_the_commit_load_into_the_documents_of_their_cases` for
//! the pull request and the commit.

use serde_json::{Value, json};

/// The valid council report of the examples.
#[must_use]
pub fn council_report() -> Value {
    json!({
        "kind": "council_report", "schema_version": 1, "id": "COUNCIL-0001",
        "title": "Council report on terminal time tracking", "status": "approved",
        "brief": "BRIEF-0001", "mode": "one_mind",
        "roles": ["product_and_business", "software_architecture", "critic"],
        "decisions": [{
            "id": "D1", "statement": "Store entries in a local file, one line per entry.",
            "reason": "Works offline and stays readable without the tool."
        }],
        "rejected_alternatives": [
            {"option": "A hosted service", "reason": "Breaks the offline constraint."}
        ],
        "dissent": [{
            "role": "product_and_business",
            "position": "Synchronisation between devices matters from the first release.",
            "decision": "D1"
        }],
        "risks": [{
            "description": "Concurrent writes from two terminals.",
            "mitigation": "Append-only writes with a lock."
        }],
        "questions": [{
            "id": "Q1", "question": "Is synchronisation in the first milestone?",
            "options": ["Yes", "No"], "recommendation": "No", "answer": "No"
        }]
    })
}

/// The valid living specification of the examples, in normalised form.
#[must_use]
pub fn specification() -> Value {
    json!({
        "kind": "specification", "schema_version": 1, "id": "SPEC-0001",
        "title": "Time entries", "status": "implementing", "capability": "time-entries",
        "owner": "@example-owner", "milestone": "M1", "commit_scope": "core",
        "related": ["COUNCIL-0001"],
        "problem": "Freelancers need to record time without leaving the terminal.",
        "scope_in": ["Recording and listing time entries"],
        "scope_out": ["Export (a later specification)"],
        "stories": ["STORY-0001"],
        "user_stories": [
            "As a freelance developer, I want to see the total of a day, so that I check my timesheet."
        ],
        "requirements": [
            {
                "id": "R1", "name": "Record",
                "text": "WHEN the user runs the record command, THE TOOL SHALL append one entry to the log."
            },
            {"id": "R2", "text": "THE TOOL SHALL store durations in minutes."},
            {"id": "R3", "text": "IF the duration is not positive, THEN THE TOOL SHALL refuse the entry."},
            {
                "id": "R4", "text": "THE TOOL SHALL ask for a project when none is given.",
                "status": "removed"
            }
        ],
        "acceptance_criteria": [
            {
                "id": "AC1", "requirements": ["R1", "R2"],
                "text": "Given an empty log, when an entry of 90 minutes is recorded, then the log holds one entry of 90 minutes.",
                "verification": ["tdd"]
            },
            {
                "id": "AC2", "requirements": ["R3"],
                "text": "Given any duration of zero or less, when recorded, then it is refused.",
                "verification": ["property"]
            },
            {
                "id": "AC3", "requirements": ["R1"],
                "text": "Given the help of the record command, when read by a new user, then it explains duration and project.",
                "verification": ["judge"],
                "rubric": "Score 1 to 5; 4 or more when duration format, project and one example are all stated."
            }
        ],
        "interfaces": ["timesheet record <duration> <project>"],
        "open_questions": [
            {"id": "Q1", "text": "Is a project required for every entry?", "resolved": true}
        ],
        "extra_sections": [
            {"heading": "Notes", "text": "The log format follows COUNCIL-0001 D1."}
        ]
    })
}

/// The valid change delta of the examples.
#[must_use]
pub fn change() -> Value {
    json!({
        "kind": "change", "schema_version": 1, "id": "CHANGE-0001",
        "title": "Durations in seconds", "status": "draft", "specification": "SPEC-0001",
        "proposal": "Short tasks need a finer unit than minutes.",
        "design": "Convert existing entries once, when the log is first read.",
        "operations": [
            {
                "op": "modified", "requirement": "R2",
                "text": "THE TOOL SHALL store durations in seconds."
            },
            {
                "op": "added",
                "text": "WHEN a log in minutes is read, THE TOOL SHALL convert it to seconds."
            }
        ]
    })
}

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

/// The valid hand-over record of the examples, as the engine records it: the valid input with
/// the members the engine sets.
#[must_use]
pub fn handover() -> Value {
    let mut record = handover_input();
    let engine = json!({
        "kind": "handover", "id": "HANDOVER-12-1", "issue": 12, "task": "TASK-0001",
        "session": "session-2027-01-15-01", "holder": "agent-lane-1",
        "executor": "claude_code", "branch": "feat/12-record-time-entry",
        "recorded_at": "2027-01-15T17:42:00Z", "tokens": 157
    });
    if let (Some(record), Value::Object(engine)) = (record.as_object_mut(), engine) {
        record.extend(engine);
    }
    record
}

/// The valid hand-over input of the examples, as the agent submits it.
#[must_use]
pub fn handover_input() -> Value {
    json!({
        "schema_version": 1, "step": "build",
        "files_touched": ["src/record.rs", "tests/record.rs"],
        "not_done": ["Error message for a missing project"], "open_questions": [],
        "rulings": [{
            "decision": "Store the log as JSON Lines.",
            "reason": "Append-only writes and one entry per line, as COUNCIL-0001 D1 states.",
            "cost_if_wrong": "One migration of the log format."
        }],
        "next_step": "Make the test for AC1 of SPEC-0001 pass."
    })
}

/// The valid pull request input of the examples, as the adapter of the tracker reads it.
#[must_use]
pub fn pull_request() -> Value {
    json!({
        "schema_version": 1,
        "title": "feat(core): append one entry per record command [SPEC-0001 R1]",
        "body": "Records one entry per command.\n\nCloses #12\n",
        "branch": "feat/12-record-time-entry",
        "labels": []
    })
}

/// The valid commit input of the examples, as the adapter of git reads it: a breaking change
/// with its paragraph and its trailers.
#[must_use]
pub fn commit() -> Value {
    json!({
        "schema_version": 1,
        "header": "feat(core)!: store durations in seconds [SPEC-0001 R2]",
        "body": "Durations were minutes; short tasks need seconds.\n\n\
                 BREAKING CHANGE: logs written in minutes are converted on first read.\n\n\
                 Refs: #12\nSigned-off-by: Example Owner <owner@example.org>\n"
    })
}
