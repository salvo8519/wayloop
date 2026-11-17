# SPEC-0003: Ledger events, hash chain and replay

| | |
| --- | --- |
| Status | Draft |
| Milestone | M1 · Engine core |
| Owner | @salvo8519 |
| Related | #12, #50, #51, #52, #161, #162; ADR 0002, ADR 0003, ADR 0006, ADR 0007, ADR 0008, ADR 0009, ADR 0010, ADR 0013, ADR 0028; SPEC-0001, SPEC-0002, SPEC-0004, SPEC-0005, SPEC-0006, SPEC-0007, SPEC-0008, SPEC-0010, SPEC-0020, SPEC-0021 |
| Commit scope | ledger |

## Problem
Agent sessions end, crash and hand over to each other, often in parallel worktrees of one
clone. The state of the loop must survive them, be the same whoever reads it and on every
platform, and its history must show any edit. An agent that arrives after weeks must resume
from a summary of known size, not from months of events. A maintainer must be able to audit
every decision taken by agents, with its reason and its cost.

## Scope
**In scope:**
- The event: common fields, the catalogue of kinds by role, the payloads owned by this specification, schema versions and migration.
- The hash chain: canonical JSON (RFC 8785), SHA-256, the genesis event, the line format, detection and report of a broken chain, recovery.
- The writer: one writer (ADR 0006), the lock between processes and worktrees, durability, expected revision, torn writes.
- Storage under the git common directory: segments, YAML projection, snapshots, summaries, hand-overs, quarantine, and the SQLite index as a rebuildable cache reached through the CLI and the MCP server (ADR 0007).
- Replay and its test, snapshots and replay equivalence.
- Clock times: where they are recorded and where they are never placed (ADR 0028).
- Checkout of an issue with a lease, hand-over and resumption.
- Pull request events, rulings and cost records.
- Session, issue and milestone summaries: content, deterministic roll-up, size and token budgets, publication on the tracker and reading back (ADR 0003).

**Out of scope:**
- The transitions, guards and the state fields that the state machine derives from events (SPEC-0002); this specification names the events it consumes by role.
- The payload schemas of event kinds owned by other specifications (section Event catalogue).
- The command set and the envelope (SPEC-0004), apart from the rules on revisions and clock times that the ledger imposes.
- The tracker calls that publish a summary (SPEC-0005) and the content of gates and questions (SPEC-0007).
- The code index of the survey (SPEC-0010); this specification fixes only the rules of the ledger index.
- The detection of secrets (SPEC-0020 R4), which the writer calls.
- Merging the ledgers of two clones. Each clone has its own ledger; the durable memory shared between machines is the set of published summaries (ADR 0003).

## User stories
- As an agent arriving after weeks, I want a summary of known size, so that I resume without reading months of history.
- As a maintainer, I want to see every decision taken by agents with its reason, so that I can audit and undo it.
- As a maintainer, I want an edited or truncated ledger to be detected and located, so that I never build on a corrupted state.
- As an agent in one worktree, I want my checkout of an issue to stop another agent from taking it, so that two agents never work on the same issue.
- As an agent of any vendor, I want to start from the hand-over of the previous session, whoever wrote it, so that work continues without loss.
- As a maintainer who loses a machine, I want the summaries on the issues to carry the memory of the project, so that a new clone resumes from them.

