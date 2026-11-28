# SPEC-0003: Ledger events, hash chain and replay

| | |
| --- | --- |
| Status | Draft |
| Milestone | M1 · Engine core |
| Owner | @salvo8519 |
| Related | #12, #48, #50, #51, #52, #58, #161, #162, #309, #316; ADR 0002, ADR 0003, ADR 0006, ADR 0007, ADR 0008, ADR 0009, ADR 0010, ADR 0013, ADR 0028, ADR 0029, ADR 0030; SPEC-0001, SPEC-0002, SPEC-0004, SPEC-0005, SPEC-0006, SPEC-0007, SPEC-0008, SPEC-0009, SPEC-0010, SPEC-0020, SPEC-0021 |
| Commit scope | ledger |
| History | Reopened on 2026-11-25 for the counter-review; approval pending |

## Problem
Agent sessions end, crash and hand over to each other, often in parallel worktrees of one
clone. The state of the loop must survive them, be the same whoever reads it and on every
platform, and its history must show accidental corruption and naive edits. An agent that
arrives after weeks must resume from a summary of known size, not from months of events. A
maintainer must be able to audit every decision taken by agents, with its reason and its cost.

## Scope
**In scope:**
- The event: common fields, the one catalogue of every kind with the specification that owns its payload, the payloads owned by this specification, schema versions and migration.
- The hash chain: canonical JSON (RFC 8785), SHA-256, the genesis event, the line format, detection and report of a broken or regressed ledger, recovery.
- The writer: one writer (ADR 0006), the lock between processes and worktrees, durability, expected revision, idempotent retries, torn writes, supported storage.
- Storage under the git common directory: segments, YAML projection, snapshots, summaries, hand-overs, quarantine, and the SQLite index as a rebuildable cache reached through the CLI and the MCP server (ADR 0007).
- Replay and its test, snapshots and replay equivalence.
- Clock times: where they are recorded and where they are never placed (ADR 0028).
- Sessions, checkout of an issue with a lease, hand-over and resumption.
- Pull request events, rulings and cost records.
- Session, issue and milestone summaries: content, deterministic roll-up, size and token budgets, publication on the tracker, reading back and import into a new clone (ADR 0003).
- The security rules of the ledger content: secret check, paths, rendering on the tracker, and the resume pack as data.

**Out of scope:**
- The transitions, guards and the state fields that the state machine derives from events, and the payloads of the kinds it owns (SPEC-0002).
- The payload schemas of event kinds owned by other specifications (section Event catalogue).
- The command set and the envelope (SPEC-0004), apart from the rules on revisions and clock times that the ledger imposes.
- The tracker calls that publish a summary (SPEC-0005) and the content of gates and questions (SPEC-0007).
- The code index of the survey (SPEC-0010); this specification fixes only the rules of the ledger index.
- The detection of secrets (SPEC-0020 R4), which the writer calls.
- Merging the ledgers of two clones. Each clone has its own ledger, and one clone works on a project at a time; the durable memory shared between machines is the set of published summaries and hand-overs (ADR 0003), which a new clone imports (R55).

## User stories
- As an agent arriving after weeks, I want a summary of known size, so that I resume without reading months of history.
- As a maintainer, I want to see every decision taken by agents with its reason, so that I can audit and undo it.
- As a maintainer, I want an edited, truncated or replaced ledger to be detected and located, so that I never build on a corrupted state.
- As an agent in one worktree, I want my checkout of an issue to stop another agent from taking it, so that two agents never work on the same issue.
- As an agent of any vendor, I want to start from the hand-over of the previous session, whoever wrote it, so that work continues without loss.
- As a maintainer who loses a machine, I want the summaries on the issues to carry the memory of the project, so that a new clone resumes from them.

## Requirements (EARS)
Requirement numbers are identities, not an order: requirements added after the first draft
take the next free number (SPEC-0001 R20), whatever section they belong to. R14 to R17 and AC7
to AC9 were merged on `main` by #309 and keep their numbers; the requirements and criteria of
this draft that held the same numbers took the next free ones:

| Earlier draft of this pull request | Now |
| --- | --- |
| R14 (genesis event), R15 (hash), R16 (line format and limit), R17 (sequence numbers) | R57, R58, R59, R60 |
| AC7 (valid fixture), AC8 (hash under permutation), AC9 (non-canonical line) | AC56, AC57, AC58 |

The amendments of 2026-11-25 answer the counter-review of SPEC-0002 and SPEC-0003; each one
names its item. R62 and R63 and AC63 to AC72 are new.