## Requirements (EARS)
- **R1**: THE ENGINE SHALL append every event to the ledger as one JSON Lines line and never rewrite, reorder or delete an acknowledged line, except through the recovery of R34.
- **R2**: THE ENGINE SHALL store in every event, as `prev_hash`, the `hash` of the previous event of the ledger, and in the genesis event 64 zero digits.
- **R3**: WHEN the ledger is replayed from the genesis event, THE ENGINE SHALL produce a projection byte-identical to the stored projection, on every supported platform.
- **R4**: IF the hash chain is broken, THEN THE ENGINE SHALL refuse every append and report the first broken event with its segment file, line, sequence number, pointer, expected and found values, and the revision of the last valid event.
- **R5**: THE ENGINE SHALL record every ruling as an event with a decision, a reason and the cost if wrong, and refuse a ruling that lacks any of them.
- **R6**: THE ENGINE SHALL produce session, issue and milestone summaries as JSON validated by the summary schema, each within the token budget of its level.
- **R7**: THE ENGINE SHALL write snapshots of the state and, WHEN it loads the ledger, replay from the latest valid snapshot plus the events that follow it.
- **R8**: WHEN the engine reads an event, a snapshot or a summary of an earlier known schema version, THE ENGINE SHALL verify the hash of the stored bytes first, then migrate the document in memory to the current version, and never rewrite a stored event.
- **R9**: THE ENGINE SHALL record pull request events: opened, checks completed with the sorted names of the failing checks, review submitted, merged, and closed without merge, each with its pull request number, issue, branch or head commit as listed in section Payloads.
- **R10**: THE ENGINE SHALL record a checkout of an issue with its holder, session, branch, worktree name and lease duration, and refuse a second checkout of the issue by another holder while the lease holds, with the reason `conflict`, the holder and the expiry.
- **R11**: WHEN a lease expires without activity, THE ENGINE SHALL record the expiry and release the issue.
- **R12**: WHEN the holder of a checkout ends a session, THE ENGINE SHALL record a hand-over (SPEC-0001, Hand-over record) and refuse one whose size exceeds the hand-over budget, with the reason `length`, the measured size and the budget.
- **R13**: WHEN an agent checks out an issue that has a hand-over, THE ENGINE SHALL start its brief from the resume pack of R44, whatever agent or executor wrote the hand-over.
- **R14**: THE ENGINE SHALL start every ledger with one genesis event of kind `ledger_created` at sequence 0, and refuse a ledger whose first event is not a genesis event or that holds a second one.
- **R15**: THE ENGINE SHALL compute the `hash` of an event as the SHA-256, in lower-case hexadecimal, of the UTF-8 bytes of the event without its `hash` member, serialised as canonical JSON (RFC 8785), and refuse an event whose stored `hash` differs.
- **R16**: THE ENGINE SHALL write every event as its canonical JSON followed by one line feed, at most the configured line limit in bytes, and refuse on read a line that is not canonical JSON or that exceeds the limit, with the segment file and line.
- **R17**: THE ENGINE SHALL number events with `seq` from 0, each one more than the previous, and report a missing, repeated or out-of-order number with the reason `range`.
- **R18**: THE ENGINE SHALL append through one writer that holds an exclusive operating-system lock on the ledger for each append, writes the line, flushes it to stable storage, and only then acknowledges the event; the lock is released when the process ends.
- **R19**: WHEN a command that changes state carries an expected revision, THE ENGINE SHALL compare it with the hash of the last event under the lock and refuse a different one with the reason `stale`, appending nothing.
- **R20**: IF the last line of the last segment has no final line feed, THEN THE ENGINE SHALL move its bytes to the quarantine, record a `torn_tail_discarded` event with their size and hash, and report a warning.
- **R21**: IF the output of an agent or an executor does not validate against the schema of its step, THEN THE ENGINE SHALL record a `result_rejected` event with the step, the error codes and pointers, and a reference to the stored output by hash, and leave every other part of the state unchanged.
- **R22**: IF an event has a kind or a schema version the engine does not know, THEN THE ENGINE SHALL stop loading, report the line with the reason `enum` and the known values in `expected`, and refuse every append.
- **R23**: THE ENGINE SHALL read the format version, the hash algorithm and the canonicalisation of a ledger from its genesis event, and refuse a ledger whose values it does not know with the reason `enum`.
- **R24**: THE ENGINE SHALL keep the ledger and its derived files under `<git common dir>/wayloop/` with the layout of section Storage, so that every worktree of a clone reads and writes the same ledger.
- **R25**: WHEN an event is appended, THE ENGINE SHALL rewrite the projection `state.yaml` deterministically from the state; IF the stored projection differs from the projection of the state, THEN THE ENGINE SHALL report a warning and rewrite it.
- **R26**: THE ENGINE SHALL treat the SQLite index as a cache built from the ledger and the summaries, bring it to the current revision before answering a query, rebuild it when its revision is not in the chain, and never read it to decide a transition.
- **R27**: THE ENGINE SHALL set `recorded_at` of every event, in UTC to the second with the suffix `Z`, from a time given to the writer as an input, and use the `recorded_at` of the previous event when the given time is earlier.
- **R28**: THE ENGINE SHALL take the current time only as the explicit input of the function that returns the events due by time, and compute every command result from recorded events, so that no envelope holds the time of the call.
- **R29**: THE ENGINE SHALL refuse an event that holds a number that is not an integer between -(2^53 - 1) and 2^53 - 1, with the reason `type`.
- **R30**: WHEN an event observed on the tracker carries an `external_id` already recorded for the same kind, THE ENGINE SHALL append nothing and report the duplicate as a warning.
- **R31**: WHEN the engine takes a snapshot, THE ENGINE SHALL record a `snapshot_taken` event with the sequence number and the SHA-256 of the snapshot, start a new segment after it, and ignore with a warning a snapshot whose hash differs from its event.
- **R32**: WHEN the engine loads the ledger, THE ENGINE SHALL verify every event after the latest valid snapshot and its link to that snapshot; WHEN `wayloop ledger verify --full` runs, THE ENGINE SHALL verify every segment from the genesis event.
- **R33**: WHILE the chain is broken, THE ENGINE SHALL answer commands that read state from the valid prefix, with a warning that names the first broken event.
- **R34**: WHEN a person runs the recovery of a broken ledger, THE ENGINE SHALL move every line from the first broken event onward to the quarantine and append a `ledger_recovered` event linked to the last valid event; IF the actor is not a person or the chain is not broken, THEN THE ENGINE SHALL refuse it with the reason `state`.
- **R35**: THE ENGINE SHALL fill a summary with the fill rule of section Roll-up, so that the result is a function of the events alone, fits its budget, and counts every dropped entry in `omitted`.
- **R36**: THE ENGINE SHALL compute the session summary when a session ends, the issue summary when a session of the issue ends and when the issue merges or closes, and the milestone summary when an issue of the milestone merges or closes, and record each with a `summary_recorded` event.
- **R37**: THE ENGINE SHALL read the budgets from `[ledger.budgets]` of `wayloop.toml` with the defaults of section Budgets, and refuse a configuration in which a budget is below 256 tokens, the session or the issue budget is below the hand-over budget plus 256, or the milestone budget is below the issue budget, with the reason `range`.
- **R38**: WHEN a summary or a hand-over is to be published, THE ENGINE SHALL render it deterministically with a marker that names its level, subject and SHA-256 and with its canonical JSON, publish it through the tracker adapter, skip it when a comment with the same marker and hash exists, and record a `summary_published` event after the tracker confirms it.
- **R39**: WHEN the engine reads a published summary, THE ENGINE SHALL validate its JSON against the summary schema and its hash against the marker, and treat a summary that fails either check as not published, with a warning.
- **R40**: THE ENGINE SHALL pass every payload through the secret check of SPEC-0020 R4 before it is appended and every rendered summary before it is published, and refuse what fails, appending and publishing nothing.
- **R41**: THE ENGINE SHALL write every summary file as its canonical JSON followed by one line feed, and IF a stored summary file differs from the hash of its `summary_recorded` event, THEN THE ENGINE SHALL report a warning and write it again from the events.
- **R42**: WHEN a person revokes a ruling, THE ENGINE SHALL record a `ruling_revoked` event that names the hash of the ruling event and a reason, keep the ruling in the history, and leave it out of the active rulings and of later summaries.
- **R43**: WHEN a lease expires on a session that recorded no hand-over, THE ENGINE SHALL end the session with the outcome `abandoned` and compute its session summary from its events.
- **R44**: THE ENGINE SHALL build the resume pack of an issue from the milestone summary, the issue summary and the latest hand-over of the issue, in that order, within the sum of their budgets.
- **R45**: THE ENGINE SHALL record every executor call with role, executor, model, effort, tokens in, tokens out, cost in integer micro-units of a currency and duration in milliseconds, and refuse a record that lacks one of them.
- **R46**: WHILE a checkout holds, THE ENGINE SHALL extend its lease with every event of the session of the holder on that issue, including a `lease_renewed` event, and WHEN the holder ends its session with a hand-over or releases the checkout, or its pull request merges or closes, THE ENGINE SHALL record a `checkout_released` event with the reason.
- **R47**: THE ENGINE SHALL assign every session the identifier `session-<YYYY-MM-DD>-<NN>`, where the date is the UTC date of its `session_started` event and `NN` is the 1-based count of sessions started on that date in the ledger, at least two digits.
- **R48**: IF a hand-over, a renewal or a session end for an issue comes from an actor that does not hold its checkout, THEN THE ENGINE SHALL refuse it with the reason `state` and name the holder.