- **R1**: THE ENGINE SHALL append every event to the ledger as one JSON Lines line and never rewrite, reorder or delete an acknowledged line; the only exceptions are the repair of a torn tail by the writer (R20), which removes no acknowledged line, and the recovery of R34.
- **R2**: THE ENGINE SHALL store in every event, as `prev_hash`, the `hash` of the previous event of the ledger, and in the genesis event 64 zero digits.
- **R3**: WHEN the ledger is replayed from the genesis event, THE ENGINE SHALL produce a state and a projection byte-identical to those the writer obtained by folding each event as it appended it, on every supported platform.
- **R4**: IF the hash chain is broken, THEN THE ENGINE SHALL refuse every append and report the first broken event with `source` `{ledger, line}` (its segment and line, section Contracts of the ledger), its pointer, the expected value in `expected`, and in `found` the value found, its sequence number and the revision of the last valid event (`found`: decided by the maintainer on 2026-11-25, counter-review item 65).
- **R5**: THE ENGINE SHALL record every ruling as an event with a decision, a reason and the cost if wrong, and refuse a ruling that lacks any of them.
- **R6**: THE ENGINE SHALL produce session, issue and milestone summaries as JSON validated by the summary schema, each within the token budget of its level.
- **R7**: THE ENGINE SHALL write snapshots of the state and, WHEN it loads the ledger, replay from the latest valid snapshot plus the events that follow it.
- **R8**: WHEN the engine reads an event, a snapshot or a summary of an earlier known schema version, THE ENGINE SHALL verify the hash of the stored bytes first, then migrate the document in memory to the current version, and never rewrite a stored event.
- **R9**: THE ENGINE SHALL record the pull request observations opened, synchronized (new head), checks completed with the name and conclusion of every check, review submitted, merged, closed without merge and reopened, each with its pull request number, issue, branch or head commit as listed in section Payloads.
- **R10**: THE ENGINE SHALL record a checkout of an issue with its holder, session, executor, branch and worktree name, and refuse a checkout of the issue by another session while the lease of the holding session holds, with the code `checkout_held`, the reason `conflict`, and the holder and the end of the lease in `found` (counter-review item 65).
- **R11**: WHEN the lease of a session ends, THE ENGINE SHALL record, through `due_events`, a `checkout_released` event with the reason `lease_expired` for each checkout of that session.
- **R12**: WHEN the holder of a checkout ends a session, THE ENGINE SHALL record a hand-over (SPEC-0001, Hand-over record) and refuse one whose size exceeds the hand-over budget, with the reason `length`, the measured size in size tokens in `found` and the budget in `message`; WHEN the last holder of an issue submits a hand-over after its lease ended and before another session claims the issue, THE ENGINE SHALL record it as a late hand-over under SPEC-0002 S5, and refuse it with `checkout_not_held` once another session has claimed the issue (decided by the maintainer on 2026-11-25, counter-review items 42 and 65).
- **R13**: WHEN an agent checks out an issue that has a hand-over, THE ENGINE SHALL start its brief from the resume pack of R44, whatever agent or executor wrote the hand-over.
- **R14**: THE ENGINE SHALL record one coverage state per coverage unit as events of the ledger, and derive the coverage of a requirement, a specification and a milestone from them.
- **R15**: THE ENGINE SHALL store with every verdict the commit it was produced on, a flag for a dirty working tree and the citations it rests on.
- **R16**: WHEN a merge into the default branch (SPEC-0002, Configuration in the state), or a push to it that the git adapter observes, changes a file cited by the evidence of a `covered` unit, the changed files being the paths of the merge or the push relative to its base, THE ENGINE SHALL record `coverage_stale` for that unit, move it to `stale` and add it to the ready work, unless every verdict of the unit was produced on the merged or pushed head.
- **R17**: IF a verdict was produced on a commit other than the head of the branch under verification, or on a dirty working tree, THEN THE ENGINE SHALL refuse it and record the reason.
- **R18**: THE ENGINE SHALL write through one writer that holds an exclusive whole-file lock of the operating system on `ledger.lock` for each write, reads, verifies and folds every event appended since its last known `seq` before it decides (decided by the maintainer on 2026-11-25, counter-review item 76), appends the lines, flushes them to stable storage, and only then acknowledges the events; the writer releases the lock after the write, and the operating system releases it when the process ends.
- **R19**: WHEN a command that changes state arrives, THE ENGINE SHALL evaluate its expected revision under the lock of the writer, after the retry check of R52, with the rule and the refusal of SPEC-0002 R41, and append nothing when it is stale.
- **R20**: IF the last line of the last segment has no final line feed, THEN every reader SHALL ignore that line, and the writer, under its lock, SHALL write its bytes to the quarantine file `quarantine/torn-<seq>-<sha256>.bin` and flush it, then truncate the segment to its last line feed and flush it, then record a `torn_tail_discarded` event with their size and hash, and report a warning, in that order, so that a crash at any point leaves either the torn line or its quarantined copy (decided by the maintainer on 2026-11-25, counter-review item 70); `<seq>` is the `seq` the torn line would have had, 12 digits.
- **R21**: IF the output of an agent or an executor does not validate against the schema of its step, THEN THE ENGINE SHALL record a `result_rejected` event with the step, the errors with the code `result_schema` of SPEC-0002 Errors and their pointers (second counter-review B7), and a reference to the stored output by hash, count it as one retry of the step (SPEC-0002 R23), and leave every other part of the state unchanged except the closed dispatch, the hold `executor_failure` of SPEC-0002 I29 when the retries are at their limit, and the `budget_exhausted` of SPEC-0002 R29 when its executor call reaches a budget (decided by the maintainer on 2026-11-25, counter-review items 58 and 79).
- **R22**: IF an event has a kind or a schema version the engine does not know, THEN THE ENGINE SHALL stop loading, report the line with the reason `enum`, the known values in `expected` and the minimum engine version recorded by the ledger, which is the `min_engine_version` of the last `min_engine_version_raised` before that line, or of the genesis event when there is none, and refuse every append; every engine, from 0.1.0, registers `min_engine_version_raised`, so that it reads the minimum before it meets a kind it does not know (decided by the maintainer on 2026-11-25, second counter-review B9).
- **R23**: THE ENGINE SHALL read the format version, the hash algorithm, the canonicalisation and the line limit of a ledger from its genesis event, and refuse a ledger whose values it does not know with the reason `enum`.
- **R24**: THE ENGINE SHALL keep the ledger and its derived files under `<git common dir>/wayloop/` with the layout of section Storage, so that every worktree of a clone reads and writes the same ledger.
- **R25**: WHEN events are appended, THE ENGINE SHALL rewrite the projection `state.yaml` under the lock, through a temporary file renamed over it, deterministically from the state, as the object `{revision, seq, state}` with its three members in that order and `state` in canonical order (decided by the maintainer on 2026-11-25, counter-review item 67); IF the stored projection differs from the projection of the state, THEN THE ENGINE SHALL report a warning and rewrite it.
- **R26**: THE ENGINE SHALL treat the SQLite index as a cache built from the ledger and the summaries, record the `(seq, hash)` it was built at, bring it to the current revision with upserts keyed by `seq` before answering a query, rebuild it when its revision is not in the chain, and never read it to decide a transition.
- **R27**: THE ENGINE SHALL set `recorded_at` of every event, a `Timestamp` in whole seconds of UTC with the suffix `Z`, from a time given to the writer as an input, use the `recorded_at` of the previous event when the given time is earlier, and IF the given time is earlier by more than `limits.lease_minutes` minutes, or by more than the default of 120 minutes before the first `limits_set`, THEN refuse the append with the code `clock_behind` (decided by the maintainer on 2026-11-25, counter-review items 9 and 81).
- **R28**: THE ENGINE SHALL take the current time only as the explicit input of `due_events`, append the events it returns under the lock before deciding any input that changes state, command, result or observation, never call it for a command that only reads, and compute every result from recorded events, so that no envelope holds the time of the call (every input: decided by the maintainer on 2026-11-25, counter-review item 9).
- **R29**: THE ENGINE SHALL read every number of an event by value with the number rules of `wayloop-schemas` (plan of SPEC-0001, Numbers), refuse a number that is not an integer with the reason `type` and an integer beyond `SAFE_INTEGER` in magnitude with the reason `range`, and write every integer in its canonical form, so that a stored line holding a fraction or an exponent is refused under R59.
- **R30**: WHEN an observation carries an `external_id` already recorded, THE ENGINE SHALL report the duplicate, which SPEC-0002 R33 leaves without effect, as a warning.
- **R31**: WHEN the engine takes a snapshot, THE ENGINE SHALL write and flush the snapshot file `snapshots/<seq>.json`, named by the `seq` of the last event it includes, then record a `snapshot_taken` event with that sequence number, the SHA-256 and the fold version of the snapshot, start a new segment after it, ignore with a warning a snapshot whose hash differs from its event, and ignore with the warning `snapshot_orphaned` a snapshot file that no `snapshot_taken` event anchors (decided by the maintainer on 2026-11-25, counter-review item 71).
- **R32**: WHEN the engine loads the ledger, THE ENGINE SHALL verify every event after the latest valid snapshot and its link to that snapshot; WHEN a session starts, and WHEN `wayloop ledger verify --full` runs, THE ENGINE SHALL verify every segment from the genesis event.
- **R33**: WHILE the chain is broken or the ledger regressed, THE ENGINE SHALL answer commands that read state from the valid prefix, with a warning that names the first broken event or the regressed file.
- **R34**: WHEN a person runs the recovery of a broken or regressed ledger, THE ENGINE SHALL show the last event it keeps, require its hash as confirmation, move every line from the first broken event onward to the quarantine, and append a `ledger_recovered` event linked to the last kept event; IF the command does not come from a person through a channel of section Person-only commands, or the ledger is neither broken nor regressed, THEN THE ENGINE SHALL refuse it with the reason `state`.
- **R35**: THE ENGINE SHALL fill a summary with the fill rule of section Roll-up, so that the result is a function of the events alone, fits its budget, and counts every dropped entry in `omitted`.
- **R36**: THE ENGINE SHALL compute the session summary when a session ends, the issue summary when a session of the issue ends and when the issue merges, its pull request closes or its task is superseded, and the milestone summary when an issue of the milestone merges or its pull request closes, each subject at most once per input, and record each with a `summary_recorded` event that carries the budget used; until #52 builds the summaries, no `summary_recorded` is emitted (decided by the maintainer on 2026-11-25, counter-review item 80).
- **R37**: THE ENGINE SHALL take the summary budgets from `ledger_budgets` of the latest `limits_set` (SPEC-0002), with the defaults of section Budgets written by `wayloop init`, and refuse a configuration in which a budget is below 256 tokens, the session or the issue budget is below the hand-over budget plus 256, or the milestone budget is below the issue budget, with the reason `range`.
- **R38**: WHEN a summary or a hand-over is to be published, THE ENGINE SHALL render it deterministically with a marker that names its level, subject and SHA-256 and with its canonical JSON, publish it through the tracker adapter, skip it when a comment with the same marker and hash exists, and record a `summary_published` event after the tracker confirms it.
- **R39**: WHEN the engine reads a published summary or hand-over, THE ENGINE SHALL accept only a comment whose author, as reported by the tracker, is the configured account of the runtime or a recorded approver, validate its JSON against its schema and its hash against the marker, and treat a comment that fails any check as not published, with a warning.
- **R40**: THE ENGINE SHALL pass every payload through the secret check of SPEC-0020 R4 before it is appended, every executor output before it is stored as a blob, and every rendered summary before it is published; THE ENGINE SHALL refuse a command or a result that fails, and SHALL record an observation that fails with each failing value redacted. Until SPEC-0020 is approved, the writer calls the check through an interface whose implementation finds nothing, and AC38 waits for SPEC-0020 (decided by the maintainer on 2026-11-25, counter-review item 77).
- **R41**: THE ENGINE SHALL write every summary file as its canonical JSON followed by one line feed, and IF the SHA-256 of a stored summary file differs from that of its `summary_recorded` event, THEN THE ENGINE SHALL report a warning and write it again from the events, at the summary schema version the event records.
- **R42**: WHEN a recorded approver revokes a ruling, THE ENGINE SHALL record a `ruling_revoked` event that names the hash of the ruling event and a reason, keep the ruling in the history, and leave it out of the active rulings and of later summaries.
- **R43**: WHEN the lease of a session ends, whether or not the session holds a checkout, THE ENGINE SHALL record, after the checkout releases of R11, `session_ended` with the outcome `abandoned` (a session with no checkout: decided by the maintainer on 2026-11-25, second counter-review B12), and compute its session summary from its events once #52 builds the summaries (counter-review item 80).
- **R44**: THE ENGINE SHALL build the resume pack of an issue from the milestone summary, the issue summary and the latest hand-over of the issue, in that order, without repeating an entry already given, within the sum of their budgets, and SHALL place it in the brief as quoted data that names its author, never in the instruction part of the brief.
- **R45**: THE ENGINE SHALL record every executor call with role, executor, model, effort, step, phase, model turns (`model_turns`, the turns of the model inside the call, never the turns of SPEC-0002; decided by the maintainer on 2026-11-25, counter-review item 56; the name decided by the maintainer on 2026-11-25, with the amendment), provider tokens in, provider tokens out, duration in milliseconds and the source of its cost, with the cost in integer micro-units of a currency when the source gives one, and refuse a record that lacks a required field.
- **R46**: WHILE a session is open, whether or not it holds a checkout, THE ENGINE SHALL extend its lease with every event caused by a command or a result of that session, `lease_renewed` included, and with no other event: the events of `due_events`, the integrity events of the writer and the events caused by observations never extend a lease (decided by the maintainer on 2026-11-25, counter-review item 82; the list of events decided by the maintainer on 2026-11-25, with the amendment); a session that holds no checkout therefore ends as `abandoned` `limits.lease_minutes` minutes after its latest event that extends a lease (R43; decided by the maintainer on 2026-11-25, second counter-review B12); WHEN a checkout ends, THE ENGINE SHALL record a `checkout_released` event with the reason of section Payloads.
- **R47**: THE ENGINE SHALL assign every session the identifier `session-<ledger>-<YYYY-MM-DD>-<NN>`, where `<ledger>` is the first eight digits of the `ledger_id`, the date is the UTC date of its `session_started` event, and `NN` is one more than the highest number used on that date in the ledger, the quarantine and the publications on the tracker, at least two digits; the numbers used in the quarantine reach the core through the `ledger_recovered` event, which carries the highest session number per date and hand-over number per issue found in the quarantined lines (decided by the maintainer on 2026-11-25, counter-review item 73).
- **R48**: IF a hand-over or a step request (SPEC-0002 I3) for an issue comes from a session that does not hold its checkout, other than the late hand-over of R12, THEN THE ENGINE SHALL refuse it with the code `checkout_not_held`, the reason `state` and the holder in `found`; IF a session end or a lease renewal comes from an actor that is not the holder of that session, or a lease renewal from a session that holds no checkout, THEN THE ENGINE SHALL refuse it the same way; a lease renewal applies to the session and carries no expected revision (decided by the maintainer on 2026-11-25, counter-review items 60 and 61), and each one extends the lease: R52 does not apply to it (decided by the maintainer on 2026-11-25, second counter-review B8).
- **R49**: IF a derived file or a published summary cites a revision that is neither in the chain nor in a quarantine recorded by `ledger_recovered`, or a sequence number above the last event, THEN THE ENGINE SHALL report `ledger_regressed` with the file and the revision, and refuse every append until a person runs the recovery of R34.
- **R50**: WHEN `wayloop ledger verify --full` runs, THE ENGINE SHALL also check that the revision of every published summary recorded by `summary_published` or `publication_imported` is in the chain or in a recorded quarantine.
- **R51**: IF the git common directory is on a network file system, on a Windows drive reached from WSL, on a container bind mount or in a folder kept by a file synchronisation client, as far as the platform reports it, THEN THE ENGINE SHALL refuse to open the ledger with the code `ledger_storage_unsupported` and exit code 3.
- **R52**: WHEN a command other than `lease_renewal_requested` arrives whose `input_hash` is already recorded by an event of the same session, or, for a command without a session, by any event of the ledger, THE ENGINE SHALL append nothing and return the result of the recorded events, before it evaluates the expected revision; `expected_revision` is part of the input, so one `input_hash` names one command on one revision. The state, and therefore every snapshot, keeps the map from the `input_hash` of every command other than `lease_renewal_requested`, with or without a session, to the `seq` of the first and the last event it caused, so that the check and the answer need no scan of the ledger and survive a snapshot (decided by the maintainer on 2026-11-25, counter-review item 63). A `lease_renewal_requested` repeats its content, and so its `input_hash`, at every heartbeat of a session; it is never a retry, and every one records `lease_renewed` (SPEC-0002 S4; decided by the maintainer on 2026-11-25, second counter-review B8). Its `input_hash` is not entered in the map, so that renewals neither overwrite an entry nor grow the map (decided by the maintainer on 2026-11-25, second counter-review check N-S4).
- **R53**: WHEN the writer appends the first event of a kind or of a schema version that requires a newer engine than the minimum recorded by the ledger, THE ENGINE SHALL first record `min_engine_version_raised` with the new minimum.
- **R54**: THE ENGINE SHALL record in every snapshot the fold version of the engine that wrote it, a constant of `wayloop-core`, and ignore with a warning a snapshot whose fold version differs from its own; THE PROJECT SHALL keep a golden test that fails when the state replayed from the golden ledger changes while the fold version does not (decided by the maintainer on 2026-11-25, counter-review item 72).
- **R55**: WHEN `wayloop init` creates a ledger in a clone whose tracker holds published summaries and hand-overs, THE ENGINE SHALL import with `publication_imported` every one that passes R39, so that resume packs and identifiers continue from them.
- **R56**: THE ENGINE SHALL refuse in `files_touched` and in every recorded path an absolute path, a `..` segment or a path not in Unicode NFC, and SHALL render published text so that it cannot close its marker or code fence or notify a tracker handle (section Security rules).
- **R57**: THE ENGINE SHALL start every ledger with one genesis event of kind `ledger_created` at sequence 0, and refuse a ledger whose first event is not a genesis event or that holds a second one.
- **R58**: THE ENGINE SHALL compute the `hash` of an event as the SHA-256, in lower-case hexadecimal, of the UTF-8 bytes of the event without its `hash` member, serialised as canonical JSON (RFC 8785), and refuse an event whose stored `hash` differs.
- **R59**: THE ENGINE SHALL write every event as its canonical JSON followed by one line feed, at most `line_limit_bytes` of the genesis event, and refuse on read a line that exceeds the limit, checked before it is parsed, or that is not canonical JSON, with `source` `{ledger, line}`; `decide` SHALL check the size of every event before it hashes it, record a result whose event would exceed the limit as `result_rejected` with the code `event_too_large` and its content in a blob, and refuse a command whose event would exceed it with the same code (decided by the maintainer on 2026-11-25, counter-review item 74); an observation kind whose payload holds a list that grows with the observed change, `plan_observed` and `branch_changes_observed`, is split by its adapter into parts, each one event with `part` and `parts` and its own `external_id`, and applied when its last part arrives; every other observation kind has a payload bounded below the limit (decided by the maintainer on 2026-11-25, Q15).
- **R60**: THE ENGINE SHALL number events with `seq` from 0, each one more than the previous, and report a missing, repeated or out-of-order number with the reason `range`.
- **R61**: THE PROJECT SHALL classify every change of a released contract of the areas `events` and `ledger` as a new version, additive changes included (section Contracts of the ledger, Q9).
- **R62**: THE RUNTIME SHALL perform every operation of the writer on the files of section Storage through one storage interface (open, read, append, flush, truncate, rename, lock), so that tests inject a failure or an interruption at any operation and at any byte of an append (decided by the maintainer on 2026-11-25, counter-review section 3 on AC22 and AC48 and section 4, code pattern 7).
- **R63**: IF a command that reads or writes the ledger finds no ledger under `<git common dir>/wayloop/`, THEN THE ENGINE SHALL refuse it with the code `ledger_missing` and exit code 3; THE ENGINE SHALL treat an empty last segment as valid, and `wayloop init` SHALL write the genesis event to a temporary file, flush it and rename it to the first segment, so that a torn genesis never exists (decided by the maintainer on 2026-11-25, counter-review item 68).