## Acceptance criteria
- **AC1** (R3): Given a generated ledger of 10,000 valid random events, when it is replayed twice, then both projections and both revisions are byte-identical. Verification: property test.
- **AC2** (R4, R15): Given `examples/ledger.edited-line.invalid.jsonl`, when it is loaded, then the error has the code `ledger_hash_mismatch`, `source` names the file and line 5, the pointer is `/hash`, and the revision of the last valid event is the hash of line 4. Verification: TDD, with the files of `examples/` as fixtures.
- **AC3** (R6, R35): Given a milestone of 40 issues with 200 sessions, each hand-over at the budget, when the milestone summary is rolled up, then it validates, its `tokens` is at most the milestone budget, and the sum of the kept and the `omitted` entries of each section equals the entries of the issues. Verification: TDD.
- **AC4** (R9, R6): Given a merged pull request, when the issue summary is rolled up, then it has the outcome `merged` and the pull request entry carries the merge commit and `merged_at`. Verification: TDD, with `examples/summary.issue.valid.json`.
- **AC5** (R11, R28): Given a checkout whose last activity plus its lease is earlier than the time given to the function of due events, when the runtime computes the next step, then a `lease_expired` event is recorded first and the issue is ready. Verification: property test.
- **AC6** (R13, R44): Given a hand-over written in a session of one executor, when an agent of another executor checks out the issue, then its brief starts with the resume pack and the pack names that hand-over. Verification: TDD.
- **AC7** (R2, R14, R15, R16, R17): Given `examples/ledger.valid.jsonl`, when it is verified, then it passes, every line is canonical, every `prev_hash` equals the `hash` of the previous line, and the revision is `43f8645e6723416a8d2edd8a127ccc64905da44c97c943ca8e61f9b4cb5a0f14`. Verification: contract test.
- **AC8** (R15): Given any event object and any permutation of the order of its members, when hashed, then the hash is the same and equals the SHA-256 of the canonical bytes without `hash`. Verification: property test.
- **AC9** (R16): Given `examples/ledger.non-canonical.invalid.jsonl`, when it is loaded, then the error has the code `ledger_line_malformed`, the reason `syntax` and `source` with line 3; given a line one byte above the line limit, then the reason is `length`. Verification: TDD.
- **AC10** (R17, R4): Given `examples/ledger.sequence-gap.invalid.jsonl`, when it is loaded, then the first error has the code `ledger_sequence_gap`, the reason `range`, `source` with line 7 and `expected` holding 6. Verification: TDD.
- **AC11** (R2, R4): Given `examples/ledger.rehashed-line.invalid.jsonl`, when it is loaded, then the error has the code `ledger_chain_broken`, the pointer `/prev_hash` of line 6, and `pointers` naming line 5. Verification: TDD.
- **AC12** (R14, R23): Given `wayloop init` on a new clone, when the ledger is created, then line 1 is `ledger_created` with `seq` 0, `prev_hash` of 64 zeros and `format_version` 1; given a ledger whose first event is another kind, or that holds a second genesis event, then it is refused with the code `ledger_genesis`; given a genesis event with `hash_algorithm` `sha512`, then it is refused with the code `ledger_format_unknown` and the reason `enum`. Verification: TDD.
- **AC13** (R18, R24): Given two processes in two worktrees of one clone, each appending 1,000 events at the same time, when both finish, then the ledger holds 2,001 events in one valid chain and every acknowledged event is present once. Verification: end-to-end.
- **AC14** (R19): Given an expected revision that is not the hash of the last event, when a command that changes state runs, then it is refused with the reason `stale` and the ledger is unchanged. Verification: TDD.
- **AC15** (R20): Given `examples/ledger.torn-tail.invalid.jsonl`, when it is loaded, then the partial line is moved to the quarantine, a `torn_tail_discarded` event follows line 22, the chain is valid and a warning `ledger_torn_tail` is reported; given a malformed line that is not the last one, then nothing is moved and the error of R16 is reported. Verification: TDD.
- **AC16** (R21): Given an executor result that misses a required field, when it is returned, then a `result_rejected` event is recorded with the error pointer and the hash of the stored output, and the projection differs from the previous one only in the list of rejected results. Verification: TDD.
- **AC17** (R22): Given an event of kind `future_kind` or of `schema_version` 99, when the ledger is loaded, then loading stops with the codes `event_kind_unknown` or `event_version_unknown`, the reason `enum` and the known values in `expected`, and an append is refused. Verification: TDD.
- **AC18** (R8): Given a ledger fixture with events of schema version 1 of a kind whose current version is 2, when it is loaded, then every event is migrated in memory, the hash is checked on the stored bytes, the files are unchanged byte for byte, and the projection equals that of the same ledger written in version 2. Verification: TDD, with one fixture per released version.
- **AC19** (R8, R39): Given a published issue summary of schema version 1 while the current version is 2, when it is read back, then it is migrated and validates against version 2. Verification: TDD.
- **AC20** (R24): Given the main worktree and a second worktree of one clone, when `wayloop status` runs in each, then both report the same `meta.revision`. Verification: end-to-end.
- **AC21** (R3, R25): Given the golden ledger of the repository, when it is replayed in CI on Linux, macOS and Windows, then the three projections are byte-identical to the golden `state.yaml`. Verification: end-to-end.
- **AC22** (R25): Given a `state.yaml` edited by hand, when the next event is appended, then a warning `projection_mismatch` is reported and the file equals the projection of the state. Verification: TDD.
- **AC23** (R26): Given a deleted index, when `wayloop history` runs, then the index is rebuilt and the result equals the result before deletion; given an index built at a revision that is not in the chain, then it is rebuilt; given any content of the index, then the next step is the same. Verification: property test.
- **AC24** (R27): Given a writer clock that moves one hour backwards between two appends, when the second event is recorded, then its `recorded_at` equals that of the first; given any appended event, then `recorded_at` matches `YYYY-MM-DDTHH:MM:SSZ`. Verification: TDD, with a fake clock.
- **AC25** (R28): Given one ledger, when `wayloop status` and `wayloop history` run at two different clock times with the same input, then the two envelopes are byte-identical. Verification: contract test.
- **AC26** (R29): Given a payload holding `1.5` or 2^53, when it is appended, then it is refused with the code `event_schema` and the reason `type` and the pointer of the number. Verification: TDD.
- **AC27** (R30): Given the merge of pull request 31 observed twice by the tracker adapter, when both observations are processed, then one `pull_request_merged` event exists and the second gives the warning `event_duplicate`. Verification: TDD.
- **AC28** (R7, R31): Given a ledger of random events and a snapshot taken at a random sequence, when the state is loaded from the snapshot plus the tail, then it is byte-identical to the state of a full replay. Verification: property test.
- **AC29** (R31, R7): Given a snapshot file whose content was changed, when the ledger is loaded, then a warning `snapshot_mismatch` names it and the state is loaded from the previous valid snapshot or from the genesis event. Verification: TDD.
- **AC30** (R32): Given an edited line before the latest snapshot, when the ledger is loaded, then no error is reported; when `wayloop ledger verify --full` runs, then the error names the line. Verification: TDD.
- **AC31** (R33): Given a ledger broken at line 6, when `wayloop history` runs, then it lists events 0 to 4 with a warning that names line 6, and `meta.revision` is the hash of line 5. Verification: TDD.
- **AC32** (R34): Given a ledger broken at line 6, when a person runs the recovery, then lines 6 onward are in the quarantine, a `ledger_recovered` event follows line 5 with `prev_hash` equal to its hash, and the chain is valid; given the same command from an agent, or on a valid ledger, then it is refused with the reason `state`. Verification: TDD.
- **AC33** (R35): Given any set of events and any valid budget, when a summary is rolled up twice, then both results are byte-identical, `tokens` is at most the budget, and for each section kept plus `omitted` equals its source entries. Verification: property test.
- **AC34** (R36): Given a session that ends with a hand-over, when it ends, then a session summary and an issue summary are recorded; given the merge of the issue, then the issue summary and the milestone summary are recorded again. Verification: TDD.
- **AC35** (R37): Given a hand-over budget of 1,200 and a session budget of 1,300, when the configuration is read, then it is refused with the code `ledger_budget_config`, the reason `range` and the pointer `/ledger/budgets/session`. Verification: TDD.
- **AC36** (R38): Given a fake tracker, when one issue summary is published twice with the same hash, then one comment exists and one `summary_published` event is recorded; when it is published with a new hash, then the comment is replaced; given a dry run, then no comment is written and the planned publication is listed. Verification: TDD.
- **AC37** (R39): Given a published comment whose JSON was edited, when it is read back, then the warning `publication_mismatch` is reported and the summary is published again at the next publication step. Verification: TDD.
- **AC38** (R40): Given a hand-over whose `next_step` holds a token that the secret check of SPEC-0020 detects, when it is submitted, then it is refused and the ledger and the tracker are unchanged. Verification: TDD.
- **AC39** (R41): Given a summary file whose content was changed, when the summaries are read, then a warning `summary_mismatch` is reported and the file equals again the one of `examples/` for the same events. Verification: TDD.
- **AC40** (R5, R42): Given a ruling without `cost_if_wrong`, when it is submitted, then it is refused with the reason `missing`; given a revoked ruling, when `wayloop rulings` runs, then it is absent from the active rulings and present in the history with its revocation. Verification: TDD.
- **AC41** (R43): Given a session whose lease expires without a hand-over, when the due events are computed, then `lease_expired` and `session_ended` with the outcome `abandoned` are recorded and the session summary has no `next_step`. Verification: TDD.
- **AC42** (R44): Given a milestone, an issue and a hand-over at their budgets, when the resume pack is built, then it holds the three in the order milestone, issue, hand-over and its size is at most the sum of the three budgets. Verification: TDD.
- **AC43** (R45): Given an executor call record without `cost_micros`, when it is submitted, then it is refused with the reason `missing`; given the fixture ledger, when `wayloop cost` runs, then the totals equal the sum of the records. Verification: TDD.
- **AC44** (R10, R46): Given issue 12 checked out by `agent-lane-1`, when `agent-lane-2` checks it out, then it is refused with the code `checkout_held`, the reason `conflict`, the holder and the expiry; when `agent-lane-1` checks it out again, then the lease is extended and no second checkout is recorded; when its pull request merges, then `checkout_released` with the reason `merged` is recorded. Verification: TDD.
- **AC45** (R12, R48): Given a hand-over of 1,300 tokens with a budget of 1,200, when it is submitted, then it is refused with the code `handover_budget`, the reason `length`, 1,300 and 1,200; given a hand-over from an actor that does not hold the checkout, then it is refused with the code `checkout_not_held` and the reason `state`. Verification: TDD.
- **AC46** (R9): Given each pull request event of `examples/ledger.valid.jsonl`, when validated against its payload schema, then it passes; given a checks event whose `failing_checks` is not sorted, then it is refused with the reason `syntax`. Verification: contract test.
- **AC47** (R47): Given two sessions started on 2027-01-15 and one on 2027-01-16, when they are recorded, then their identifiers are `session-2027-01-15-01`, `session-2027-01-15-02` and `session-2027-01-16-01`. Verification: TDD.
- **AC48** (R1, R18): Given an append interrupted at every byte offset of the line, when the ledger is loaded again, then every acknowledged event is present unchanged and only the unacknowledged line is quarantined. Verification: property test.

## Event model

### Common fields

Every event has the fields below, whatever its kind. Keys are `snake_case`, and enum values are
`snake_case` (ADR 0009 C4). Absent means not given (ADR 0009 C2): an optional field is left out,
never set to `null`.

| Name | Type | Required | Meaning |
| --- | --- | --- | --- |
| `seq` | integer, from 0 | yes | Position in the ledger (R17) |
| `kind` | enum, section Event catalogue | yes | Kind of the event; selects the payload schema |
| `schema_version` | integer, at least 1 | yes | Version of the schema of this kind (R8, R22) |
| `recorded_at` | string `YYYY-MM-DDTHH:MM:SSZ` | yes | When the writer recorded it, in UTC (R27) |
| `actor` | object `{kind, id}`; `kind` is `engine`, `agent`, `person` or `tracker` | yes | Who caused the event: the engine for its own decisions, the agent holder for validated agent input, a tracker handle for an approval or an answer, the tracker for an observation |
| `session` | string, R47 | no | Session in which the event occurred |
| `issue` | integer | no | Tracker number of the issue concerned |
| `input_hash` | string, 64 lower-case hexadecimal digits | no | `meta.input_hash` of the command that caused the event (SPEC-0004 R11) |
| `external_id` | string, 1 to 200 characters | for observations of the tracker | Identity of the observed fact on the tracker, used to refuse duplicates (R30) |
| `payload` | object | yes, may be `{}` | Fields of the kind |
| `prev_hash` | string, 64 lower-case hexadecimal digits | yes | Hash of the previous event; 64 zeros for the genesis event (R2) |
| `hash` | string, 64 lower-case hexadecimal digits | yes | Hash of this event (R15) |

Every number is an integer within the exact range of a double (R29). Amounts of money are
integer micro-units (`cost_micros`) with an ISO 4217 `currency`; durations are integer
milliseconds (`duration_ms`). Floating-point numbers would make canonical JSON depend on the
formatting of the platform.

Content larger than an event, such as a rejected executor output or a test log, is stored once
under `blobs/<sha256>` and referenced as `{file, sha256}`; the event never inlines it (R16).

### Event catalogue

Kinds are named by role. This specification owns the payload schemas of the kinds marked
SPEC-0003; the other kinds are reserved here and their payloads are specified by the owner.
Adding a kind is an additive change of the catalogue; an engine that meets a kind it does not
know stops loading (R22), because the state would otherwise differ from that of a newer engine.