## Acceptance criteria
- **AC1** (R3): Given a ledger of 10,000 events produced by `decide` from random inputs through the writer (SPEC-0002 AC2), when it is replayed twice and compared with the incremental fold of the writer that appended it, then the three states, the three projections and the revisions are byte-identical. Verification: property test (events generated through `decide`: section 3 of the counter-review).
- **AC2** (R4, R58): Given `examples/ledger.edited-line.invalid.jsonl`, when it is loaded, then the error has the code `ledger_hash_mismatch`, `source` is `{ledger, line}` naming the segment loaded and line 5, the pointer is `/hash`, and the revision of the last valid event is the hash of line 4. Verification: TDD, with the files of `examples/` as fixtures.
- **AC3** (R6, R35): Given a milestone of 40 issues with 200 sessions, each hand-over at the budget, when the milestone summary is rolled up, then it validates, its `tokens` is at most the milestone budget, and for each section the kept entries plus the count in `omitted` equal the source entries of that section. Verification: TDD.
- **AC4** (R9, R6): Given a merged pull request, when the issue summary is rolled up, then it has the outcome `merged` and the pull request entry carries the merge commit and `merged_at`. Verification: TDD, with `examples/summary.issue.merged.valid.json`.
- **AC5** (R11, R28, R43): Given a session whose latest event plus its lease is earlier than the time given to `due_events`, when the next command that changes state is decided, then `checkout_released` with `lease_expired` and `session_ended` with `abandoned` are recorded first and the issue is ready; given a command that only reads, then nothing is appended. Verification: property test.
- **AC6** (R13, R44): Given a hand-over written in a session of one executor, when an agent of another executor checks out the issue, then its brief starts with the resume pack as quoted data that names the author of each part, and the pack names that hand-over. Verification: TDD.
- **AC7** (R14): Given a ledger of coverage events, when replayed, then the coverage of each specification equals its `covered` units over its active units. Verification: property test.
- **AC8** (R16): Given a `covered` unit whose verdicts were produced on commit C0 and a merge of another pull request that edits a cited file, when `pull_request_merged` is observed with its `changed_paths` (SPEC-0002 R51), then `coverage_stale` is recorded with the unit, the merge commit as `commit` and the file, the unit is `stale` and appears in the ready work, even when the edited lines are outside the cited range; given a merge that edits no cited file, then the unit stays `covered`; given the merge of the pull request on whose merged head every verdict of the unit was produced, whose changes include the cited file, then the unit stays `covered`; given the merge of a pull request that closes no planned issue, or a `default_branch_pushed` of a commit into the default branch, that edits the cited file, then the unit is `stale` as well and `commit` names the pushed commit. Verification: TDD.
- **AC9** (R17): Given a verdict carrying a commit other than the head of the branch as last observed, when submitted, then it is refused with the code `verdict_commit_stale` and the reason `stale`. Verification: TDD.
- **AC10** (R60, R4): Given `examples/ledger.sequence-gap.invalid.jsonl`, when it is loaded, then the first error has the code `ledger_sequence_gap`, the reason `range`, `source` `{ledger, line}` with line 7 and `expected` holding 6. Verification: TDD.
- **AC11** (R2, R4): Given `examples/ledger.rehashed-line.invalid.jsonl`, when it is loaded, then the error has the code `ledger_chain_broken`, the pointer `/prev_hash` of line 6, and `pointers` naming line 5. Verification: TDD.
- **AC12** (R57, R23): Given `wayloop init` on a new clone, when the ledger is created, then line 1 is `ledger_created` with `seq` 0, `prev_hash` of 64 zeros, `format_version` 1 and `line_limit_bytes` 65,536; given a ledger whose first event is another kind, or that holds a second genesis event, then it is refused with the code `ledger_genesis`; given a genesis event with `hash_algorithm` `sha512`, then it is refused with the code `ledger_format_unknown` and the reason `enum`. Verification: TDD.
- **AC13** (R18, R24): Given a ledger of N events in which each of two sessions holds a checkout of its own issue, a test binary of the runtime and two processes of it in two worktrees of one clone, each sending 1,000 `lease_renewal_requested` commands of its own session through the writer, with a clock that never ends a lease, when both finish, then the ledger holds N + 2,000 events in one valid chain, every acknowledged event is present once, and no event was appended by `due_events`; the test runs in CI on Linux, macOS and Windows. Verification: end-to-end (append path named: section 3 of the counter-review).
- **AC14** (R19, R52): Given an expected revision after which an event named the issue of the command, when a command that changes state runs, then it is refused with `revision_stale` and the reason `stale` (SPEC-0002 Errors) and the ledger is unchanged; given an expected revision after which only events of other issues were recorded, then the command is accepted; given that accepted command, then an event on its issue that makes its expected revision stale, when the command is retried with the same `input_hash` in the same session, then the result of its recorded events is returned, nothing is appended and no `revision_stale` is reported; given two `lease_renewal_requested` of one session with the same `input_hash`, then two `lease_renewed` are recorded (second counter-review B8). Verification: TDD.
- **AC15** (R20): Given `examples/ledger.torn-tail.invalid.jsonl`, when a reader loads it, then the partial line is ignored and nothing is moved; when the writer next takes the lock, then the partial line is moved to the quarantine, a `torn_tail_discarded` event follows line 31, the chain is valid and a warning `ledger_torn_tail` is reported; given a malformed line that is not the last one, then nothing is moved and the error of R59 is reported. Verification: TDD.
- **AC16** (R21): Given an executor result that misses a required field, when it is returned, then a `result_rejected` event is recorded with the error pointer and the hash of the stored output, the retries of the step increase by one, and the state differs from the previous one only in the retries, the closed dispatch and the last event `(seq, hash)`; given the retries at their limit, then the hold `executor_failure` is added too (counter-review item 79). Verification: TDD.
- **AC17** (R22): Given an event of kind `future_kind` or of `schema_version` 99, when the ledger is loaded, then loading stops with the codes `event_kind_unknown` or `event_version_unknown`, the reason `enum`, the known values in `expected` and the recorded minimum engine version, and an append is refused. Verification: TDD.
- **AC18** (R8): Given a ledger fixture with events of schema version 1 of a kind whose current version is 2, when it is loaded, then every event is migrated in memory, the hash is checked on the stored bytes, the files are unchanged byte for byte, and the projection equals that of the same ledger written in version 2. Verification: TDD, with one fixture per released version.
- **AC19** (R8, R39): Given a published issue summary of schema version 1 while the current version is 2, when it is read back, then its hash is checked on the published JSON, it is migrated and validates against version 2. Verification: TDD.
- **AC20** (R24): Given the main worktree and a second worktree of one clone, when `wayloop status` runs in each, then both report the same `meta.revision`. Verification: end-to-end.
- **AC21** (R3, R25): Given the golden ledger of the repository, a small ledger written for the test with its golden `state.yaml`, whose every value a person reviewed against this specification and SPEC-0002 before it was committed, when it is replayed in CI on Linux, macOS and Windows, then the three projections are byte-identical to the golden `state.yaml`. Verification: end-to-end (hand-reviewed golden ledger: section 3 of the counter-review).
- **AC22** (R25, R62): Given a `state.yaml` edited by hand, when the next event is appended, then a warning `projection_mismatch` is reported and the file equals the projection of the state; given a storage that fails at each operation of the rewrite in turn (write of the temporary file, flush, rename), then `state.yaml` is either the previous or the new projection. Verification: TDD, with the storage interface of R62.
- **AC23** (R26): Given a deleted index, when `wayloop history` runs, then the index is rebuilt and the result equals the result before deletion; given an index built at a revision that is not in the chain, then it is rebuilt; given the same events upserted twice, then the index is unchanged; given any content of the index, then the next step is the same. Verification: property test.
- **AC24** (R27): Given a writer clock that moves 30 minutes backwards between two appends with a lease of 120 minutes, when the second event is recorded, then its `recorded_at` equals that of the first; given a clock 3 hours behind, then the append is refused with `clock_behind`; given any appended event, then `recorded_at` matches `YYYY-MM-DDTHH:MM:SSZ`. Verification: TDD, with a fake clock.
- **AC25** (R28): Given one ledger, when `wayloop status` and `wayloop history` run at two different clock times with the same input, then the two envelopes are byte-identical and the ledger is unchanged. Verification: contract test.
- **AC26** (R29): Given a payload holding `1.5` or 2^53, when it is appended, then it is refused with the code `event_schema`, the reason `type` or `range` and the pointer of the number; given a payload holding `1.0` or `1e3`, then the appended line holds `1` or `1000`; given a stored line holding `1.0`, then it is refused with `ledger_line_malformed` and the reason `syntax`. Verification: TDD.
- **AC27** (R30): Given the merge of pull request 31 observed twice by the tracker adapter, when both observations are processed, then one `pull_request_merged` event exists and the second gives the warning `event_duplicate`; given two check suites on one head, then both are recorded. Verification: TDD.
- **AC28** (R7, R31): Given a ledger of random events and a snapshot taken at a random sequence, when the state is loaded from the snapshot plus the tail, then it is byte-identical to the state of a full replay. Verification: property test.
- **AC29** (R31, R7, R54): Given a snapshot file whose content was changed, or whose fold version differs from that of the engine, when the ledger is loaded, then a warning `snapshot_mismatch` names it and the state is loaded from the previous valid snapshot or from the genesis event. Verification: TDD.
- **AC30** (R32): Given an edited line before the latest snapshot, when the ledger is loaded, then no error is reported; when a session starts or `wayloop ledger verify --full` runs, then the error names the line. Verification: TDD.
- **AC31** (R33): Given a ledger broken at line 6, when `wayloop history` runs, then it lists events 0 to 4 with a warning that names line 6, and `meta.revision` is the hash of line 5. Verification: TDD.
- **AC32** (R34): Given a ledger broken at line 6, when a person runs the recovery at an interactive terminal, then the command shows line 5 and refuses to continue without `--confirm` and the hash of line 5; with it, lines 6 onward are in the quarantine, a `ledger_recovered` event follows line 5 with `prev_hash` equal to its hash, and the chain is valid; given the same command through the MCP server, from a session of an agent, or on a valid ledger, then it is refused with the reason `state`. Verification: TDD.
- **AC33** (R35): Given any set of events and any valid budget, when a summary is rolled up twice, then both results are byte-identical, `tokens` is at most the budget, and for each section kept plus `omitted` equals its source entries. Verification: property test.
- **AC34** (R36): Given a session that ends with a hand-over, when it ends, then a session summary and an issue summary are recorded; given the merge of the issue, then the issue summary and the milestone summary are recorded again; given one input that triggers the issue summary twice, then it is recorded once. Verification: TDD.
- **AC35** (R37): Given a hand-over budget of 1,200 and a session budget of 1,300, when the configuration is decided, then it is refused with the code `ledger_budget_config`, the reason `range` and the pointer `/ledger/budgets/session`. Verification: TDD.
- **AC36** (R38): Given a fake tracker, when one issue summary is published twice with the same hash, then one comment exists and one `summary_published` event is recorded; when it is published with a new hash, then the comment is replaced; given a dry run, then no comment is written and the planned publication is listed. Verification: TDD.
- **AC37** (R39): Given a published comment whose JSON was edited, when it is read back, then the warning `publication_mismatch` is reported and the summary is published again at the next publication step; given a valid comment authored by an account that is neither the runtime nor an approver, then it is ignored with `publication_author`. Verification: TDD.
- **AC38** (R40): Given a hand-over whose `next_step` holds a token that the secret check of SPEC-0020 detects, when it is submitted, then it is refused and the ledger and the tracker are unchanged; given a pull request title holding such a token, when it is observed, then `pull_request_opened` is recorded with the title redacted; given a rejected executor output holding one, then no blob is stored and `result_rejected` names the finding. Verification: TDD; waits for SPEC-0020 (R40, counter-review item 77).
- **AC39** (R41): Given a summary file whose content was changed, when the summaries are read, then a warning `summary_mismatch` is reported and the file equals again the one of `examples/` for the same events. Verification: TDD.
- **AC40** (R5, R42): Given a ruling without `cost_if_wrong`, when it is submitted, then it is refused with the reason `missing`; given a ruling revoked by an approver, when `wayloop rulings` runs, then it is absent from the active rulings and present in the history with its revocation. Verification: TDD.
- **AC41** (R43, R11): Given a session whose lease ends without a hand-over, when the due events are computed, then `checkout_released` with `lease_expired` and `session_ended` with the outcome `abandoned` are recorded and the session summary has no `next_step`; given then a hand-over from the same holder before another claim, then it is recorded as late and the issue summary is rolled up again. Verification: TDD.
- **AC42** (R44): Given a milestone, an issue and a hand-over at their budgets that share one ruling, when the resume pack is built, then it holds the three in the order milestone, issue, hand-over, the ruling once, and its size is at most the sum of the three budgets. Verification: TDD.
- **AC43** (R45): Given an executor call record without `cost_source`, when it is submitted, then it is refused with the reason `missing`; given the fixture ledger, when `wayloop cost` runs, then the totals per currency equal the sum of the records and the call without a cost is counted apart. Verification: TDD.
- **AC44** (R10, R46): Given issue 12 checked out by `agent-lane-1`, when `agent-lane-2` checks it out, then it is refused with the code `checkout_held`, the reason `conflict`, and the holder and the end of the lease in `found`; when `agent-lane-1` renews the lease, then the lease is extended and no second checkout is recorded; when the result of its `pull_request` step closes the dispatch, then `checkout_released` with the reason `awaiting_checks` is recorded. Verification: TDD.
- **AC45** (R12, R48): Given a hand-over of 1,300 size tokens with a budget of 1,200 size tokens, when it is submitted, then it is refused with the code `handover_budget`, the reason `length`, 1,300 in `found`; given a hand-over from a session that does not hold the checkout, then it is refused with the code `checkout_not_held` and the reason `state`; given a session end from the holder of a session that holds no checkout, then it is accepted. Verification: TDD.
- **AC46** (R9): Given each pull request event of `examples/ledger.valid.jsonl`, when validated against its payload schema, then it passes; given a checks event whose `checks` are not sorted by name, then it is refused with the reason `syntax`. Verification: contract test.
- **AC47** (R47): Given the ledger `5b0c7a1e-...`, two sessions started on 2027-01-15 and one on 2027-01-16, when they are recorded, then their identifiers are `session-5b0c7a1e-2027-01-15-01`, `session-5b0c7a1e-2027-01-15-02` and `session-5b0c7a1e-2027-01-16-01`; given a quarantine that holds `session-5b0c7a1e-2027-01-16-02`, then the next session of that date is `-03`. Verification: TDD.
- **AC48** (R1, R18, R52, R62): Given a storage of R62 that interrupts an append at every byte offset of its line, and after the line feed before the flush, when the ledger is loaded and the command is retried with the same input in the same session, then every acknowledged event is present once; a partial line is quarantined by the writer and the retry appends the event once; a complete line is kept and the retry appends nothing and returns its result. Verification: property test, with the storage interface of R62.
- **AC49** (R59, R29, R56): Given `examples/canonicalization.vectors.json`, when each input is canonicalised and hashed, then the output and the hash equal the expected ones, and each invalid input is refused with its expected reason: `type` for `1.5`, `range` for 2^53, `syntax` for a lone surrogate and for invalid UTF-8; `1.0` and `1e3` have the canonical forms `1` and `1000`. Verification: contract test.
- **AC50** (R27): Given `examples/ledger.time-order.invalid.jsonl`, when it is verified, then the error has the code `ledger_time_order`, the reason `range` and `source` `{ledger, line}` with line 9. Verification: TDD.
- **AC51** (R49, R50): Given a `state.yaml` or a snapshot whose `seq` is above the last event, or a published summary whose revision is not in the chain or a recorded quarantine, when the ledger is loaded or verified in full, then `ledger_regressed` names the file and the revision and an append is refused; given the recovery by a person, then `ledger_recovered` with the reason `regressed` is recorded and the derived files are rebuilt. Verification: TDD.
- **AC52** (R51): Given a git common directory on a network file system, when a command opens the ledger, then it fails with `ledger_storage_unsupported` and exit code 3, on each platform where the test can mount one. Verification: manual check, recorded in the pull request that builds R51, since CI may not mount a network file system (section 3 of the counter-review).
- **AC53** (R53): Given a ledger whose minimum engine version is 0.1.0, when an engine of version 0.3.0 first appends a kind introduced in 0.3.0, then `min_engine_version_raised` with 0.3.0 precedes it; when an engine of version 0.2.0 loads that ledger, then it stops with the code `event_kind_unknown` and reports 0.3.0, read from the `min_engine_version_raised` it knows; given an engine of version 0.1.0, then it does the same, since 0.1.0 registers `min_engine_version_raised` (second counter-review B9). Verification: TDD.
- **AC54** (R55): Given a tracker holding a published hand-over and an issue summary of issue 12, a forged summary from another account and a new clone, when `wayloop init` runs, then two `publication_imported` events are recorded, the forged one is ignored, the next hand-over is `HANDOVER-12-2` and the resume pack of issue 12 holds the imported ones. Verification: TDD.
- **AC55** (R56): Given a hand-over whose `files_touched` holds `/etc/passwd`, `../x` or a path in NFD, when submitted, then each is refused with its pointer; given a ruling whose text holds a marker of R38, a run of four backticks and a tracker handle, when the summary is rendered, then the marker is escaped, the fence is longer than four backticks and the handle notifies nobody. Verification: TDD.
- **AC56** (R2, R57, R58, R59, R60): Given `examples/ledger.valid.jsonl`, when it is verified, then it passes, every line is canonical, every `prev_hash` equals the `hash` of the previous line, no lease ends between two events of one session, and the revision is `27765ee853d32a87cf9a58811891449fa18c103f1ea96854c95deb9e059c0c0c`, which task T3 replaces with the revision of the regenerated fixture (section Examples, second counter-review B11). Verification: contract test.
- **AC57** (R58): Given any event object and any permutation of the order of its members, when hashed, then the hash is the same and equals the SHA-256 of the canonical bytes without `hash`. Verification: property test.
- **AC58** (R59): Given `examples/ledger.non-canonical.invalid.jsonl`, when it is loaded, then the error has the code `ledger_line_malformed`, the reason `syntax` and `source` `{ledger, line}` with line 3; given a line one byte above the line limit, then the reason is `length` and the line is not parsed. Verification: TDD.
- **AC59** (R15): Given a review result whose vote cites `{role: code, file: crates/a.rs, line: 10, end_line: 20}` and `{role: test, file: crates/a.rs, line: 40, end_line: 52}`, produced on the head of the branch with a clean working tree, when it is accepted, then its `coverage_verdict` holds that `commit`, `dirty` `false` and the two citations as given; given a vote without `commit` or without `dirty`, then the result is recorded as `result_rejected` with the reason `missing` at the pointer of the field. Verification: TDD.
- **AC60** (R37): Given `wayloop init` without `[ledger.budgets]`, when it runs, then the first `limits_set` records `ledger_budgets` `handover` 1,200, `session` 1,500, `issue` 2,000 and `milestone` 4,000, and `meta.defaults_applied` lists the four keys; given `issue` 3,000, then 3,000 is recorded and `issue` is not listed. Verification: TDD.
- **AC61** (R61): Given a contract `events/<kind>` released in version 1 and a member added to its type without a new version, when `scripts/schema_snapshots.py --diff-against origin/main` runs, then it fails and names the contract; given the same change of `ledger/summary`, then it fails too; given an added optional member of a released contract of the area `commands`, then it is classified as additive and passes. Verification: contract test.
- **AC62** (R52): Given a `project_step_requested` without a session, accepted with the expected revision R1, and then an event that makes R1 stale for the project, when the same input is sent again, then the result of its recorded events is returned, nothing is appended and no `revision_stale` is reported; given the same command with the expected revision R2, whose `input_hash` therefore differs, then it is decided as a new command. Verification: TDD.
- **AC63** (R62): Given the writer over the storage interface of R62, when a test storage records every call, then every read, append, flush, truncate, rename and lock of the files of section Storage goes through it, and no other file operation is made by the writer. Verification: TDD.
- **AC64** (R63): Given a git common directory without `wayloop/`, when `wayloop history` or a command that changes state runs, then it fails with `ledger_missing`, the reason `missing` and exit code 3; given a ledger whose last segment is empty after a snapshot, then it loads with no error; given `wayloop init` interrupted after the write of the temporary genesis file and before the rename, then no segment exists and a second `wayloop init` succeeds. Verification: TDD, with the storage interface of R62.
- **AC65** (R20): Given a storage of R62 that interrupts the repair of a torn tail after each of its three operations in turn, when the writer next takes the lock, then the ledger ends either with the torn line or with `torn_tail_discarded`, and the bytes of the torn line are present in the segment or in `quarantine/torn-<seq>-<sha256>.bin`; given a malformed line that is not the last line and has no line feed, then the load stops with `ledger_line_malformed` and the reason `syntax` (counter-review item 69). Verification: TDD.
- **AC66** (R31): Given a snapshot of the state up to `seq` 99, when it is taken, then the file `snapshots/000000000099.json` is written and flushed before `snapshot_taken` with `snapshot_seq` 99; given a snapshot file with no `snapshot_taken` event, then it is ignored with the warning `snapshot_orphaned` and the state is loaded from an earlier snapshot or the genesis event. Verification: TDD.
- **AC67** (R54): Given the golden ledger and the fold version constant, when a change of `apply` changes the replayed state without a change of the constant, then the golden test fails and names the fold version. Verification: TDD.
- **AC68** (R18): Given a writer that last read the ledger at `seq` 10, and another process that appended events 11 to 13 that make the expected revision of a command stale, when the writer takes the lock and decides the command, then it is refused with `revision_stale` and its new events would follow `seq` 13. Verification: TDD.
- **AC69** (R25): Given any state, when the projection is written, then the first three keys of `state.yaml` are `revision`, `seq` and `state`, in that order, the members of `state` are in canonical order, and `state` holds `input_hashes_count` and `external_ids_count` with the sizes of the two collections and not the collections themselves (Q16). Verification: TDD.
- **AC70** (R46): Given a session whose last command was recorded at 10:00 with a lease of 120 minutes, when an observation of the tracker on its issue is recorded at 11:00, then the lease still ends at 12:00; given a `step_started` of the session at 11:00, then it ends at 13:00. Verification: TDD.
- **AC71** (R27): Given no `limits_set` recorded yet, when the given time is 121 minutes earlier than the last `recorded_at`, then the append is refused with `clock_behind`; given 119 minutes, then the event carries the last `recorded_at`. Verification: TDD, with a fake clock.
- **AC72** (R59): Given a `step_result` whose `result_rejected` event would exceed `line_limit_bytes`, when it is decided, then `result_rejected` with `event_too_large` is recorded, its output is in a blob referenced by `{file, sha256}`, and the event is within the limit; given a command whose event would exceed the limit, then it is refused with `event_too_large` and the reason `length`. Verification: TDD.