| Kind | Role | Payload owner | Consumed by |
| --- | --- | --- | --- |
| `ledger_created` | Genesis of the ledger (R14, R23) | SPEC-0003 | Loader |
| `session_started`, `session_ended` | Session lifecycle (R43, R47) | SPEC-0003 | SPEC-0002, roll-up |
| `checkout_acquired`, `lease_renewed`, `lease_expired`, `checkout_released` | Checkout and lease (R10, R11, R46) | SPEC-0003 | SPEC-0002: issue claimed, issue ready again |
| `handover_recorded` | Hand-over (R12) | SPEC-0003, record of SPEC-0001 | Resume pack, roll-up |
| `ruling_recorded`, `ruling_revoked` | Rulings (R5, R42) | SPEC-0003 | Roll-up, `wayloop rulings` |
| `pull_request_opened`, `pull_request_checks_completed`, `pull_request_review_submitted`, `pull_request_merged`, `pull_request_closed` | Pull request observations (R9) | SPEC-0003 | SPEC-0002: review, merge and close transitions; SPEC-0007 reconciliation |
| `executor_call_recorded` | Cost of a step (R45) | SPEC-0008, with the fields of R45 | SPEC-0004 `wayloop cost`, SPEC-0006 budgets |
| `result_rejected` | Model output refused by its schema (R21) | SPEC-0003 | SPEC-0002 repeated failures |
| `summary_recorded`, `summary_published` | Summaries (R36, R38) | SPEC-0003 | Publication, resume pack |
| `snapshot_taken` | Snapshot anchor (R31) | SPEC-0003 | Loader |
| `torn_tail_discarded`, `ledger_recovered` | Integrity repair (R20, R34) | SPEC-0003 | Loader, audit |
| `step_completed`, `step_failed`, `issue_blocked` | Issue cycle steps and stops | SPEC-0002 | SPEC-0002 |
| `approval_recorded` | Approval by a person (SPEC-0001 R17) | SPEC-0007 | SPEC-0002 approval states |
| `question_opened`, `question_answered`, `answer_ignored` | Questions (SPEC-0007 R1, R2) | SPEC-0007 | SPEC-0002 waiting states |
| `plan_change_proposed`, `plan_change_approved` | Reconciliation (SPEC-0007 R3, R4) | SPEC-0007 | Roadmap |
| `attempt_discarded`, `budget_exhausted` | Runtime (SPEC-0006 R6, R8) | SPEC-0006 | SPEC-0002 stops |
| `policy_evaluated`, `waiver_used` | Process policy (SPEC-0021 R7) | SPEC-0021 | SPEC-0021 R8 |

### Payloads

Payloads owned by this specification, version 1. Every payload is a flat object; lists are
written `[]` when empty.

| Kind | Fields |
| --- | --- |
| `ledger_created` | `ledger_id` (string, UUID chosen at creation), `format_version` (1), `hash_algorithm` (`sha256`), `canonicalization` (`rfc8785`), `previous_ledger` (optional `{ledger_id, last_hash}`, set when a ledger replaces an earlier one) |
| `session_started` | `holder`, `executor` (snake_case name, as in the hand-over record), `mode` (`in_session` or `unattended`) |
| `session_ended` | `outcome` (`handed_over`, `completed` or `abandoned`), `handover` (identifier, when `handed_over`) |
| `checkout_acquired` | `holder`, `branch` (valid under SPEC-0001 R11), `worktree` (name of the git worktree, never a path), `task` (optional), `milestone` (optional), `lease_minutes`, `resumed_from` (optional hand-over identifier) |
| `lease_renewed` | none |
| `lease_expired` | `holder`, `expired_at` (the instant the lease ended, computed as in section Clock times) |
| `checkout_released` | `holder`, `reason` (`handed_over`, `merged`, `closed`, `released_by_holder`); an expiry releases the checkout through `lease_expired` alone |
| `handover_recorded` | `record`: the hand-over record of SPEC-0001, whose `recorded_at` equals that of the event |
| `ruling_recorded` | `decision`, `reason`, `cost_if_wrong`, `question` (optional, the question it answers) |
| `ruling_revoked` | `ruling` (hash of the `ruling_recorded` event), `reason` |
| `pull_request_opened` | `number`, `branch`, `head_sha`, `title` |
| `pull_request_checks_completed` | `number`, `head_sha`, `conclusion` (`success`, `failure`, `cancelled`, `timed_out`), `failing_checks` (names, sorted by code point, without duplicates) |
| `pull_request_review_submitted` | `number`, `head_sha`, `reviewer`, `state` (`approved`, `changes_requested`, `commented`) |
| `pull_request_merged` | `number`, `branch`, `head_sha`, `merge_commit`, `merged_at` (tracker time, UTC), `merged_by` |
| `pull_request_closed` | `number`, `branch`, `head_sha`, `closed_at` (tracker time, UTC) |
| `result_rejected` | `step`, `errors` (at most 20 objects `{code, pointer, reason}`, then `errors_omitted`), `output` (`{file, sha256}`) |
| `summary_recorded` | `level`, the subject field of the level (`session`, `issue` or `milestone`), `file`, `sha256`, `tokens` |
| `summary_published` | `level`, the subject field, `sha256`, `target` (`issue_comment` or `milestone_description`), `comment` (tracker identifier, optional) |
| `snapshot_taken` | `snapshot_seq`, `sha256`, `state_schema_version`, `file` |
| `torn_tail_discarded` | `bytes`, `sha256`, `file` (in the quarantine) |
| `ledger_recovered` | `broken_seq`, `last_valid_hash`, `file` (in the quarantine), `reason` |

The event header carries the issue (`issue`) for every kind that concerns one. The pull request
events link their issue through the header `issue` and their branch: R9 requires both, and the
writer refuses a pull request event whose branch names another issue (SPEC-0001 R10).

### Schema versions and migration

- Each kind has its own schema snapshot `schemas/events/<kind>/v<N>.json`, which includes the
  common fields. A change of the common fields is a new version of every kind.
- Released snapshots are immutable (SPEC-0001 AC3). Every change is a new version.
- Migration is a pure function from version N to N+1 of one kind, applied in memory on read,
  after the hash of the stored line is checked (R8). Stored events are never rewritten: the hash
  chain covers the bytes as written.
- Snapshots and summaries are migrated the same way. A snapshot that cannot be migrated is
  ignored, and the state is replayed from an earlier snapshot or from the genesis event.
- A change of the hashing rule, of the canonicalisation or of the line format is not a schema
  migration: it is a new `format_version` in the genesis event of a new ledger, which names the
  earlier ledger and its last hash in `previous_ledger` (R23).

## Hashing rule

1. Take the event as a JSON object and remove the member `hash`.
2. Serialise it as canonical JSON (RFC 8785): members sorted by the UTF-16 code units of their
   names, no insignificant white space, strings in the minimal escaping of the standard,
   integers without exponent or fraction.
3. Compute the SHA-256 of the UTF-8 bytes and write it in lower-case hexadecimal as `hash`.
4. Write the line as the canonical JSON of the whole event, `hash` included, followed by one
   line feed (`0x0A`). The stored line is therefore canonical, and every reader computes the
   same bytes on every platform.