## Coverage and verification

Definitions shared by SPEC-0002, SPEC-0006 and SPEC-0009.

- **Coverage unit**: one acceptance criterion of an active requirement, identified by `<spec>/<requirement>/<criterion>`, for example `0009/R6/AC3`. It is the smallest thing the engine can declare covered.
- **Coverage state**: `planned`, `in_progress`, `covered`, `uncovered`, `needs_validation`, `stale`. Only the engine moves a unit between states.
- **Verdict**: the result of one verification of a unit: `covered`, `uncovered` or `needs_validation`. A `needs_validation` verdict names the exact fact that the repository cannot establish.
- **Lens**: the question a verifier answers about a unit: `presence` (is there code that implements it, cited by file and line), `behaviour` (does that code do what the criterion says, edge cases included), `proof` (does a test fail without that code and pass with it).
- **Evidence**: a verdict with its citations, the commit it was produced on and whether the working tree was dirty.

How the ledger holds these states (R14 to R17):

| State | Recorded or derived from |
| --- | --- |
| `planned` | `coverage_planned`, recorded when the gate of the specification is approved (SPEC-0002, Gates and approval states) |
| `in_progress` | Derived: a dispatch of `review` is open for an issue whose task cites the requirement of the unit |
| `covered`, `uncovered`, `needs_validation` | The tally of SPEC-0002 R9 over the `coverage_verdict` events of the latest review step of the unit; the engine computes it, no event carries it |
| `stale` | `coverage_stale`, recorded when a merge into the default branch is observed as `pull_request_merged`, whatever pull request it closes, or a push to it as `default_branch_pushed` (SPEC-0002 R51, I18, I24, P13), for a `covered` unit with a cited file in the `changed_paths` that the observation carries, the paths of the merge or the push relative to its base (R16, Q12); a unit whose verdicts were all produced on the merged or pushed head is not stale, because the merge adds no change after them. Changed ranges re-anchored along merges, which mark fewer units stale, arrive with SPEC-0009 |

The coverage of a requirement, a specification or a milestone is the number of its `covered`
units over its active units, with the list of units, their state and their evidence (SPEC-0009
R12). The head that R17 compares with is the head of the branch of the issue as last observed by
`branch_changes_observed` (SPEC-0002); a dispatch of `review` starts only while that head is the
head that `verify` verified, whatever the order of the two observations (SPEC-0002 I3, I10,
`head_not_observed`). A refused verdict is a
`result_rejected` of SPEC-0002 R23 with the code `verdict_commit_stale` or `verdict_tree_dirty`.

## Event model

### Common fields

Every event has the fields below, whatever its kind. Keys are `snake_case`, and enum values are
`snake_case` (ADR 0009 C4). Absent means not given (ADR 0009 C2): an optional field is left out,
never set to `null`.

| Name | Type | Required | Meaning |
| --- | --- | --- | --- |
| `seq` | integer, from 0 | yes | Position in the ledger (R60) |
| `kind` | enum, section Event catalogue | yes | Kind of the event; selects the payload schema |
| `schema_version` | integer, at least 1 | yes | Version of the schema of this kind (R8, R22) |
| `recorded_at` | `Timestamp`: string `YYYY-MM-DDTHH:MM:SSZ`, whole seconds | yes | When the writer recorded it, in UTC (R27) |
| `actor` | object `{kind, id}`; `kind` is `engine`, `agent`, `person` or `tracker` | yes | Who caused the event: the engine for its own decisions, the agent holder for validated agent input, a tracker handle for a person's act, the tracker for an observation |
| `session` | string, R47 | no | Session in which the event occurred; set for events caused by a command or a result of that session, and for the release of a checkout of that session |
| `issue` | integer | no | Tracker number of the issue concerned |
| `input_hash` | string, 64 lower-case hexadecimal digits | for events caused by a command | `meta.input_hash` of the command (SPEC-0004 R11); the idempotency key of R52 |
| `dispatch` | string, 64 lower-case hexadecimal digits | for events caused by a result | Hash of the `step_started` event the result answers (SPEC-0002 R21) |
| `external_id` | string, 1 to 200 characters | for events caused by an observation | Identity of the observed fact, used to ignore duplicates (R30) |
| `payload` | object | yes, may be `{}` | Fields of the kind |
| `prev_hash` | string, 64 lower-case hexadecimal digits | yes | Hash of the previous event; 64 zeros for the genesis event (R2) |
| `hash` | string, 64 lower-case hexadecimal digits | yes | Hash of this event (R58) |

An event carries at most one of `input_hash`, `dispatch` and `external_id`. Events returned by
`due_events`, the integrity events of the writer and `summary_published` carry none.

`external_id` is built by the adapter that observes the fact, so that a repeated delivery of one
fact gives the same value and two facts never share one:

| Source | `external_id` |
| --- | --- |
| Pull request opened, merged | `pull_request:<number>:opened`, `pull_request:<number>:merged` |
| Pull request closed, reopened | `pull_request:<number>:closed:<k>`, `pull_request:<number>:reopened:<k>`, where `k` is the occurrence of that act on the pull request, from 1 |
| New head | `pull_request:<number>:head:<previous_sha>:<sha>`, so that a force-push back to an earlier head is a new fact (decided by the maintainer on 2026-11-25, counter-review item 40) |
| Checks | `check_suite:<id>` or `check_run_attempt:<id>`, the identifier the tracker gives to one run of the checks |
| Review | `review:<id>` |
| Comment of a person (approval, answer, request) | `comment:<id>`, or `comment:<id>:<edit>` for an edited comment |
| Plan observation | `plan:<previous_plan_sha256>:<plan_sha256>:<part>`, with 64 zeros as `<previous_plan_sha256>` for the baseline, so that a plan that returns to an earlier content is a new fact (decided by the maintainer on 2026-11-25, counter-review item 16) |
| Branch changes | `branch:<branch>:<head_sha>:<part>` (R59, Q15) |
| Push to the default branch (`default_branch_pushed`, SPEC-0002) | `push:<commit>` |
| Imported publication | `comment:<id>:<sha256>` |

A publication by the engine (`summary_published`) is not an observation: its event carries no
`external_id`; the marker and hash of R38 make the publication idempotent. Every number is an
integer within the exact range of a double (R29). Amounts of money are integer micro-units
(`cost_micros`) with an ISO 4217 `currency`; durations are integer milliseconds (`duration_ms`).
Floating-point numbers would make canonical JSON depend on the formatting of the platform.

Content larger than an event, such as a rejected executor output or a test log, is stored once
under `blobs/<sha256>` and referenced as `{file, sha256}`; the event never inlines it (R59).

### Event catalogue

This is the one catalogue of the ledger. Every kind recorded by the engine is listed here, with
the specification that owns its payload schema. A kind owned by another specification is
reserved here, and its payload is specified by its owner. Adding a kind is an additive change of
the catalogue, preceded by `min_engine_version_raised` in a ledger that holds it (R53); an
engine that meets a kind it does not know stops loading (R22), because the state would
otherwise differ from that of a newer engine. Consumers of the outputs of the engine, such as
agents and SDKs, accept enum values they do not know (ADR 0009 C5).

An engine registers in its contracts and its fold only the kinds it builds. Version 0.1.0
registers the kinds of the slice of the plan of SPEC-0002 and `min_engine_version_raised`; a
later version that builds another kind of this catalogue records `min_engine_version_raised`
before the first event of that kind (R53), so that 0.1.0 reads the new minimum and stops cleanly
on the kind it does not know, reporting that minimum (R22) (decided by the maintainer on
2026-11-25, counter-review item 78; `min_engine_version_raised` in 0.1.0 decided by the
maintainer on 2026-11-25, second counter-review B9). The kinds owned by SPEC-0021, a Draft specification, are reserved here and registered
only with it.