Verification of a line, in this order, stopping at the first line that fails and reporting
every error of that line: it ends with a line feed (R20 if it is the last line); it parses and
equals its canonical form (R16); its size is within the line limit (R16); its `hash` is correct
(R15); its `seq` is one more than the previous (R17); its `prev_hash` equals the `hash` of the
previous line (R2); it validates against the schema of its kind and version (R22, R8).

The revision of SPEC-0004 R12 and ADR 0009 C6 is the `hash` of the last event. The SHA-256 of a
summary, a snapshot or a stored output is computed on its canonical JSON, without the final
line feed of its file.

## Storage

```text
<git common dir>/wayloop/
  ledger/<seq of the first event, 12 digits>.jsonl   segments of the ledger
  ledger.lock                                         lock of the writer (R18)
  state.yaml                                          projection (R25)
  snapshots/<seq, 12 digits>.json                     snapshots (R31)
  summaries/session/<session>.json                    summaries (R41)
  summaries/issue/<issue>.json
  summaries/milestone/<milestone>.json
  handovers/HANDOVER-<issue>-<k>.json                 hand-over records, written from their events
  blobs/<sha256>                                      large content referenced by events
  quarantine/                                         torn tails and broken lines (R20, R34)
  index.sqlite                                        index, a cache (R26)
```

- The git common directory is shared by every worktree of a clone (ADR 0007, ADR 0013), so the
  orchestrator and every lane read one ledger. It is never committed.
- The ledger, the summaries and the hand-overs are the source of truth. The projection, the
  snapshots, the summary files and the index are derived and can be rebuilt from the ledger;
  the summary files are checked against their events (R41).
- A segment is closed by a `snapshot_taken` event, which is its last line; the next event opens
  a new segment. Closed segments are never written again. Their retention is Q1.
- Paths in events and errors are relative to the git common directory, such as
  `wayloop/ledger/000000000000.jsonl`; an absolute path never enters the ledger or an envelope.
- The projection is the YAML rendering of the canonical JSON of the state: members in canonical
  order, block style, two-space indentation, every string double-quoted, line feeds, one final
  line feed. Its first members are `revision` and `seq`.
- A snapshot file is `{schema_version, seq, revision, state}` in canonical JSON. Its `revision`
  equals the `prev_hash` of the `snapshot_taken` event that anchors it.

## One writer and concurrency

- Decisions live in the core (ADR 0001, ADR 0006): it validates the input, assigns `seq`,
  `recorded_at` and the hashes, decides to append or refuse, folds the event into the state and
  returns the bytes to write. The runtime performs the I/O: it reads the clock, holds the lock,
  writes, flushes and calls the tracker.
- Inside one process, every worker sends its result to one writer task, which appends in order
  (SPEC-0006 R5).
- Between processes, such as two `wayloop` commands in two worktrees, the MCP server and an
  unattended run, the writer takes an exclusive advisory lock of the operating system on
  `ledger.lock` for each append: read the tail, check the expected revision (R19), append,
  flush, release. The operating system releases the lock of a process that dies, so a crash
  never leaves a stale lock. A writer that waits longer than the lock timeout fails with the
  code `ledger_locked` (exit code 3, ADR 0009 C7).
- The lock serialises appends; the expected revision detects a decision taken on an older
  state; the checkout lease stops two agents from working on one issue. Network file systems
  are not supported for the git common directory.
- An event is acknowledged only after its line is flushed to stable storage. An effect on the
  tracker happens before the event that records it, and is idempotent: a crash between the two
  repeats the effect, which the marker of R38 or the `external_id` of R30 makes harmless.

## Replay and snapshots

- The state is a fold of the events from the genesis event with the pure transition function of
  the core. Replay never reads a clock, a file other than the ledger, or the tracker.
- The replay test of this repository is the golden ledger and its golden projection, replayed
  in CI on Linux, macOS and Windows (AC21), plus the property tests AC1, AC28 and AC33. The
  gate of milestone M1 is AC21.
- A snapshot is taken after every `snapshot_every` events since the previous one, and on
  `wayloop ledger snapshot`. Loading uses the latest snapshot whose hash matches its event
  (R31), verifies the events after it (R32), and folds them.
- Replay equivalence: the state loaded from any valid snapshot plus its tail is byte-identical
  to the state of a full replay (AC28).

## Integrity: detection, report and recovery

| Condition | Code | Reason | Effect |
| --- | --- | --- | --- |
| Partial last line | `ledger_torn_tail` (warning) | `syntax` | Quarantined and recorded (R20); never acknowledged, so nothing is lost |
| Line not canonical or not JSON | `ledger_line_malformed` | `syntax` | Broken chain |
| Line over the limit | `ledger_line_malformed` | `length` | Broken chain |
| Stored `hash` wrong | `ledger_hash_mismatch` | `conflict` | Broken chain |
| `seq` not one more than the previous | `ledger_sequence_gap` | `range` | Broken chain |
| `prev_hash` not the previous `hash` | `ledger_chain_broken` | `conflict` | Broken chain |
| First event not genesis, or a second genesis | `ledger_genesis` | `missing`, `conflict` | Broken chain |
| `recorded_at` earlier than the previous | `ledger_time_order` | `range` | Broken chain |

On a broken chain the engine refuses every append (R4), answers reads from the valid prefix with
a warning (R33), and the loop stops with the stop reason of SPEC-0002 for a broken state. The
report carries `source` `{file, line}` (SPEC-0001, Errors), the pointer into the event, the
`expected` and found values, `seq`, and the revision of the last valid event. Only a person can
recover (R34), by moving the broken tail to the quarantine and continuing the chain from the
last valid event; see Q2.

## Clock times

- Events carry clock times: `recorded_at` set by the writer (R27), and times reported by the
  tracker such as `merged_at`. They are data, covered by the hash, and replay reads them; it
  never compares them with the current clock.
- The core never reads a clock. The runtime passes the current time to one function,
  `due_events(state, now)`, which returns the events due by time, such as `lease_expired`. The
  runtime appends them before it computes the next step (R28, AC5).
- A lease ends at the `recorded_at` of the latest event of the session of the holder on that
  issue plus `lease_minutes` (R46).
- Envelopes never hold the time of the call (ADR 0028, SPEC-0004 R15). A result may hold the
  `recorded_at` of recorded events or summaries: those values are part of the state at
  `meta.revision`, so the same input on the same revision gives the same envelope at any time
  (AC25). Durations are computed between recorded times, never as an age relative to now.

## Summaries and roll-up

### Content

A summary has one schema, `schemas/ledger/summary/v<N>.json`, with a `level`. The examples are
`examples/summary.session.valid.json`, `examples/summary.session.completed.valid.json`,
`examples/summary.issue.valid.json` and `examples/summary.milestone.valid.json`.