| Kind | Role | Payload owner | Consumed by |
| --- | --- | --- | --- |
| `ledger_created` | Genesis of the ledger (R57, R23) | SPEC-0003 | Loader |
| `min_engine_version_raised` | Minimum engine version of the ledger (R53) | SPEC-0003 | Loader |
| `session_started` | Session opened (R47) | SPEC-0003 | SPEC-0002 sessions and leases, roll-up |
| `session_ended` | Session closed with its outcome (R36, R43) | SPEC-0003 | SPEC-0002 sessions, roll-up |
| `checkout_acquired` | Issue claimed by a session (R10) | SPEC-0003 | SPEC-0002 checkout (I1, I2) |
| `lease_renewed` | Heartbeat of the holding session (R46) | SPEC-0003 | SPEC-0002 leases |
| `checkout_released` | Checkout ended, with its reason (R11, R46) | SPEC-0003 | SPEC-0002 checkout, lanes |
| `handover_recorded` | Hand-over record of SPEC-0001 (R12) | SPEC-0003 | Resume pack, roll-up |
| `ruling_recorded` | Ruling (R5) | SPEC-0003 | Roll-up, `wayloop rulings` |
| `ruling_revoked` | Revocation of a ruling (R42) | SPEC-0003 | Roll-up, `wayloop rulings` |
| `pull_request_opened` | Observation (R9) | SPEC-0003 | SPEC-0002 I15 |
| `pull_request_synchronized` | Observation: new head (R9) | SPEC-0003 | SPEC-0002 I21, I22 |
| `pull_request_checks_completed` | Observation: one run of the checks (R9) | SPEC-0003 | SPEC-0002 I12, I16, I17, I20 |
| `pull_request_review_submitted` | Observation (R9) | SPEC-0003 | SPEC-0002 I19 |
| `pull_request_merged` | Observation (R9) | SPEC-0003 | SPEC-0002 I18, I24, P13; roll-up |
| `pull_request_closed` | Observation (R9) | SPEC-0003 | SPEC-0002 I25; roll-up |
| `pull_request_reopened` | Observation (R9) | SPEC-0003 | SPEC-0002 I26 |
| `executor_call_recorded` | Cost of one executor call (R45) | SPEC-0003 | SPEC-0002 turns and budgets, SPEC-0004 `wayloop cost`, roll-up |
| `result_rejected` | Model output refused (R21) | SPEC-0003 | SPEC-0002 retries (I28, I29) |
| `summary_recorded` | Summary rolled up (R36) | SPEC-0003 | Publication, resume pack |
| `summary_published` | Summary or hand-over published (R38) | SPEC-0003 | Publication, R50 |
| `publication_imported` | Publication read into a new clone (R55) | SPEC-0003 | Resume pack, identifiers, R50 |
| `snapshot_taken` | Snapshot anchor (R31) | SPEC-0003 | Loader |
| `torn_tail_discarded` | Integrity repair (R20) | SPEC-0003 | Loader, audit |
| `ledger_recovered` | Recovery by a person (R34) | SPEC-0003 | Loader, audit |
| `run_started` | Run of the runtime with its account | SPEC-0002 | SPEC-0002 R46 |
| `limits_set` | Approvers, limits, budgets, summary budgets | SPEC-0002 | SPEC-0002 R7; R37 |
| `policy_set` | Recorded policy and its revision | SPEC-0002 | SPEC-0002 R7, R17, R25 |
| `plan_observed` | Plan items and active sprint | SPEC-0002 | SPEC-0002 readiness and phase; milestone membership of the roll-up |
| `branch_changes_observed` | Changed paths at a head | SPEC-0002 | SPEC-0002 R25 |
| `phase_changed` | Project phase | SPEC-0002 | Status, audit |
| `gate_opened` | Approval state or question (kind `question`) | SPEC-0002 | SPEC-0002 gates; SPEC-0007 |
| `gate_resolved` | Gate resolved, including an approved plan change | SPEC-0002 | SPEC-0002 gates; SPEC-0007 R4 |
| `gate_withdrawn` | Gate withdrawn | SPEC-0002 | SPEC-0002 gates |
| `gate_waived` | Merge gate waived by the policy (ADR 0029) | SPEC-0002 | SPEC-0002 R17, R39; audit |
| `gate_bypassed` | Merge without approval | SPEC-0002 | Reconciliation |
| `approval_recorded` | Approval or rejection by a person (SPEC-0001 R17) | SPEC-0002 | SPEC-0002 gates |
| `question_answered` | Answer by a person | SPEC-0002 | SPEC-0002 gates and holds; roll-up |
| `input_ignored` | Observation with no effect, with its code | SPEC-0002 | Audit |
| `step_started` | Dispatch of a step | SPEC-0002 | SPEC-0002 dispatch and turns |
| `step_completed` | Step completed or skipped | SPEC-0002 | SPEC-0002 issue cycle |
| `step_retried` | Executor failure with retries left | SPEC-0002 | SPEC-0002 retries |
| `step_failed` | Retries of a step exhausted | SPEC-0002 | SPEC-0002 hold `executor_failure` |
| `attempt_failed` | Failed verification or required check, with the failure note | SPEC-0002 | SPEC-0002 attempts; SPEC-0006 R8 |
| `issue_blocked` | Hold added | SPEC-0002 | SPEC-0002 holds |
| `issue_unblocked` | Last hold removed | SPEC-0002 | SPEC-0002 holds |
| `subject_resumed` | A rejected brief taken up again by an approver (SPEC-0002 P9) | SPEC-0002 | SPEC-0002 R48 |
| `issue_done` | Issue cycle completed by a merge | SPEC-0002 | SPEC-0002; roll-up |
| `issue_superseded` | Issue cycle ended by a superseded task | SPEC-0002 | SPEC-0002; roll-up |
| `artifact_status_changed` | Status of an artifact | SPEC-0002 | SPEC-0002 phase and readiness |
| `budget_exhausted` | Budget scope exhausted | SPEC-0002 | SPEC-0002 R29; SPEC-0006 R6 |
| `budget_restored` | Budget scope raised again | SPEC-0002 | SPEC-0002 R29 |
| `reconciliation_required` | Inconsistent observation | SPEC-0002 | SPEC-0007 reconciliation |
| `coverage_planned` | Coverage units of an approved specification (R14) | SPEC-0003 | Coverage of R14; SPEC-0009 R11, R12 |
| `coverage_verdict` | One vote on one unit, with its evidence (R14, R15) | SPEC-0003 | SPEC-0002 R9, R49; coverage of R14 |
| `coverage_stale` | A covered unit with a cited file that a merge changed (R16) | SPEC-0003 | SPEC-0002 R50; SPEC-0009 R11 |
| `policy_evaluated` | Evaluation of the policy on a pull request | SPEC-0021 R7 | SPEC-0021 R8 |
| `waiver_used` | Waiver of a policy rule | SPEC-0021 R7 | SPEC-0021 R8 |

The catalogue holds 58 kinds; `subject_resumed` was added on 2026-11-25 (counter-review item
23). A stopped ledger is not an event: a broken chain cannot be
appended to, and SPEC-0002 reports it with the stop reason `ledger_broken`.

### Payloads

Payloads owned by this specification, version 1. Payloads are objects at most four levels deep
(ADR 0009 C4); lists are written `[]` when empty.

| Kind | Fields |
| --- | --- |
| `ledger_created` | `ledger_id` (string, UUID chosen at creation), `format_version` (1), `hash_algorithm` (`sha256`), `canonicalization` (`rfc8785`), `line_limit_bytes` (integer, at least 4,096), `min_engine_version` (semantic version), `previous_ledger` (optional `{ledger_id, last_hash}`, set when a ledger replaces an earlier one) |
| `min_engine_version_raised` | `min_engine_version`, `kind`, `schema_version` (the kind and version that require it) |
| `session_started` | `holder`, `executor` (snake_case name, as in the hand-over record), `mode` (`in_session` or `unattended`) |
| `session_ended` | `outcome` (`handed_over`, the holder ended the session with a hand-over, SPEC-0002 S2; `completed`, the holder ended a session that held no checkout, SPEC-0002 S3; `abandoned`, its lease ended, R43), `handover` (identifier, when `handed_over`) (`completed`: decided by the maintainer on 2026-11-25, counter-review item 41) |
| `checkout_acquired` | `holder`, `executor`, `branch` (valid under SPEC-0001 R11), `worktree` (name of the git worktree, never a path), `resumed_from` (optional hand-over identifier) |
| `lease_renewed` | none |
| `checkout_released` | `holder`, `reason`: `handed_over` (the session ended with a hand-over), `awaiting_checks` (the result of the `pull_request` step closed its dispatch, SPEC-0002 R35 and I44; it replaces `awaiting_merge`, which no released version holds: decided by the maintainer on 2026-11-25, SPEC-0002 Q15), `merged` (merged before it reached `merge`, SPEC-0002 I24), `pull_request_closed` (SPEC-0002 I25), `lease_expired` (R11) or `superseded` (the task was superseded); `lease_ended_at` (when `lease_expired`, computed as in section Clock times) |
| `handover_recorded` | `record`: the hand-over record of SPEC-0001, whose `recorded_at` equals that of the event; `late` (`true`, only for a late hand-over of R12) |
| `ruling_recorded` | `decision`, `reason`, `cost_if_wrong`, `question` (optional, the hash of the question gate it answers) |
| `ruling_revoked` | `ruling` (hash of the `ruling_recorded` event), `reason` |
| `pull_request_opened` | `number`, `branch`, `head_sha`, `title` |
| `pull_request_synchronized` | `number`, `branch`, `head_sha`, `previous_head_sha` |
| `pull_request_checks_completed` | `number`, `head_sha`, `suite` (identifier of the run of the checks), `checks` (list of `{name, conclusion}`, sorted by `name` in code point order, names unique; `conclusion` is `success`, `failure`, `cancelled` or `timed_out`) |
| `pull_request_review_submitted` | `number`, `head_sha`, `reviewer`, `state` (`approved`, `changes_requested`, `commented`) |
| `pull_request_merged` | `number`, `branch`, `head_sha`, `merge_commit`, `merged_at` (tracker time, UTC), `merged_by` |
| `pull_request_closed` | `number`, `branch`, `head_sha`, `closed_at` (tracker time, UTC), `occurrence` |
| `pull_request_reopened` | `number`, `branch`, `head_sha`, `reopened_at` (tracker time, UTC), `occurrence` |
| `executor_call_recorded` | `role`, `executor`, `model`, `effort`, `step`, `phase`, `model_turns` (turns of the model inside the call, a count; renamed from `turns` on 2026-11-25, counter-review item 56), `tokens_in`, `tokens_out` (provider tokens), `duration_ms`, `cost_source` (`provider_reported`, `price_table` or `none`), `cost` (`{cost_micros, currency}`, absent when `cost_source` is `none`) |
| `result_rejected` | `step`, `errors` (at most 20 objects `{code, pointer, reason}`), `errors_omitted` (count of further errors), `output` (`{file, sha256}`, absent when the output failed the secret check), `output_withheld` (the code of SPEC-0020, when it failed) |
| `summary_recorded` | `level`, the subject field of the level (`session`, `issue` or `milestone`), `file`, `sha256`, `tokens` (size tokens), `budget` (size tokens), `summary_schema_version` |
| `summary_published` | `level` (`session`, `issue`, `milestone` or `handover`), the subject field (`handover` for a hand-over), `sha256`, `target` (`issue_comment`), `issue` (the issue that holds the comment), `comment` (tracker identifier) |
| `publication_imported` | `level`, the subject field, `sha256`, `issue`, `comment`, `author`, `revision` (the revision the publication cites), `file` (where the document was written) |
| `snapshot_taken` | `snapshot_seq`, `sha256`, `state_schema_version`, `fold_version`, `file` |
| `torn_tail_discarded` | `bytes`, `sha256`, `file` (in the quarantine) |
| `ledger_recovered` | `reason` (`chain_broken` or `regressed`), `broken_seq` (optional), `last_kept_hash`, `file` (in the quarantine), `approved_by` (handle), `channel` (`terminal` or `tracker`), `sessions` (list of `{date, number}`, the highest session number of each date found in the quarantined lines), `handovers` (list of `{issue, number}`, the highest hand-over number of each issue found there) (R47, counter-review item 73) |
| `coverage_planned` | `specification` (identifier), `units` (unit identifiers of section Coverage and verification, sorted by code point, unique) |
| `coverage_verdict` | `unit`, `lens` (`presence`, `behaviour` or `proof`), `verdict` (`covered`, `uncovered` or `needs_validation`, as counted by the engine), `downgrade` (`uncited`, set only when the engine counted a vote as `uncovered` because a claim had no citation, SPEC-0002 R10), `citations` (list of `{role, file, line, end_line}`; `role` is `code` or `test`; `file` relative to the root of the project; lines from 1), `missing_fact` (for `needs_validation`, the fact the repository cannot establish), `commit` (the commit the verdict was produced on), `dirty` (whether the working tree had uncommitted changes) |
| `coverage_stale` | `unit`, `commit` (the merge commit of `pull_request_merged`, or the pushed commit of `default_branch_pushed`, on the default branch), `files` (the cited files that the merge changed, sorted by code point, unique) |

The event header carries the issue (`issue`) for every kind that concerns one. The pull request
events link their issue through the header `issue` and their branch: R9 requires both, and the
writer refuses a pull request event whose branch names another issue (SPEC-0001 R10).

`lease_ended_at` is the only time computed by the engine; it is derived from recorded times and
`limits.lease_minutes`, never from the clock.

### Contracts of the ledger

The event header, the payloads owned here, the summary and the snapshot are Rust types of
`wayloop-schemas`; their JSON schemas are generated from the types and committed as snapshots
with a history per contract (ADR 0030). The tables of this specification give the members and
their meaning; no JSON schema is written by hand.

- One header type, generic over the payload as `Envelope<D>` is over the data of a command:
  `Event<P>` holds the common fields of section Common fields and `payload: P`. The header is
  defined once, for every kind, whatever specification owns its payload.
- Registry `contract::CONTRACTS` (ADR 0030 G5): every kind of the catalogue is a contract of the
  area `events` named after its kind, `schemas/events/<kind>/v<N>.json`; the summary and the
  snapshot are the contracts `ledger/summary` and `ledger/snapshot`. `events` and `ledger` are
  new values of `Area`. The hand-over record stays the artifact contract of SPEC-0001
  (`artifacts/handover`). The commands that return events, summaries or rulings are contracts
  of the area `commands` (SPEC-0004) and reuse these types.
- Direction: the engine reads back every event, summary and snapshot it wrote, so their
  contracts are inputs: closed objects, an unknown member refused (ADR 0009 C5 as amended on
  2026-11-20). A command output that holds them uses the same types in the output direction,
  open, as ADR 0030 G2 provides.
- Versions: a version that no release holds may still change, as the amendment of ADR 0030 of
  2026-11-20 allows. From the first release that holds it, every change of an event, summary or
  snapshot contract is a new version, additive or not: an older engine would refuse an added
  member as `event_schema`, without the minimum engine version that R53 raises only for a new
  kind or version. This is stricter than rule (B) of ADR 0030 for these two areas (Q9), and
  is recorded by the amendment of ADR 0030 of 2026-11-22.
- Value forms: each form is one type of `wayloop-schemas`, used wherever it appears (ADR 0030
  G1): one SHA-256 type for `hash`, `prev_hash`, `input_hash`, `dispatch`, revisions and the
  `sha256` of files (today `InputHash`, which the implementing task generalises); `IssueNumber`;
  `ProjectPath` for repository paths; one type for paths relative to the git common directory;
  `Timestamp` for `recorded_at` and the other times, whole seconds of UTC; `SessionId` for
  session identifiers. Distinct types wrap the SHA-256 type where the meaning differs, so that one
  cannot be passed for another: `Revision` (the hash of the last event a sender read),
  `DispatchRef` (the hash of a `step_started`) and `GateRef` (the hash of a `gate_opened`);
  `ProviderTokens` and `SizeTokens` are two integer types with the units and the naming rule of
  SPEC-0002 section Limits and budgets (decided by the maintainer on 2026-11-25, counter-review
  item 57 and section 4, code pattern 5; the naming rule decided by the maintainer on
  2026-11-25, with the amendment). Integers are bounded by `SAFE_INTEGER` of the interpreter (R29), never in
  each schema (ADR 0030 G2).