| Name | Type | Levels | Meaning |
| --- | --- | --- | --- |
| `schema_version` | integer | all | Version of the summary schema |
| `level` | `session`, `issue`, `milestone` | all | Level |
| `session`, `issue`, `milestone` | subject fields | as applicable | Subject, plus the issue and milestone a session or an issue belongs to |
| `task` | `TASK` identifier | issue, optional | Task of the issue |
| `outcome` | session: `handed_over`, `completed`, `abandoned`; issue: `open`, `merged`, `closed_without_merge`; milestone: `open`, `done` | all | Result |
| `as_of` | `recorded_at` of the last event included | all | Time of the summary, from the ledger |
| `revision` | hash of the last event included | all | Point of the ledger summarised |
| `handover` | hand-over identifier | session, when `handed_over` | Source of the session |
| `next_step` | string | session and issue, when a hand-over exists and the issue is open | From the latest hand-over |
| `open_questions`, `not_done` | lists of strings | all | From the latest hand-over of each open issue |
| `pull_requests` | list of `{number, state, head_sha, merge_commit, merged_at}` | all | `state` is `open`, `merged` or `closed` |
| `rulings` | list of `{decision, reason, cost_if_wrong, event}`, plus `issue` at milestone level | all | Active rulings; `event` is the hash of the ruling event |
| `sessions` | list of session identifiers | issue | Sessions of the issue |
| `issues` | list of `{issue, outcome}` | milestone | Issues of the milestone |
| `files_touched` | list of strings | session, issue | Union of the hand-overs, without duplicates |
| `cost` | `{tokens_in, tokens_out, cost_micros, currency, duration_ms, executor_calls}` | all | Totals of R45 |
| `omitted` | object from a section name to the count of dropped entries | all | Entries left out by the fill rule (R35); `{}` when none |
| `tokens` | integer | all | Size of the summary without `tokens`: bytes of its canonical JSON divided by four, rounded up (SPEC-0001 Q6) |

The issue summary is built from the session summaries of the issue and the pull request events;
the milestone summary from the issue summaries of the milestone. A milestone with 200 sessions
is therefore summarised from its issues, never from its sessions. Membership of an issue in a
milestone comes from `checkout_acquired` (`milestone`).

### Roll-up

The fill rule (R35) is deterministic; no model writes a summary (Q4).

1. Write the fixed part: every field except the sections below, with `omitted` empty.
2. Fill the sections in this order: `open_questions`, `not_done`, `pull_requests`, `rulings`,
   `issues` or `sessions`, `files_touched`. Within a section, entries are ordered by the `seq` of
   the event they come from, newest first, and in the order of their source list when the event
   is the same.
3. Add an entry only when the summary still fits the budget with it. When an entry does not fit,
   the rest of that section is dropped and counted in `omitted`, and the next section is filled.
4. Compute `tokens` last.

The fixed part fits by construction: the configuration check of R37 keeps the session and issue
budgets above the hand-over budget, which bounds `next_step`, plus 256 tokens for the other
fixed fields.

### Budgets

| Key of `[ledger.budgets]` | Default, tokens | Bounds |
| --- | --- | --- |
| `handover` | 1,200 | Hand-over record (R12) |
| `session` | 1,500 | Session summary |
| `issue` | 2,000 | Issue summary |
| `milestone` | 4,000 | Milestone summary |

The resume pack of an issue (R44) is therefore at most 7,200 tokens with the defaults. Other
keys of `[ledger]`: `lease_minutes` (default 120), `snapshot_every` (default 10,000 events),
`line_limit_bytes` (default 65,536), `lock_timeout_seconds` (default 10). Q5 asks the
maintainer to confirm the defaults.

### Publication on the tracker

ADR 0003 makes the summaries on the issues the durable memory of the project. The core renders
what to publish; the tracker adapter writes it (SPEC-0005); the event records it (R38).

| What | When | Where |
| --- | --- | --- |
| Hand-over with the cost of its session | At the end of a session with a hand-over | New comment on the issue (SPEC-0007 R5) |
| Session summary | At the end of an abandoned session | New comment on the issue |
| Issue summary | When the issue merges or closes | One comment on the issue, replaced when its hash changes |
| Milestone summary | When an issue of the milestone merges or closes | Q3 |

A published body has three parts: a first line `<!-- wayloop:summary level=<level>
subject=<subject> sha256=<hex> -->`; the rendering in Markdown, with English headings and the
prose of the project; and the canonical JSON in a collapsed section. The rendering is a pure
function of the JSON. Publication is skipped when a comment with the same marker and hash
exists, and a failed publication stays pending in the state and is retried at the next
publication step. A dry run lists the planned publications and writes nothing (SPEC-0004 R8).

## Index

The SQLite index of ADR 0007 holds events, rulings, hand-overs and summaries with full-text
search. It records the revision it was built at; before a query, the engine adds the events
after that revision, and rebuilds the index when that revision is not in the chain, as after a
recovery (R26). Agents reach it only through `wayloop history`, `wayloop rulings` and `wayloop
find` and the MCP tools generated from them, within the output budget of SPEC-0004 R6. No
transition reads it. The code index of the survey is SPEC-0010.

## Errors

Every refusal is the error report of ADR 0009 C3 with the field `source` of SPEC-0001, Errors.
Pointers follow RFC 6901 from the root of the event, the summary or the input sent. A new
meaning gets a new code; codes are never renamed or reused (ADR 0009 C8).

| Requirement | Code | Reasons |
| --- | --- | --- |
| R1, R5, R9, R29, R45 | `event_schema` | `missing`, `unknown_field`, `type`, `enum`, `length`, `range`, `syntax`, `requires` |
| R2, R4 | `ledger_chain_broken` | `conflict` |
| R14 | `ledger_genesis` | `missing`, `conflict` |
| R15 | `ledger_hash_mismatch` | `conflict` |
| R16 | `ledger_line_malformed` | `syntax`, `length` |
| R17 | `ledger_sequence_gap` | `range` |
| R18 | `ledger_locked` | `state` |
| R19 | `revision_stale` | `stale` |
| R20 | `ledger_torn_tail` (warning) | `syntax` |
| R22 | `event_kind_unknown`, `event_version_unknown` | `enum` |
| R23 | `ledger_format_unknown` | `enum` |
| R25 | `projection_mismatch` (warning) | `conflict` |
| R27 | `ledger_time_order` | `range` |
| R30 | `event_duplicate` (warning) | `duplicate` |
| R31 | `snapshot_mismatch` (warning) | `conflict` |
| R33 | `ledger_broken_prefix` (warning) | `state` |
| R34 | `recovery_refused` | `state` |
| R37 | `ledger_budget_config` | `range` |
| R39 | `publication_mismatch` (warning) | `conflict` |
| R41 | `summary_mismatch` (warning) | `conflict` |
| R42 | `ruling_unknown` | `missing` |
| R10 | `checkout_held` | `conflict` |
| R12 | `handover_budget` | `length` |
| R48 | `checkout_not_held` | `state` |

R40 uses the code of SPEC-0020.

## Examples

`examples/` holds fixtures computed with the hashing rule above. They form one story: issue 12
worked in two sessions by two executors, a ruling, a failed and a passed check run, a review, the
merge, and the summaries.

| File | Expected result |
| --- | --- |
| `ledger.valid.jsonl` | Valid; 23 events; revision `43f8645e6723416a8d2edd8a127ccc64905da44c97c943ca8e61f9b4cb5a0f14` |
| `ledger.edited-line.invalid.jsonl` | Line 5 payload changed without a new hash: `ledger_hash_mismatch` at line 5 |
| `ledger.rehashed-line.invalid.jsonl` | Line 5 changed and hashed again: `ledger_chain_broken` at line 6, `pointers` naming line 5 |
| `ledger.sequence-gap.invalid.jsonl` | Line 7 removed: `ledger_sequence_gap` at line 7, expected 6 |
| `ledger.non-canonical.invalid.jsonl` | Line 3 with white space: `ledger_line_malformed`, `syntax`, line 3 |
| `ledger.torn-tail.invalid.jsonl` | Line 23 cut without its line feed: `ledger_torn_tail` warning, 22 valid events |
| `summary.session.valid.json` | Session summary with a hand-over; its SHA-256 is in line 11 |
| `summary.session.completed.valid.json` | Session that ended after the merge, without a hand-over; line 20 |
| `summary.issue.valid.json` | Issue summary after the merge; lines 21 and 23 |
| `summary.milestone.valid.json` | Milestone summary of M1; line 22 |

The payload of `executor_call_recorded` in the fixtures uses the fields of R45; its schema is
owned by SPEC-0008 and the fixture follows it once that schema exists.

## Assumptions on SPEC-0002

SPEC-0002 is being rewritten. This specification assumes that:

- the transition function is pure and folds one event of this catalogue at a time, and the
  next-step function reads only the state (no clock, no index);
- an issue becomes claimed on `checkout_acquired` and ready again on `lease_expired` or
  `checkout_released` without a merge;
- pull request events drive the review and merge steps, and `pull_request_merged` completes the
  issue cycle;
- `result_rejected` counts as a failed attempt for the limit on repeated failure;
- a broken ledger is a stop reason of the "broken plan" or a new "broken state" kind;
- approvals and answers arrive as `approval_recorded` and `question_answered` with a person as
  actor.

## Interfaces
- Ledger files under `<git common dir>/wayloop/` with the layout of section Storage. New.
- Schemas: `schemas/events/<kind>/v1.json` for each kind owned here; `schemas/ledger/summary/v1.json`; `schemas/ledger/snapshot/v1.json`. New; every later change is a new snapshot.
- Configuration: `[ledger]` and `[ledger.budgets]` of `wayloop.toml` (ADR 0010), keys of section Budgets. New.
- Commands, to be added to SPEC-0004: `wayloop history`, `wayloop rulings` (existing), `wayloop ledger verify [--full]`, `wayloop ledger snapshot`, `wayloop ledger recover` (person only), `wayloop index rebuild`, `wayloop checkout`, `wayloop handover`, `wayloop summary <level> <subject>`. All results that read state report `meta.revision` (SPEC-0004 R12).
- Error codes of section Errors. New.
- Every addition is additive. No released contract changes.

## Dependencies
- SPEC-0001 is Approved: the hand-over record, its identifier and `tokens`, and the field `source` of errors come from it.
- SPEC-0004 R11, R12 and R15 fix `input_hash`, the revision and the absence of clock times in envelopes; this specification relies on them.
- SPEC-0020 R4 provides the secret check called by R40.
- SPEC-0005 performs the tracker calls of publication.
- ADR 0013 is Proposed; the worktree name in a checkout follows it.

## Plan and tasks
`plan.md` and `tasks.md` are written after the maintainer approves this specification
(`specs/README.md`, Agent workflow).

## Open questions
- [ ] **Q1. Retention of closed segments after a snapshot.** Options: (a) keep every segment uncompressed, and delete only on an explicit command of a person, recorded as an event; (b) compress closed segments automatically; (c) delete closed segments once their summaries are published. Recommendation: (a). A year of events is a few hundred megabytes at most, full verification (R32) and audit need every segment, and deletion of history must be a decision of a person.
- [ ] **Q2. Recovery of a broken chain.** Options: (a) a person runs `wayloop ledger recover`, which quarantines the tail and continues the chain from the last valid event with `ledger_recovered` (R34); (b) start a new ledger whose genesis names the old one in `previous_ledger`, leaving every old file untouched; (c) no command, manual repair only. Recommendation: (a). It keeps one ledger and one chain, loses only events after the break, which the quarantine keeps for inspection, and stays a person decision.
- [ ] **Q3. Where the milestone summary is published.** A tracker milestone has no comments. Options: (a) a section of the milestone description below the structured block, replaced when its hash changes; (b) a comment on a pinned tracking issue per milestone; (c) not published, file only. Recommendation: (a). It needs no extra issue, sits where the milestone is read, and the budget of 4,000 tokens fits the description limit of the tracker.
- [ ] **Q4. Who writes summaries.** Options: (a) the deterministic fill rule of section Roll-up, with prose only from hand-overs; (b) a model condenses the children into a new text, validated by the summary schema. Recommendation: (a). It keeps summaries a function of the ledger (constitution II), testable by replay, at no token cost; (b) can be added later as an optional field without changing the fixed part.
- [ ] **Q5. Default values.** Proposed: budgets of 1,200 (hand-over), 1,500 (session), 2,000 (issue) and 4,000 (milestone) tokens; `lease_minutes` 120; `snapshot_every` 10,000 events; `line_limit_bytes` 65,536; `lock_timeout_seconds` 10. Options: (a) accept them; (b) change some values. Recommendation: (a), and revise them with the measures of the benchmark (SPEC-0019).
- [ ] **Q6. Verification on load.** Options: (a) verify the events after the latest snapshot on every load, and every segment with `wayloop ledger verify --full`, which CI and `wayloop run` start with; (b) verify every segment on every load. Recommendation: (a). Load time stays bounded by `snapshot_every`; an edit before the snapshot is found by the full check at the start of every run.
- [ ] **Q7. Identity of a ruling.** Options: (a) the hash of its `ruling_recorded` event, no new identifier type; (b) a new artifact identifier `RULING-<seq>` in SPEC-0001. Recommendation: (a). A hash is unique across clones and needs no allocator, and rulings are cited mostly by the engine; (b) needs a change of an Approved specification.
- [ ] **Q8. Repair of a torn tail.** Options: (a) automatic, as in R20, because the partial line was never acknowledged; (b) stop and ask a person, as for a broken chain. Recommendation: (a). A crash during an append is a normal event of a long run and loses no acknowledged data; the quarantine and the event keep the evidence.