- Absent, not null (ADR 0009 C2): an optional member is left out, a list is `[]`.
- Refusals and warnings are the `ErrorReport` and `Error` of `wayloop-schemas`, with codes as
  constants of `Code` and reasons from `Reason` (section Errors). `Error` gains an optional
  member `found`, absent when not given, with one meaning: the values that the refused condition
  found, in the input or in the state. Its shape is fixed per code, and the row of each code that
  carries it, in section Errors of this specification and of SPEC-0002, gives that shape; a code
  whose row gives none never carries it (one definition and a shape per code: decided by the
  maintainer on 2026-11-25, second counter-review S5). It is an additive change of
  `envelope/result` v1, which no release holds (decided by the maintainer on 2026-11-25,
  counter-review item 65, option (a)), recorded as an amendment of ADR 0009 C3.
- Location of a ledger line (Q10): `Source` of `wayloop-schemas` gains a fourth form,
  `{ledger, line}`, where `ledger` is the path of the segment relative to the git common
  directory and `line` counts from 1. `{file, line}` keeps its meaning, a path relative to the
  root of the project. The form is an additive change of `envelope/result` v1, which no release
  holds (SPEC-0004 R2); the change of the type belongs to the first ledger task that reports a
  line (#50).

### Schema versions and migration

- Each kind is one contract with its own versions, whose schema includes the common fields
  (section Contracts of the ledger). A change of the common fields is a new version of every
  kind.
- The versions follow section Contracts of the ledger, and the first event written in a new
  version raises the minimum engine version (R53).
- Migration is a pure function from version N to N+1 of one kind, applied in memory on read,
  after the hash of the stored line is checked (R8). Stored events are never rewritten: the hash
  chain covers the bytes as written.
- Snapshots and summaries are migrated the same way. A snapshot that cannot be migrated, or
  whose fold version differs from that of the engine (R54), is ignored, and the state is
  replayed from an earlier snapshot or from the genesis event. A summary file is checked against
  its event on the stored bytes before any migration (R41).
- A change of the hashing rule, of the canonicalisation, of the line format or of the line limit
  is not a schema migration: it is a new genesis event of a new ledger, which names the earlier
  ledger and its last hash in `previous_ledger` (R23).

## Hashing rule

1. Take the event as a JSON object and remove the member `hash`.
2. Serialise it with the canonical JSON of the core (`wayloop_core::canonical`, RFC 8785), the
   function that computes `meta.input_hash` (SPEC-0004 R11); integers are written in their
   canonical form (R29).
3. Compute the SHA-256 of the UTF-8 bytes with the SHA-256 of the core and write it in lower-case
   hexadecimal as `hash`.
4. Write the line as the canonical JSON of the whole event, `hash` included, followed by one
   line feed (`0x0A`). The stored line is therefore canonical, and every reader computes the
   same bytes on every platform.

Strings are UTF-8 on disk. A lone surrogate, in an escape or in the bytes, and a byte sequence
that is not UTF-8 are refused with the reason `syntax`. Numbers are read by value (R29): in a
payload to append, `1.0` and `1e3` are integers and are written `1` and `1000`, while a number
whose value is not an integer, such as `1.5`, is refused with the reason `type`. A stored line
that holds `1.0` or `1e3` is not canonical and is refused with the reason `syntax` (R59). `examples/canonicalization.vectors.json` holds the vectors of AC49: key order by
UTF-16 code units, with a key outside the Basic Multilingual Plane; escapes of control
characters; accented letters written as UTF-8; and the refused inputs.

Verification of a line, in this order, stopping at the first line that fails and reporting
every error of that line:

1. It ends with a line feed; the last line of the last segment without one is a torn tail (R20),
   and any other line without one is `ledger_line_malformed` with the reason `syntax` (decided by
   the maintainer on 2026-11-25, counter-review item 69).
2. Its size is within the line limit, checked before it is parsed (R59).
3. It is UTF-8, parses as JSON, holds only numbers whose value is an integer (R29) and equals
   its canonical form (R59).
4. Its `hash` is correct (R58).
5. Its `seq` is one more than the previous (R60); the first is the genesis event (R57).
6. Its `prev_hash` equals the `hash` of the previous line (R2).
7. Its `recorded_at` is not earlier than that of the previous line (`ledger_time_order`).
8. Its kind and version are known (R22), and it validates against their schema (R8).

The revision of SPEC-0004 R12 and ADR 0009 C6 is the `hash` of the last event. The SHA-256 of a
summary, a snapshot or another JSON document is computed on its canonical JSON, without the final
line feed of its file; the SHA-256 of any other blob, such as an executor output or a log, is
computed on its bytes as stored.

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
  quarantine/torn-<seq, 12 digits>-<sha256>.bin      a torn tail (R20)
  quarantine/                                         broken lines and regressed files (R34)
  index.sqlite                                        index, a cache (R26)
```

- The git common directory is shared by every worktree of a clone (ADR 0007, ADR 0013), so the
  orchestrator and every lane read one ledger. It is never committed.
- The ledger is the source of truth. The projection, the snapshots, the summary files, the
  hand-over files and the index are derived and can be rebuilt from the ledger; the summary
  files are checked against their events (R41).
- A segment is closed by a `snapshot_taken` event, which is its last line; the next event opens
  a new segment, and until it is written the last segment may be empty, which is valid (R63).
  The first segment is created by `wayloop init` from a temporary file holding the genesis
  event, flushed and renamed (R63). Closed segments are never written again and are kept uncompressed. Deleting
  closed segments is not part of M1 and is planned separately (Q1, Q13): in M1 every segment is
  kept (R1).
- Paths of ledger files in events and errors are relative to the git common directory, such as
  `wayloop/ledger/000000000000.jsonl`; repository paths (`files_touched`, citations) are relative
  to the root of the project. An absolute path never enters the ledger or an envelope.
- The projection is the YAML rendering of the object `{revision, seq, state}`: these three
  members in that fixed order, and the members of `state` in the canonical order of its JSON;
  block style, two-space indentation, every string double-quoted, line feeds, one final line feed
  (decided by the maintainer on 2026-11-25, counter-review item 67). The projection leaves out
  the map of `input_hash` values (which holds no `lease_renewal_requested`, R52; second
  counter-review check N-S4) and the set of `external_id` values, which grow with the
  ledger, and states their sizes in their place as `input_hashes_count` and
  `external_ids_count`; the state, the snapshots and replay keep them in full, and the slice
  measures the cost of the rewrite (decided by the maintainer on 2026-11-25, Q16, counter-review
  item 75).
- A snapshot file is `snapshots/<seq>.json`, named by the `seq` of the last event it includes,
  `{schema_version, fold_version, engine_version, seq, revision, state}` in canonical JSON. Its
  `revision` equals the `prev_hash` of the `snapshot_taken` event that anchors it. The file is
  written and flushed before that event; a file whose event was never written is ignored with
  the warning `snapshot_orphaned` (R31).
- Every derived file is written under the lock of the writer, to a temporary file in the same
  directory that is flushed and then renamed over the target, so that a reader sees either the
  old or the new file.

Supported storage (R51). The lock and the durability rules need a local file system. The engine
refuses, with exit code 3, a git common directory on a network file system, on a Windows drive
reached from WSL, on a container bind mount, or in a folder kept by a file synchronisation
client, as far as the platform reports the type of the mount or the attributes of the folder.
When the platform reports nothing, the check passes, and the guide says which setups are
unsupported.

## One writer and concurrency

- Decisions live in the core (ADR 0001, ADR 0006): it validates the input, assigns `seq`,
  `recorded_at` and the hashes, decides to append or refuse, folds the events into the state and
  returns the bytes to write. The runtime performs the I/O: it reads the clock, holds the lock,
  writes, flushes and calls the tracker.
- Inside one process, every worker sends its result to one writer task, which appends in order
  (SPEC-0006 R5).
- Between processes, such as two `wayloop` commands in two worktrees, the MCP server and an
  unattended run, the writer takes an exclusive whole-file lock on `ledger.lock` for each write:
  `flock` on Linux and macOS, `LockFileEx` on Windows. Record locks of `fcntl` are not used,
  because a process loses them when it closes any descriptor of the file. Under the lock the
  writer reads the tail, repairs a torn tail (R20), verifies and folds every event appended
  since its last known `seq`, by any process (R18, counter-review item 76), appends the due
  events (R28), checks the idempotency key (R52) and the expected revision (R19), decides,
  appends, flushes, writes the derived files and releases the lock. Every file operation goes
  through the storage interface of R62. The operating system releases the lock of a process that dies,
  so a crash never leaves a stale lock. A writer that waits longer than `lock_timeout_seconds`
  fails with the code `ledger_locked` (exit code 3, ADR 0009 C7).
- Flushing to stable storage means `fsync` of the segment on Linux, `fcntl(F_FULLFSYNC)` on
  macOS, where `fsync` does not reach the disk, and `FlushFileBuffers` on Windows; after a new
  segment or a renamed file is created, the directory is flushed too on Linux and macOS.
- The lock serialises writes; the expected revision detects a decision taken on an older state;
  the lease of a session stops two agents from working on one issue; the idempotency key makes
  a retried command safe.
- An event is acknowledged only after its line is flushed to stable storage. A command
  interrupted before the acknowledgement may still have left a complete line; its retry finds the
  `input_hash`, in the same session or, for a command without a session, in the ledger, and
  appends nothing (R52).
- An effect on the tracker happens before the event that records it, and is idempotent: a crash
  between the two repeats the effect, which the marker of R38 or the `external_id` of R30 makes
  harmless.

## Replay and snapshots

- The state is a fold of the events from the genesis event with the function `apply` of
  SPEC-0002. Replay never reads a clock, a file other than the ledger, or the tracker.
- The replay test of this repository is the golden ledger and its golden projection, replayed
  in CI on Linux, macOS and Windows (AC21), plus the property tests AC1, AC28 and AC33. The
  gate of milestone M1 is AC21.
- A snapshot is taken after every `snapshot_every` events since the previous one, and on
  `wayloop ledger snapshot`. Loading uses the latest snapshot whose hash matches its event and
  whose fold version is that of the engine (R31, R54), verifies the events after it (R32), and
  folds them.
- Replay equivalence: the state loaded from any valid snapshot plus its tail is byte-identical
  to the state of a full replay (AC28).

## Integrity: detection, report and recovery

The chain detects accidental corruption and naive edits: a changed byte, a removed, reordered or
inserted line, a truncation inside the file. It does not prove who wrote the ledger: anyone who
can write the file can change an event and compute every later hash again, as
`ledger.rehashed-line.invalid.jsonl` shows for the line after the edit. Such a rewrite, or the
replacement of the ledger by an older copy, becomes visible when the derived files or the
published summaries cite revisions that the chain does not hold (R49, R50).

| Condition | Code | Reason | Effect |
| --- | --- | --- | --- |
| No ledger | `ledger_missing` | `missing` | Command refused, exit code 3 (R63) |
| Partial last line | `ledger_torn_tail` (warning) | `syntax` | Ignored by readers; quarantined and recorded by the writer (R20); never acknowledged, so nothing is lost |
| Line over the limit | `ledger_line_malformed` | `length` | Broken chain |
| Line not UTF-8, not JSON or not canonical, such as a stored `1.0` or `1e3`, or a line other than the last without a final line feed | `ledger_line_malformed` | `syntax` | Broken chain |
| Number whose value is not an integer, such as `1.5` | `ledger_line_malformed` | `type` | Broken chain |
| Stored `hash` wrong | `ledger_hash_mismatch` | `conflict` | Broken chain |
| `seq` not one more than the previous | `ledger_sequence_gap` | `range` | Broken chain |
| `prev_hash` not the previous `hash` | `ledger_chain_broken` | `conflict` | Broken chain |
| First event not genesis, or a second genesis | `ledger_genesis` | `missing`, `conflict` | Broken chain |
| `recorded_at` earlier than the previous | `ledger_time_order` | `range` | Broken chain |
| Derived file or publication ahead of the chain | `ledger_regressed` | `conflict` | Appends refused (R49) |

On a broken chain or a regressed ledger the engine refuses every append (R4, R49), answers reads
from the valid prefix with a warning (R33), and the loop stops with the stop reason
`ledger_broken` of SPEC-0002, which the loader passes to `next_steps` as its `integrity`
(SPEC-0002 R5). The report carries `source` `{ledger, line}` with the segment and the line
(section Contracts of the ledger), the pointer into the event, the `expected` value, and in
`found` the value found, `seq` and the revision of the last valid event (R4).

Recovery (R34). Only a person recovers, through a channel of section Person-only commands:

1. The command shows the last event it will keep, with its `seq`, kind, `recorded_at` and hash,
   and the number of lines it will move.
2. The person confirms with the hash of that event (`--confirm <hash>`, or in the approval on
   the tracker). A confirmation that names another hash is refused, so that a recovery never
   runs on a ledger that changed since it was shown.
3. Every line from the first broken event onward is moved to the quarantine, together with every
   later segment and every snapshot after the break. A break found by the full verification in a
   closed segment is treated the same way: the later segments are quarantined too, because their
   chain starts from the broken line.
4. `ledger_recovered` is appended after the last kept event, with `prev_hash` equal to its hash;
   the derived files are rebuilt from the ledger.
5. Published summaries whose revision is now in the quarantine are reported with the warning
   `publication_orphaned` and published again at the next publication step.

A regressed ledger is recovered the same way with the reason `regressed`: the person confirms
the last event of the ledger as it is, the derived files that are ahead are moved to the
quarantine, and the derived files are rebuilt.

## Person-only commands

`wayloop ledger recover` and `wayloop init` are decisions of a person. The deletion of closed
segments is not part of M1 and is planned separately (Q13). The engine accepts these commands
only through these channels:

- An interactive terminal: the command is not exposed through the MCP server, the SDKs or the
  tool definitions of agents (ADR 0024); it requires a terminal on its standard input and the
  confirmation of R34; it records the handle the person gives, which must be a recorded
  approver, with `channel` `terminal`. The terminal does not authenticate the handle: the
  channel keeps the decision out of reach of agents, and the record names who claimed it.
- A tracker approval: the engine posts the request, with the last kept event, on the tracking
  issue of the ledger, and runs it when a recorded approver approves it there with the hash;
  the handle is the one the tracker authenticates, with `channel` `tracker`.

A command of this kind from a session of an agent is refused with the reason `state`.

## Clock times

- Events carry clock times: `recorded_at` set by the writer (R27), and times reported by the
  tracker such as `merged_at`. They are data, covered by the hash, and replay reads them; it
  never compares them with the current clock.
- The core never reads a clock. The runtime passes the current time to one function,
  `due_events(state, now)` of SPEC-0002, which returns the events due by time: the releases of
  R11, the session ends of R43 and the `summary_recorded` events of R36 that those ends cause,
  in the order of SPEC-0002 section Ready work and lanes. Every command that changes state appends them first, under
  the lock (R28); a command that only reads never calls it and never appends.
- The lease of a session, whether or not it holds a checkout, ends at the `recorded_at` of the
  latest event of that session that extends it, plus `limits.lease_minutes` minutes of the latest
  `limits_set` (R46); a session that holds no checkout cannot renew it (SPEC-0002 S4) and ends as
  `abandoned` at that time (R43, second counter-review B12); its length and
  default are those of SPEC-0002 section Limits and budgets. The events that extend it are those
  caused by a command or a result of the session, `lease_renewed` included; the events of
  `due_events`, the integrity events and the events caused by observations do not, even when
  their header names the session (decided by the maintainer on 2026-11-25, counter-review item
  82, with the amendment). While a step runs, the runtime of the holder sends `lease_renewal_requested` every quarter
  of the lease, without an expected revision (SPEC-0002 S4), and its `lease_renewed` extends it
  when nothing else happens; every heartbeat is a new renewal, never a retry of R52 (second
  counter-review B8). This is the one definition of the lease; SPEC-0002 R22 applies it.
- A given time earlier than the last `recorded_at` is raised to it (R27). A clock behind by more
  than the lease would extend leases that already ended, so the append is refused with
  `clock_behind`; before the first `limits_set` the default lease of 120 minutes applies
  (counter-review item 81).
- Envelopes never hold the clock time of the call (ADR 0028, SPEC-0004 R15 as amended on
  2026-11-22, Q11). A result may hold the recorded times of the state, such as the `recorded_at`
  of recorded events or summaries and the `merged_at` of a pull request: those values are part of the state at
  `meta.revision`, so the same input on the same revision gives the same envelope at any time
  (AC25). Durations are computed between recorded times, never as an age relative to now.

## Sessions, checkouts and hand-overs

- A session starts with `session_started` and ends with `session_ended`. Its identifier follows
  R47; the short ledger prefix keeps identifiers of two clones apart in the publications on the
  tracker, and the number continues from the highest used on that date in the ledger, the
  quarantine and the publications, so that a recovery or a new clone never reuses one.
- Hand-over numbers `HANDOVER-<issue>-<k>` continue the same way: `k` is one more than the
  highest found in the ledger, the quarantine and the publications of the issue.
- A session holds zero or more checkouts. A checkout ends with `checkout_released` and one of
  the reasons listed in section Payloads (R46). A session that holds no checkout ends with the
  outcome `completed` when its holder ends it (SPEC-0002 S3, counter-review item 41), and with
  the outcome `abandoned` at the end of its lease (SPEC-0002 S6, R43; second counter-review B12).
- A late hand-over (R12) is accepted from the last holder after its lease ended, while no other
  session has claimed the issue. The session stays `abandoned`; the hand-over is recorded with
  `late` and the issue summary is rolled up again.

## Summaries and roll-up

### Content

A summary has one schema, `schemas/ledger/summary/v<N>.json`, with a `level`. The examples are
the `summary.*.valid.json` files of `examples/`.

| Name | Type | Levels | Meaning |
| --- | --- | --- | --- |
| `schema_version` | integer | all | Version of the summary schema |
| `level` | `session`, `issue`, `milestone` | all | Level |
| `session`, `issue`, `milestone` | subject fields | as applicable | Subject, plus the issue and milestone a session or an issue belongs to |
| `task` | `TASK` identifier | issue, optional | Task of the issue |
| `outcome` | session: `handed_over`, `completed`, `abandoned`; issue: `open`, `merged`, `closed_without_merge`, `superseded`; milestone: `open`, `done` | all | Result |
| `as_of` | `recorded_at` of the event of `revision` | all | Time of the summary, from the ledger |
| `revision` | hash of the last event that is not a summary event among the events recorded for the input that triggered the roll-up | all | Point of the ledger summarised; the summary covers every event up to it |
| `handover` | hand-over identifier | session, when `handed_over` | Source of the session |
| `next_step` | string | session, when it has a hand-over; issue, when a hand-over exists and the issue is open | From that hand-over |
| `open_questions`, `not_done` | lists of strings | all | Session: from its hand-over; issue: from its latest hand-over while it is open; milestone: from the latest hand-over of each open issue |
| `pull_requests` | list of `{number, state, head_sha, merge_commit, merged_at}` | all | `state` is `open`, `merged` or `closed` |
| `rulings` | list of `{decision, reason, cost_if_wrong, event, actor}`, plus `issue` at milestone level | all | Active rulings; `event` is the hash of the ruling event; `actor` is its `actor.id` |
| `sessions` | list of session identifiers | issue | Sessions of the issue |
| `issues` | list of `{issue, outcome}` | milestone | Issues of the milestone |
| `files_touched` | list of strings | session, issue | Paths of the hand-over of the session, or of the hand-overs of the issue, without duplicates |
| `cost` | `{tokens_in, tokens_out, duration_ms, executor_calls, calls_without_cost, amounts}`; `amounts` is a list of `{currency, cost_micros}` sorted by currency | all | Totals of R45; amounts are never summed across currencies |
| `omitted` | object from a section name to the count of dropped entries | all | Entries left out by the fill rule (R35); `{}` when none |
| `tokens` | integer | all | Size of the summary: bytes of the canonical JSON of the summary without the member `tokens`, divided by four, rounded up (SPEC-0001 Q6) |

The issue summary is built from the session summaries of the issue and the pull request events;
the milestone summary from the issue summaries of the milestone. A milestone with 200 sessions
is therefore summarised from its issues, never from its sessions. The milestone of an issue is
the milestone of its task in the latest `plan_observed`, which the plan adapter reads from the
tracker; in a new clone it comes from the tracker in the same way.

### Roll-up

The fill rule (R35) is deterministic; no model writes a summary (Q4).

1. Write the fixed part: every field except the sections below.
2. Reserve the size of `omitted` as if every section were dropped whole: the budget left for
   entries is the budget minus the size of the fixed part with that `omitted`. The written
   `omitted` lists fewer or smaller counts, so it always fits.
3. Fill the sections in this order: `open_questions`, `not_done`, `pull_requests`, `rulings`,
   `issues` or `sessions`, `files_touched`. Entries are ordered by the `seq` of the event they
   come from, newest first, and in the order of their source list when the event is the same.
   At milestone level, the source event of an entry is the `summary_recorded` of the latest
   summary of its issue, and the entries of one issue keep the order of that summary.
4. `files_touched` takes the paths of the hand-overs, newest hand-over first; a path already
   listed is skipped and is not counted in `omitted`.
5. Add an entry only when the summary still fits the budget with it. When an entry does not fit,
   the rest of that section is dropped and counted in `omitted`, and the next section is filled.
6. Write `omitted` with the sections whose count is positive, then compute `tokens` last.

The fixed part fits by construction: the configuration check of R37 keeps the session and issue
budgets above the hand-over budget, which bounds `next_step`, plus 256 tokens for the other
fixed fields.

### Budgets

| Member of `ledger_budgets` | Default, size tokens | Bounds |
| --- | --- | --- |
| `handover` | 1,200 | Hand-over record (R12) |
| `session` | 1,500 | Session summary |
| `issue` | 2,000 | Issue summary |
| `milestone` | 4,000 | Milestone summary |

`wayloop init` writes the defaults to `[ledger.budgets]` of `wayloop.toml` and the first
`limits_set` records them; later changes reach the ledger under SPEC-0002 R45. The resume pack of
an issue (R44) is therefore at most the sum of the issue, milestone and hand-over budgets, in
size tokens; its text is part of the input of the executor call that reads it, so the provider
tokens it costs are part of `tokens_in` and count against the budget of the step kind, sprint
or milestone (SPEC-0002). Size tokens and provider tokens are never converted into each other
(SPEC-0002 section Limits and budgets). Other settings: `line_limit_bytes` (default 65,536), fixed for
the life of the ledger by its genesis event; `snapshot_every` (default 10,000 events) and
`lock_timeout_seconds` (default 10) in `[ledger]` of `wayloop.toml`, which change no recorded
content. The lease is in section Clock times.

### Publication on the tracker

ADR 0003 makes the summaries on the issues the durable memory of the project. The core renders
what to publish; the tracker adapter writes it (SPEC-0005); the event records it (R38).

| What | When | Where |
| --- | --- | --- |
| Hand-over with the cost of its session | At the end of a session with a hand-over, and for a late hand-over | New comment on the issue (SPEC-0007 R5) |
| Session summary | At the end of an abandoned session | New comment on the issue |
| Issue summary | When the issue merges, its pull request closes or its task is superseded | One comment on the issue, replaced when its hash changes |
| Milestone summary | When an issue of the milestone merges or its pull request closes | One comment on the tracking issue of the milestone, replaced when its hash changes (Q3) |

A published body has three parts: a first line `<!-- wayloop:summary level=<level>
subject=<subject> sha256=<hex> -->`, where `level` is `session`, `issue`, `milestone` or
`handover`; the rendering in Markdown, with English headings and the prose of the project; and
the canonical JSON in a collapsed section. The rendering is a pure function of the JSON.
Publication is skipped when a comment with the same marker and hash exists, and a failed
publication stays pending in the state and is retried at the next publication step. A dry run
lists the planned publications and writes nothing (SPEC-0004 R8).

Reading back (R39) accepts only a comment authored, according to the metadata of the tracker,
by the configured account of the runtime or by a recorded approver. Text in a comment written by
anyone else is never read as a summary, whatever its marker.

### Import into a new clone

A new clone has a new ledger. `wayloop init` reads the published hand-overs and summaries of the
open issues and milestones, keeps those that pass R39, writes each to its file, and records
`publication_imported` (R55). Resume packs then start from them, and the numbers of sessions and
hand-overs continue after them (R47). Events of the lost ledger are not imported: the
publications are the memory that survives a machine (ADR 0003).

## Security rules

Content in the ledger is data, never instructions (constitution IV).

- Secret check (R40). Every payload is checked before it is appended, every executor output
  before it is stored as a blob, and every rendered summary before it is published. A command or
  a result that fails is refused, and a rejected output that fails is not stored: its event
  names the finding in `output_withheld`. An observation that fails is a fact that happened, so
  it is recorded with each failing string replaced by `redacted:<code>`, where `<code>` is the
  code of SPEC-0020.
- Paths (R56). `files_touched`, `paths` and every other recorded path are relative, use `/`,
  hold no `..` segment, and are in Unicode NFC; a path in another form is refused with the
  reason `syntax` and the NFC form in `fix`.
- Rendering on the tracker (R56). In rendered text, a sequence that starts a marker of R38 is
  escaped; the canonical JSON sits in a code fence longer than the longest run of backticks it
  holds; a tracker handle is written inside a code span, so that a summary never notifies a
  person; the rendering adds no link that the JSON does not hold.
- Resume pack (R44). The brief holds the resume pack as quoted data, in a delimited block that
  names the author of each part (the holder of a hand-over, the engine for a summary) and its
  identifier. The brief never places it in its instruction part, and no text of the pack can
  change the role or the instructions of the agent.
- Rulings carry the actor that recorded them, in the event and in the summaries, so that an
  audit tells a ruling of an agent from a decision of a person.

## Index

The SQLite index of ADR 0007 holds events, rulings, hand-overs and summaries with full-text
search. The engine bundles its own SQLite library with full-text search, so that the index
behaves the same on every platform. The index records the `(seq, hash)` of the last event it
holds; before a query, the engine upserts the events after it, keyed by `seq`, so that a repeated
upsert changes nothing, and rebuilds the index when that hash is not the hash of that `seq` in
the chain, as after a recovery (R26). Agents reach it only through `wayloop history`,
`wayloop rulings` and `wayloop find` and the MCP tools generated from them, within the output
budget of SPEC-0004 R6. No transition reads it. The code index of the survey is SPEC-0010.

## Errors

Every refusal is the error report of ADR 0009 C3 as amended, with the field `source` of
SPEC-0001, Errors, built from the types of `wayloop-schemas`: every code below is a constant of
`Code` and every reason a value of `Reason`, spelt nowhere else (ADR 0030 G1). Pointers follow
RFC 6901 from the root of the event, the summary or the snapshot of the line or file in error,
and for a command from the root of the input sent, or of the document given by `document` when
the input holds a list `documents` (SPEC-0004 R2). A new meaning gets a new code; codes are
never renamed or reused (ADR 0009 C8). A code used here and defined by SPEC-0002 is listed with
a reference, not defined again.

Severity: `refusal`, the command, the append or the load is refused; `recorded`, the code is
carried by an event (`result_rejected`) because a result is not refused; `warning`, reported in
`diagnostics.warnings` while the operation goes on; `readiness`, reported for an issue that is
not ready (SPEC-0002 R19).

| Requirement | Code | Reasons | Severity |
| --- | --- | --- | --- |
| R1, R5, R9, R29, R45, R56 | `event_schema` | `missing`, `unknown_field`, `type`, `enum`, `length`, `range`, `syntax`, `requires` | refusal |
| R2, R4 | `ledger_chain_broken` | `conflict` | refusal |
| R57 | `ledger_genesis` | `missing`, `conflict` | refusal |
| R58 | `ledger_hash_mismatch` | `conflict` | refusal |
| R59, R29 | `ledger_line_malformed` | `syntax`, `length`, `type` | refusal |
| R59 | `event_too_large` | `length` | refusal for a command; recorded (`result_rejected`) for a result |
| R60 | `ledger_sequence_gap` | `range` | refusal |
| R63 | `ledger_missing` | `missing` | refusal, exit code 3 |
| R18 | `ledger_locked` | `state` | refusal, exit code 3 |
| R19 | `revision_stale` | Defined by SPEC-0002 (R41) | refusal |
| R17 | `verdict_commit_stale` | `stale` | recorded (`result_rejected`) |
| R17 | `verdict_tree_dirty` | `state` | recorded (`result_rejected`) |
| R21 | `result_schema` | Defined by SPEC-0002 (R23; second counter-review B7) | recorded (`result_rejected`) |
| R20 | `ledger_torn_tail` | `syntax` | warning |
| R22 | `event_kind_unknown`, `event_version_unknown` | `enum` | refusal |
| R23 | `ledger_format_unknown` | `enum` | refusal |
| R25 | `projection_mismatch` | `conflict` | warning |
| R27 | `ledger_time_order` | `range` | refusal |
| R27 | `clock_behind` | `range` | refusal |
| R30 | `event_duplicate` | `duplicate` | warning |
| R31, R54 | `snapshot_mismatch` | `conflict` | warning |
| R31 | `snapshot_orphaned` | `missing` | warning |
| R33 | `ledger_broken_prefix` | `state` | warning |
| R34 | `recovery_refused` | `state`, `conflict` | refusal |
| R34 | `publication_orphaned` | `state` | warning |
| R37 | `ledger_budget_config` | `range` | refusal |
| R39 | `publication_mismatch` | `conflict` | warning |
| R39 | `publication_author` | `requires` | warning |
| R41 | `summary_mismatch` | `conflict` | warning |
| R42 | `ruling_unknown` | `missing` | refusal |
| R10 | `checkout_held` | `conflict` | refusal, readiness |
| R12 | `handover_budget` | `length` | refusal |
| R48 | `checkout_not_held` | `state` | refusal |
| R49, R50 | `ledger_regressed` | `conflict` | refusal of every append |
| R51 | `ledger_storage_unsupported` | `state` | refusal, exit code 3 |

R40 uses the codes of SPEC-0020. R52 returns the recorded result, not an error. The member
`found` of the error (section Contracts of the ledger) has these shapes, one per code, and no
other code of this table carries it (second counter-review S5):

| Code | `found` |
| --- | --- |
| Every code whose effect is "Broken chain" in section Integrity (R4) | `{value, seq, revision}`: the value found at the pointer, absent when the line does not parse; the `seq` of the line as stored, absent when it cannot be read; and the revision of the last valid event, absent when no event before the line is valid |
| `checkout_held` (R10) | `{holder, lease_ends_at}`: the holder of the checkout and the end of the lease of its session, a `Timestamp` |
| `handover_budget` (R12) | The measured size, in size tokens, an integer |
| `checkout_not_held` (R48) | The holder of the checkout of the issue, or of the session named, a handle; absent when the issue has no checkout |

## Examples

`examples/` holds fixtures computed with the hashing rule above, and `.gitattributes` keeps
their bytes as written on every platform. The ledgers form one story: issue 12 worked in two
sessions by two executors, a ruling, a failed and a passed check run on two heads, the wait at
`merge`, a review, the merge, the summaries and their publications. They hold the kinds owned by
this specification plus the `limits_set` and `plan_observed` of SPEC-0002 that the leases, the
budgets and the milestone membership read; the other kinds of SPEC-0002, such as the steps and
the gates, change no summary and are left out.

The ledger fixtures were written before the amendments of 2026-11-25 and contradict them
(second counter-review B11): in `ledger.valid.jsonl`, line 2 carries `budgets.intake`, which is
removed, and lacks the `policy_set` with `project_name` that `wayloop init` records (SPEC-0002
`genesis`); line 3 carries the external identifier `plan:<sha>:1` without
`previous_plan_sha256`, where the form is now `plan:<previous>:<sha>:<part>`; lines 6, 9 and 20
carry `turns`, now `model_turns`; line 23 carries the reason `awaiting_merge`, which is removed,
and a release caused by `check_suite:5002`, where SPEC-0002 I44 now releases the checkout at the
result of the `pull_request` step; and the file holds `ruling_recorded`, `summary_recorded` and
`summary_published`, which 0.1.0 does not register. Every ledger derived from it inherits these
lines. Until task T3 of `specs/0002-state-machine/tasks.md` regenerates them with the hashing
rule, to the amended payloads and with the kinds that 0.1.0 registers only, and states the new
revision in AC56 and the new line numbers in AC2, AC10, AC11, AC15, AC50, AC58 and in the table
below, the ledger fixtures, their line numbers and the revision of AC56 are not normative
(decided by the maintainer on 2026-11-25, second counter-review B11). The regenerated ledger keeps
a line with accented letters stored as UTF-8. The summary fixtures and the lines that cite them
are rewritten by the task that builds the summaries (#52).

| File | Expected result |
| --- | --- |
| `ledger.valid.jsonl` | Valid; 32 events; revision of AC56; no lease ends between two events of one session (not normative until T3 regenerates it) |
| `ledger.edited-line.invalid.jsonl` | Line 5 payload changed without a new hash: `ledger_hash_mismatch` at line 5 |
| `ledger.rehashed-line.invalid.jsonl` | Line 5 changed and hashed again: `ledger_chain_broken` at line 6, `pointers` naming line 5 |
| `ledger.sequence-gap.invalid.jsonl` | Line 7 removed: `ledger_sequence_gap` at line 7, expected 6 |
| `ledger.non-canonical.invalid.jsonl` | Line 3 with white space: `ledger_line_malformed`, `syntax`, line 3 |
| `ledger.time-order.invalid.jsonl` | Line 9 with a `recorded_at` earlier than line 8, hashed again with the lines after it: `ledger_time_order` at line 9 |
| `ledger.torn-tail.invalid.jsonl` | Line 32 cut without its line feed: ignored by readers; `ledger_torn_tail` warning and 31 valid events after the repair |
| `summary.session.handed-over.valid.json` | Session summary with a hand-over; its SHA-256 is in line 15 |
| `summary.issue.handed-over.valid.json` | Issue summary at the end of that session; line 16 |
| `summary.session.completed.valid.json` | Session that ended while the issue waited at `merge`, without a hand-over; line 25 |
| `summary.issue.awaiting-merge.valid.json` | Issue summary at the end of that session; line 26 |
| `summary.issue.merged.valid.json` | Issue summary after the merge; line 29, published in line 31 |
| `summary.milestone.valid.json` | Milestone summary of M1; line 30, published in line 32 |
| `canonicalization.vectors.json` | Vectors of AC49: inputs, canonical forms and hashes, and refused inputs with their reason |

The hand-over is published in line 17. Line 7 holds accented letters, stored as UTF-8.

## Alignment with SPEC-0002

SPEC-0002 and this specification share the catalogue above. In particular:

- the state changes only by `apply` of recorded events; `decide` is never called in a replay;
- an issue is claimed by `checkout_acquired` and released by `checkout_released`, with the
  reasons of section Payloads; the end of a lease is found only by `due_events`;
- every event caused by an input carries its cause: `input_hash`, `dispatch` or `external_id`;
- `result_rejected` counts as one retry of the step, and `step_failed` records the exhaustion of
  the retries; repeated failures of checks are counted by `attempt_failed`;
- approvals and answers arrive as `approval_recorded` and `question_answered` with a person as
  actor; questions are gates of kind `question`; an approved plan change is a `gate_resolved`;
- staleness of an expected revision is scoped as in SPEC-0002 section Revisions;
- a broken or regressed ledger is the stop reason `ledger_broken`;
- the votes of `review` are recorded as `coverage_verdict` and tallied by SPEC-0002 R9; the stale
  units of R16 are recorded by SPEC-0002 R51 when a merge is observed and listed in the ready
  work by SPEC-0002 R50.

## Interfaces
- Ledger files under `<git common dir>/wayloop/` with the layout of section Storage. New.
- Contracts: the header `Event<P>` and the payloads owned here, the summary and the snapshot, as types of `wayloop-schemas` registered in `contract::CONTRACTS` under section Contracts of the ledger (`events/<kind>`, `ledger/summary`, `ledger/snapshot`); snapshots and their history are generated (ADR 0030). New.
- New event kinds of #309: `coverage_planned`, `coverage_verdict` and `coverage_stale`, schema version 1, in the catalogue.
- Configuration: `[ledger]` and `[ledger.budgets]` of `wayloop.toml` (ADR 0010), keys of section Budgets; the budgets and the lease reach the ledger through `limits_set` (SPEC-0002). New.
- Commands, to be added to SPEC-0004: `wayloop history`, `wayloop rulings` (existing), `wayloop ledger verify [--full]`, `wayloop ledger snapshot`, `wayloop ledger recover --confirm <hash>` (person only, section Person-only commands), `wayloop index rebuild`, `wayloop checkout`, `wayloop handover`, `wayloop summary <level> <subject>`. All results that read state report `meta.revision` (SPEC-0004 R12).
- Error codes of section Errors. New.
- Every addition is additive. No released contract changes.

## Dependencies
- SPEC-0001 is Approved: the hand-over record, its identifier and `tokens`, and the field `source` of errors come from it.
- SPEC-0002 owns `decide`, `apply` and `due_events`, the payloads of its kinds in the catalogue, the scoped rule of revisions (R41), the input classes (R4), the ignored duplicate observation (R33), the review by risk tier and the tally of votes (R8 to R11, R49), and the defaults of the lease and the limits.
- ADR 0030 (Accepted): contracts generated from the types of `wayloop-schemas`; this specification adds the areas `events` and `ledger` to the registry.
- SPEC-0004 R2, R14 and R18 as amended: the error report with `document`, the entry point, and `error` without `data`.
- SPEC-0006 R11 reserves the invocations of the verifiers; SPEC-0009 R9 to R12 use the coverage recorded here.
- SPEC-0004 R11, R12 and R15 fix `input_hash`, the revision and the absence of the clock time of the call in envelopes; this pull request amends R2 (the form `{ledger, line}` of `source`) and R15 with AC11 (recorded times of the state allowed).
- SPEC-0009 owns the re-anchoring of cited ranges along merges, which would replace the file rule of R16 (its open questions).
- SPEC-0020 R4 provides the secret check called by R40; until SPEC-0020 is approved, R40 cannot be verified, and AC38 waits for it.
- SPEC-0005 performs the tracker calls of publication and reports the author of a comment.
- ADR 0013 is Proposed; the worktree name in a checkout follows it.

## Plan and tasks
The smallest vertical slice of the issue cycle, decided by the maintainer on 2026-11-25
(counter-review section 4), is planned once, in `specs/0002-state-machine/plan.md` and
`tasks.md`: its ledger tasks implement #50 and #51 in part, with the minimal parts of #161 and
#162. Summaries (#52), snapshots, migration and recovery follow it.

## Open questions
- [x] **Q1. Retention of closed segments after a snapshot.** Decided by the maintainer on 2026-11-17: (a), every segment is kept uncompressed and deleted only on an explicit command of a person, recorded as an event. Narrowed by the maintainer on 2026-11-22 (Q13): deletion is not part of M1 and is planned separately; in M1 every segment is kept.
- [x] **Q2. Recovery of a broken chain.** Decided by the maintainer on 2026-11-17: (a), `wayloop ledger recover` run by a person, with the confirmation of the last kept hash, and with breaks in closed segments treated by quarantining every later segment (R34, section Integrity).
- [x] **Q3. Where the milestone summary is published.** Decided by the maintainer on 2026-11-17: (b), one comment on the tracking issue of the milestone, on the same path as an issue summary.
- [x] **Q4. Who writes summaries.** Decided by the maintainer on 2026-11-17: (a), the deterministic fill rule of section Roll-up, with prose only from hand-overs.
- [x] **Q5. Default values.** Decided by the maintainer on 2026-11-17: (a), the proposed defaults, with the heartbeat every quarter of the lease and the line limit fixed in the genesis event; to be revised with the measures of the benchmark (SPEC-0019).
- [x] **Q6. Verification on load.** Decided by the maintainer on 2026-11-17: verify the events after the latest snapshot on every load, and every segment when a session starts and with `wayloop ledger verify --full`. Load time stays bounded by `snapshot_every`, and an edit before the snapshot is found before the next session acts on the state, in either mode of the runtime, not only when `wayloop run` starts (R32).
- [x] **Q7. Identity of a ruling.** Decided by the maintainer on 2026-11-17: (a), the hash of its `ruling_recorded` event, with no new identifier type.
- [x] **Q8. Repair of a torn tail.** Decided by the maintainer on 2026-11-17: (a), automatic, by the writer only, under its lock; readers ignore an unterminated last line (R20).
- [x] **Q9. Version rule of the event, summary and snapshot contracts after a release.** Decided by the maintainer on 2026-11-22: (a), every change of an event, summary or snapshot contract after the first release that holds its version is a new version, so that R53 raises the minimum engine version and an older engine stops cleanly; recorded by the amendment of ADR 0030 of 2026-11-22 for the areas `events` and `ledger` (section Contracts of the ledger).
- [x] **Q10. Which `source` locates a ledger line?** Decided by the maintainer on 2026-11-22: (a), a fourth form of `source`, `{ledger, line}`, with the path of the segment relative to the git common directory, an additive change of `envelope/result` v1, which no release holds; SPEC-0004 R2 lists it, and the change of the type belongs to the ledger tasks (section Contracts of the ledger).
- [x] **Q11. Recorded times in an envelope.** Decided by the maintainer on 2026-11-22: (a), SPEC-0004 R15 and AC11 are amended in this pull request to forbid only the clock time of the call; recorded times of the state, such as `recorded_at` and `merged_at`, may be returned (section Clock times, AC25).
- [x] **Q12. How do the changed lines of a merge reach the core (R16)?** Decided by the maintainer on 2026-11-22: (b) for M1, the git adapter reports the changed paths and a unit is `stale` when a cited file changes (R16, AC8); changed ranges re-anchored along merges arrive with SPEC-0009, where they are recorded as an open question.
- [x] **Q13. How is a closed segment deleted?** Decided by the maintainer on 2026-11-22: deletion of closed segments is not part of M1 and is planned separately (#316). R1 is unchanged: no acknowledged line is deleted in M1, every segment is kept, and no event kind is added (section Storage, section Person-only commands; this narrows Q1).
- [x] **Q14. Are commands without a session idempotent?** Decided by the maintainer on 2026-11-22: the same rule as for a command with a session: a retry with the same `input_hash` and the same `expected_revision` returns the result of the recorded events; for a command without a session the key is looked up in the whole ledger (R52, AC62). Completed by the maintainer on 2026-11-25 (counter-review item 63): the state maps every `input_hash` to the `seq` of its first and last event.
- [x] **Q15. What happens to an observation whose event would exceed the line limit?** (Found while applying counter-review item 74.) Decided by the maintainer on 2026-11-25: (a), the adapter splits it in parts, as `plan_observed` is, applied when its last part arrives (R59; SPEC-0002 payload of `branch_changes_observed`). The `changed_paths` of `pull_request_merged` and `default_branch_pushed` are not part of a payload, so their events stay within the limit.
- [x] **Q16. Does the projection hold the map of `input_hash` values and the set of `external_id` values?** (Counter-review item 75.) Decided by the maintainer on 2026-11-25: (b), the projection leaves them out and states their sizes; the state, the snapshots and replay keep them in full; task T10 of the tasks of SPEC-0002 measures the cost (section Storage, AC69).
