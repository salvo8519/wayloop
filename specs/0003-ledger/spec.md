# SPEC-0003: Ledger events, hash chain and replay

| | |
| --- | --- |
| Status | Draft |
| Milestone | M1 · Engine core |
| Owner | @salvo8519 |
| Related | #12, #50, #51, #52, #161, #162; ADR 0002, ADR 0003, ADR 0006, ADR 0007, ADR 0008, ADR 0009, ADR 0010, ADR 0013, ADR 0028, ADR 0029; SPEC-0001, SPEC-0002, SPEC-0004, SPEC-0005, SPEC-0006, SPEC-0007, SPEC-0008, SPEC-0010, SPEC-0020, SPEC-0021 |
| Commit scope | ledger |

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
take the next free number (SPEC-0001 R20), whatever section they belong to.

- **R1**: THE ENGINE SHALL append every event to the ledger as one JSON Lines line and never rewrite, reorder or delete an acknowledged line; the only exceptions are the repair of a torn tail by the writer (R20), which removes no acknowledged line, and the recovery of R34.
- **R2**: THE ENGINE SHALL store in every event, as `prev_hash`, the `hash` of the previous event of the ledger, and in the genesis event 64 zero digits.
- **R3**: WHEN the ledger is replayed from the genesis event, THE ENGINE SHALL produce a state and a projection byte-identical to those the writer obtained by folding each event as it appended it, on every supported platform.
- **R4**: IF the hash chain is broken, THEN THE ENGINE SHALL refuse every append and report the first broken event with its segment file, line, sequence number, pointer, expected and found values, and the revision of the last valid event.
- **R5**: THE ENGINE SHALL record every ruling as an event with a decision, a reason and the cost if wrong, and refuse a ruling that lacks any of them.
- **R6**: THE ENGINE SHALL produce session, issue and milestone summaries as JSON validated by the summary schema, each within the token budget of its level.
- **R7**: THE ENGINE SHALL write snapshots of the state and, WHEN it loads the ledger, replay from the latest valid snapshot plus the events that follow it.
- **R8**: WHEN the engine reads an event, a snapshot or a summary of an earlier known schema version, THE ENGINE SHALL verify the hash of the stored bytes first, then migrate the document in memory to the current version, and never rewrite a stored event.
- **R9**: THE ENGINE SHALL record the pull request observations opened, synchronized (new head), checks completed with the name and conclusion of every check, review submitted, merged, closed without merge and reopened, each with its pull request number, issue, branch or head commit as listed in section Payloads.
- **R10**: THE ENGINE SHALL record a checkout of an issue with its holder, session, executor, branch and worktree name, and refuse a checkout of the issue by another session while the lease of the holding session holds, with the code `checkout_held`, the reason `conflict`, the holder and the end of the lease.
- **R11**: WHEN the lease of a session ends, THE ENGINE SHALL record, through `due_events`, a `checkout_released` event with the reason `lease_expired` for each checkout of that session.
- **R12**: WHEN the holder of a checkout ends a session, THE ENGINE SHALL record a hand-over (SPEC-0001, Hand-over record) and refuse one whose size exceeds the hand-over budget, with the reason `length`, the measured size and the budget; WHEN the last holder of an issue submits a hand-over after its lease ended and before another session claims the issue, THE ENGINE SHALL record it as a late hand-over.
- **R13**: WHEN an agent checks out an issue that has a hand-over, THE ENGINE SHALL start its brief from the resume pack of R44, whatever agent or executor wrote the hand-over.
- **R14**: THE ENGINE SHALL start every ledger with one genesis event of kind `ledger_created` at sequence 0, and refuse a ledger whose first event is not a genesis event or that holds a second one.
- **R15**: THE ENGINE SHALL compute the `hash` of an event as the SHA-256, in lower-case hexadecimal, of the UTF-8 bytes of the event without its `hash` member, serialised as canonical JSON (RFC 8785), and refuse an event whose stored `hash` differs.
- **R16**: THE ENGINE SHALL write every event as its canonical JSON followed by one line feed, at most `line_limit_bytes` of the genesis event, and refuse on read a line that exceeds the limit, checked before it is parsed, or that is not canonical JSON, with the segment file and line.
- **R17**: THE ENGINE SHALL number events with `seq` from 0, each one more than the previous, and report a missing, repeated or out-of-order number with the reason `range`.
- **R18**: THE ENGINE SHALL write through one writer that holds an exclusive whole-file lock of the operating system on `ledger.lock` for each write, appends the lines, flushes them to stable storage, and only then acknowledges the events; the writer releases the lock after the write, and the operating system releases it when the process ends.
- **R19**: WHEN a command that changes state arrives, THE ENGINE SHALL require its expected revision, evaluate it under the lock with the scoped rule of SPEC-0002 section Revisions, and refuse a stale one with the code `revision_stale` and the reason `stale`, appending nothing.
- **R20**: IF the last line of the last segment has no final line feed, THEN every reader SHALL ignore that line, and the writer, under its lock, SHALL move its bytes to the quarantine, record a `torn_tail_discarded` event with their size and hash, and report a warning.
- **R21**: IF the output of an agent or an executor does not validate against the schema of its step, THEN THE ENGINE SHALL record a `result_rejected` event with the step, the error codes and pointers, and a reference to the stored output by hash, count it as one retry of the step (SPEC-0002 R23), and leave every other part of the state unchanged.
- **R22**: IF an event has a kind or a schema version the engine does not know, THEN THE ENGINE SHALL stop loading, report the line with the reason `enum`, the known values in `expected` and the minimum engine version recorded by the ledger, and refuse every append.
- **R23**: THE ENGINE SHALL read the format version, the hash algorithm, the canonicalisation and the line limit of a ledger from its genesis event, and refuse a ledger whose values it does not know with the reason `enum`.
- **R24**: THE ENGINE SHALL keep the ledger and its derived files under `<git common dir>/wayloop/` with the layout of section Storage, so that every worktree of a clone reads and writes the same ledger.
- **R25**: WHEN events are appended, THE ENGINE SHALL rewrite the projection `state.yaml` under the lock, through a temporary file renamed over it, deterministically from the state; IF the stored projection differs from the projection of the state, THEN THE ENGINE SHALL report a warning and rewrite it.
- **R26**: THE ENGINE SHALL treat the SQLite index as a cache built from the ledger and the summaries, record the `(seq, hash)` it was built at, bring it to the current revision with upserts keyed by `seq` before answering a query, rebuild it when its revision is not in the chain, and never read it to decide a transition.
- **R27**: THE ENGINE SHALL set `recorded_at` of every event, in UTC to the second with the suffix `Z`, from a time given to the writer as an input, use the `recorded_at` of the previous event when the given time is earlier, and IF the given time is earlier by more than `limits.lease_minutes`, THEN refuse the append with the code `clock_behind`.
- **R28**: THE ENGINE SHALL take the current time only as the explicit input of `due_events`, append the events it returns under the lock before deciding any command that changes state, never call it for a command that only reads, and compute every result from recorded events, so that no envelope holds the time of the call.
- **R29**: THE ENGINE SHALL refuse an event that holds a number that is not an integer written without fraction or exponent between -(2^53 - 1) and 2^53 - 1, with the reason `type`.
- **R30**: WHEN an observation carries an `external_id` already recorded, THE ENGINE SHALL append nothing and report the duplicate as a warning.
- **R31**: WHEN the engine takes a snapshot, THE ENGINE SHALL record a `snapshot_taken` event with the sequence number, the SHA-256 and the fold version of the snapshot, start a new segment after it, and ignore with a warning a snapshot whose hash differs from its event.
- **R32**: WHEN the engine loads the ledger, THE ENGINE SHALL verify every event after the latest valid snapshot and its link to that snapshot; WHEN a session starts, and WHEN `wayloop ledger verify --full` runs, THE ENGINE SHALL verify every segment from the genesis event.
- **R33**: WHILE the chain is broken or the ledger regressed, THE ENGINE SHALL answer commands that read state from the valid prefix, with a warning that names the first broken event or the regressed file.
- **R34**: WHEN a person runs the recovery of a broken or regressed ledger, THE ENGINE SHALL show the last event it keeps, require its hash as confirmation, move every line from the first broken event onward to the quarantine, and append a `ledger_recovered` event linked to the last kept event; IF the command does not come from a person through a channel of section Person-only commands, or the ledger is neither broken nor regressed, THEN THE ENGINE SHALL refuse it with the reason `state`.
- **R35**: THE ENGINE SHALL fill a summary with the fill rule of section Roll-up, so that the result is a function of the events alone, fits its budget, and counts every dropped entry in `omitted`.
- **R36**: THE ENGINE SHALL compute the session summary when a session ends, the issue summary when a session of the issue ends and when the issue merges, its pull request closes or its task is superseded, and the milestone summary when an issue of the milestone merges or its pull request closes, each subject at most once per input, and record each with a `summary_recorded` event that carries the budget used.
- **R37**: THE ENGINE SHALL take the summary budgets from `ledger_budgets` of the latest `limits_set` (SPEC-0002), with the defaults of section Budgets written by `wayloop init`, and refuse a configuration in which a budget is below 256 tokens, the session or the issue budget is below the hand-over budget plus 256, or the milestone budget is below the issue budget, with the reason `range`.
- **R38**: WHEN a summary or a hand-over is to be published, THE ENGINE SHALL render it deterministically with a marker that names its level, subject and SHA-256 and with its canonical JSON, publish it through the tracker adapter, skip it when a comment with the same marker and hash exists, and record a `summary_published` event after the tracker confirms it.
- **R39**: WHEN the engine reads a published summary or hand-over, THE ENGINE SHALL accept only a comment whose author, as reported by the tracker, is the configured account of the runtime or a recorded approver, validate its JSON against its schema and its hash against the marker, and treat a comment that fails any check as not published, with a warning.
- **R40**: THE ENGINE SHALL pass every payload through the secret check of SPEC-0020 R4 before it is appended, every executor output before it is stored as a blob, and every rendered summary before it is published; THE ENGINE SHALL refuse a command or a result that fails, and SHALL record an observation that fails with each failing value redacted.
- **R41**: THE ENGINE SHALL write every summary file as its canonical JSON followed by one line feed, and IF the SHA-256 of a stored summary file differs from that of its `summary_recorded` event, THEN THE ENGINE SHALL report a warning and write it again from the events, at the summary schema version the event records.
- **R42**: WHEN a recorded approver revokes a ruling, THE ENGINE SHALL record a `ruling_revoked` event that names the hash of the ruling event and a reason, keep the ruling in the history, and leave it out of the active rulings and of later summaries.
- **R43**: WHEN the lease of a session ends, THE ENGINE SHALL record, after the checkout releases of R11, `session_ended` with the outcome `abandoned`, and compute its session summary from its events.
- **R44**: THE ENGINE SHALL build the resume pack of an issue from the milestone summary, the issue summary and the latest hand-over of the issue, in that order, without repeating an entry already given, within the sum of their budgets, and SHALL place it in the brief as quoted data that names its author, never in the instruction part of the brief.
- **R45**: THE ENGINE SHALL record every executor call with role, executor, model, effort, step, phase, turns, tokens in, tokens out, duration in milliseconds and the source of its cost, with the cost in integer micro-units of a currency when the source gives one, and refuse a record that lacks a required field.
- **R46**: WHILE a session holds a checkout, THE ENGINE SHALL extend its lease with every event of that session, including `lease_renewed`, and WHEN the checkout ends, THE ENGINE SHALL record a `checkout_released` event with the reason of section Payloads.
- **R47**: THE ENGINE SHALL assign every session the identifier `session-<ledger>-<YYYY-MM-DD>-<NN>`, where `<ledger>` is the first eight digits of the `ledger_id`, the date is the UTC date of its `session_started` event, and `NN` is one more than the highest number used on that date in the ledger, the quarantine and the publications on the tracker, at least two digits.
- **R48**: IF a hand-over or a lease renewal for an issue comes from a session that does not hold its checkout, other than the late hand-over of R12, THEN THE ENGINE SHALL refuse it with the code `checkout_not_held`, the reason `state` and the holder; IF a session end comes from an actor that is not the holder of that session, THEN THE ENGINE SHALL refuse it the same way.
- **R49**: IF a derived file or a published summary cites a revision that is neither in the chain nor in a quarantine recorded by `ledger_recovered`, or a sequence number above the last event, THEN THE ENGINE SHALL report `ledger_regressed` with the file and the revision, and refuse every append until a person runs the recovery of R34.
- **R50**: WHEN `wayloop ledger verify --full` runs, THE ENGINE SHALL also check that the revision of every published summary recorded by `summary_published` or `publication_imported` is in the chain or in a recorded quarantine.
- **R51**: IF the git common directory is on a network file system, on a Windows drive reached from WSL, on a container bind mount or in a folder kept by a file synchronisation client, as far as the platform reports it, THEN THE ENGINE SHALL refuse to open the ledger with the code `ledger_storage_unsupported` and exit code 3.
- **R52**: WHEN a command arrives whose `input_hash` is already recorded by an event of the same session, THE ENGINE SHALL append nothing and return the result of the recorded events, before it evaluates the expected revision.
- **R53**: WHEN the writer appends the first event of a kind or of a schema version that requires a newer engine than the minimum recorded by the ledger, THE ENGINE SHALL first record `min_engine_version_raised` with the new minimum.
- **R54**: THE ENGINE SHALL record in every snapshot the fold version of the engine that wrote it, and ignore with a warning a snapshot whose fold version differs from its own.
- **R55**: WHEN `wayloop init` creates a ledger in a clone whose tracker holds published summaries and hand-overs, THE ENGINE SHALL import with `publication_imported` every one that passes R39, so that resume packs and identifiers continue from them.
- **R56**: THE ENGINE SHALL refuse in `files_touched` and in every recorded path an absolute path, a `..` segment or a path not in Unicode NFC, and SHALL render published text so that it cannot close its marker or code fence or notify a tracker handle (section Security rules).

## Acceptance criteria
- **AC1** (R3): Given a generated ledger of 10,000 valid random events, when it is replayed twice and compared with the incremental fold of the writer that appended it, then the three states, the three projections and the revisions are byte-identical. Verification: property test.
- **AC2** (R4, R15): Given `examples/ledger.edited-line.invalid.jsonl`, when it is loaded, then the error has the code `ledger_hash_mismatch`, `source` names the file and line 5, the pointer is `/hash`, and the revision of the last valid event is the hash of line 4. Verification: TDD, with the files of `examples/` as fixtures.
- **AC3** (R6, R35): Given a milestone of 40 issues with 200 sessions, each hand-over at the budget, when the milestone summary is rolled up, then it validates, its `tokens` is at most the milestone budget, and for each section the kept entries plus the count in `omitted` equal the source entries of that section. Verification: TDD.
- **AC4** (R9, R6): Given a merged pull request, when the issue summary is rolled up, then it has the outcome `merged` and the pull request entry carries the merge commit and `merged_at`. Verification: TDD, with `examples/summary.issue.merged.valid.json`.
- **AC5** (R11, R28, R43): Given a session whose latest event plus its lease is earlier than the time given to `due_events`, when the next command that changes state is decided, then `checkout_released` with `lease_expired` and `session_ended` with `abandoned` are recorded first and the issue is ready; given a command that only reads, then nothing is appended. Verification: property test.
- **AC6** (R13, R44): Given a hand-over written in a session of one executor, when an agent of another executor checks out the issue, then its brief starts with the resume pack as quoted data that names the author of each part, and the pack names that hand-over. Verification: TDD.
- **AC7** (R2, R14, R15, R16, R17): Given `examples/ledger.valid.jsonl`, when it is verified, then it passes, every line is canonical, every `prev_hash` equals the `hash` of the previous line, no lease ends between two events of one session, and the revision is `27765ee853d32a87cf9a58811891449fa18c103f1ea96854c95deb9e059c0c0c`. Verification: contract test.
- **AC8** (R15): Given any event object and any permutation of the order of its members, when hashed, then the hash is the same and equals the SHA-256 of the canonical bytes without `hash`. Verification: property test.
- **AC9** (R16): Given `examples/ledger.non-canonical.invalid.jsonl`, when it is loaded, then the error has the code `ledger_line_malformed`, the reason `syntax` and `source` with line 3; given a line one byte above the line limit, then the reason is `length` and the line is not parsed. Verification: TDD.
- **AC10** (R17, R4): Given `examples/ledger.sequence-gap.invalid.jsonl`, when it is loaded, then the first error has the code `ledger_sequence_gap`, the reason `range`, `source` with line 7 and `expected` holding 6. Verification: TDD.
- **AC11** (R2, R4): Given `examples/ledger.rehashed-line.invalid.jsonl`, when it is loaded, then the error has the code `ledger_chain_broken`, the pointer `/prev_hash` of line 6, and `pointers` naming line 5. Verification: TDD.
- **AC12** (R14, R23): Given `wayloop init` on a new clone, when the ledger is created, then line 1 is `ledger_created` with `seq` 0, `prev_hash` of 64 zeros, `format_version` 1 and `line_limit_bytes` 65,536; given a ledger whose first event is another kind, or that holds a second genesis event, then it is refused with the code `ledger_genesis`; given a genesis event with `hash_algorithm` `sha512`, then it is refused with the code `ledger_format_unknown` and the reason `enum`. Verification: TDD.
- **AC13** (R18, R24): Given two processes in two worktrees of one clone, each appending 1,000 events at the same time, when both finish, then the ledger holds 2,001 events in one valid chain and every acknowledged event is present once; the test runs in CI on Linux, macOS and Windows. Verification: end-to-end.
- **AC14** (R19): Given an expected revision after which an event named the issue of the command, when a command that changes state runs, then it is refused with `revision_stale` and the reason `stale` and the ledger is unchanged; given an expected revision after which only events of other issues were recorded, then the command is accepted; given a command without an expected revision, then it is refused with the reason `missing`. Verification: TDD.
- **AC15** (R20): Given `examples/ledger.torn-tail.invalid.jsonl`, when a reader loads it, then the partial line is ignored and nothing is moved; when the writer next takes the lock, then the partial line is moved to the quarantine, a `torn_tail_discarded` event follows line 31, the chain is valid and a warning `ledger_torn_tail` is reported; given a malformed line that is not the last one, then nothing is moved and the error of R16 is reported. Verification: TDD.
- **AC16** (R21): Given an executor result that misses a required field, when it is returned, then a `result_rejected` event is recorded with the error pointer and the hash of the stored output, the retries of the step increase by one, and the state differs from the previous one only in the retries, the closed dispatch and the list of rejected results. Verification: TDD.
- **AC17** (R22): Given an event of kind `future_kind` or of `schema_version` 99, when the ledger is loaded, then loading stops with the codes `event_kind_unknown` or `event_version_unknown`, the reason `enum`, the known values in `expected` and the recorded minimum engine version, and an append is refused. Verification: TDD.
- **AC18** (R8): Given a ledger fixture with events of schema version 1 of a kind whose current version is 2, when it is loaded, then every event is migrated in memory, the hash is checked on the stored bytes, the files are unchanged byte for byte, and the projection equals that of the same ledger written in version 2. Verification: TDD, with one fixture per released version.
- **AC19** (R8, R39): Given a published issue summary of schema version 1 while the current version is 2, when it is read back, then its hash is checked on the published JSON, it is migrated and validates against version 2. Verification: TDD.
- **AC20** (R24): Given the main worktree and a second worktree of one clone, when `wayloop status` runs in each, then both report the same `meta.revision`. Verification: end-to-end.
- **AC21** (R3, R25): Given the golden ledger of the repository, when it is replayed in CI on Linux, macOS and Windows, then the three projections are byte-identical to the golden `state.yaml`. Verification: end-to-end.
- **AC22** (R25): Given a `state.yaml` edited by hand, when the next event is appended, then a warning `projection_mismatch` is reported and the file equals the projection of the state; given a writer killed during the rewrite, then `state.yaml` is either the previous or the new projection. Verification: TDD.
- **AC23** (R26): Given a deleted index, when `wayloop history` runs, then the index is rebuilt and the result equals the result before deletion; given an index built at a revision that is not in the chain, then it is rebuilt; given the same events upserted twice, then the index is unchanged; given any content of the index, then the next step is the same. Verification: property test.
- **AC24** (R27): Given a writer clock that moves 30 minutes backwards between two appends with a lease of 120 minutes, when the second event is recorded, then its `recorded_at` equals that of the first; given a clock 3 hours behind, then the append is refused with `clock_behind`; given any appended event, then `recorded_at` matches `YYYY-MM-DDTHH:MM:SSZ`. Verification: TDD, with a fake clock.
- **AC25** (R28): Given one ledger, when `wayloop status` and `wayloop history` run at two different clock times with the same input, then the two envelopes are byte-identical and the ledger is unchanged. Verification: contract test.
- **AC26** (R29): Given a payload holding `1.5`, `1.0`, `1e3` or 2^53, when it is appended, then it is refused with the code `event_schema`, the reason `type` and the pointer of the number. Verification: TDD.
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
- **AC38** (R40): Given a hand-over whose `next_step` holds a token that the secret check of SPEC-0020 detects, when it is submitted, then it is refused and the ledger and the tracker are unchanged; given a pull request title holding such a token, when it is observed, then `pull_request_opened` is recorded with the title redacted; given a rejected executor output holding one, then no blob is stored and `result_rejected` names the finding. Verification: TDD.
- **AC39** (R41): Given a summary file whose content was changed, when the summaries are read, then a warning `summary_mismatch` is reported and the file equals again the one of `examples/` for the same events. Verification: TDD.
- **AC40** (R5, R42): Given a ruling without `cost_if_wrong`, when it is submitted, then it is refused with the reason `missing`; given a ruling revoked by an approver, when `wayloop rulings` runs, then it is absent from the active rulings and present in the history with its revocation. Verification: TDD.
- **AC41** (R43, R11): Given a session whose lease ends without a hand-over, when the due events are computed, then `checkout_released` with `lease_expired` and `session_ended` with the outcome `abandoned` are recorded and the session summary has no `next_step`; given then a hand-over from the same holder before another claim, then it is recorded as late and the issue summary is rolled up again. Verification: TDD.
- **AC42** (R44): Given a milestone, an issue and a hand-over at their budgets that share one ruling, when the resume pack is built, then it holds the three in the order milestone, issue, hand-over, the ruling once, and its size is at most the sum of the three budgets. Verification: TDD.
- **AC43** (R45): Given an executor call record without `cost_source`, when it is submitted, then it is refused with the reason `missing`; given the fixture ledger, when `wayloop cost` runs, then the totals per currency equal the sum of the records and the call without a cost is counted apart. Verification: TDD.
- **AC44** (R10, R46): Given issue 12 checked out by `agent-lane-1`, when `agent-lane-2` checks it out, then it is refused with the code `checkout_held`, the reason `conflict`, the holder and the end of the lease; when `agent-lane-1` renews the lease, then the lease is extended and no second checkout is recorded; when the issue reaches `merge`, then `checkout_released` with the reason `awaiting_merge` is recorded. Verification: TDD.
- **AC45** (R12, R48): Given a hand-over of 1,300 tokens with a budget of 1,200, when it is submitted, then it is refused with the code `handover_budget`, the reason `length`, 1,300 and 1,200; given a hand-over from a session that does not hold the checkout, then it is refused with the code `checkout_not_held` and the reason `state`; given a session end from the holder of a session that holds no checkout, then it is accepted. Verification: TDD.
- **AC46** (R9): Given each pull request event of `examples/ledger.valid.jsonl`, when validated against its payload schema, then it passes; given a checks event whose `checks` are not sorted by name, then it is refused with the reason `syntax`. Verification: contract test.
- **AC47** (R47): Given the ledger `5b0c7a1e-...`, two sessions started on 2027-01-15 and one on 2027-01-16, when they are recorded, then their identifiers are `session-5b0c7a1e-2027-01-15-01`, `session-5b0c7a1e-2027-01-15-02` and `session-5b0c7a1e-2027-01-16-01`; given a quarantine that holds `session-5b0c7a1e-2027-01-16-02`, then the next session of that date is `-03`. Verification: TDD.
- **AC48** (R1, R18, R52): Given an append interrupted at every byte offset of its line, and after the line feed before the flush, when the ledger is loaded and the command is retried with the same input in the same session, then every acknowledged event is present once; a partial line is quarantined by the writer and the retry appends the event once; a complete line is kept and the retry appends nothing and returns its result. Verification: property test.
- **AC49** (R16, R29, R56): Given `examples/canonicalization.vectors.json`, when each input is canonicalised and hashed, then the output and the hash equal the expected ones, and each invalid input is refused with its expected reason: `type` for `1.0` and `1e3`, `syntax` for a lone surrogate and for invalid UTF-8. Verification: contract test.
- **AC50** (R27): Given `examples/ledger.time-order.invalid.jsonl`, when it is verified, then the error has the code `ledger_time_order`, the reason `range` and `source` with line 9. Verification: TDD.
- **AC51** (R49, R50): Given a `state.yaml` or a snapshot whose `seq` is above the last event, or a published summary whose revision is not in the chain or a recorded quarantine, when the ledger is loaded or verified in full, then `ledger_regressed` names the file and the revision and an append is refused; given the recovery by a person, then `ledger_recovered` with the reason `regressed` is recorded and the derived files are rebuilt. Verification: TDD.
- **AC52** (R51): Given a git common directory on a network file system, when a command opens the ledger, then it fails with `ledger_storage_unsupported` and exit code 3, on each platform where the test can mount one. Verification: end-to-end.
- **AC53** (R53): Given a ledger whose minimum engine version is 0.1.0, when an engine of version 0.3.0 first appends a kind introduced in 0.3.0, then `min_engine_version_raised` with 0.3.0 precedes it; when an engine of version 0.2.0 loads that ledger, then it stops with the code `event_kind_unknown` and reports 0.3.0. Verification: TDD.
- **AC54** (R55): Given a tracker holding a published hand-over and an issue summary of issue 12, a forged summary from another account and a new clone, when `wayloop init` runs, then two `publication_imported` events are recorded, the forged one is ignored, the next hand-over is `HANDOVER-12-2` and the resume pack of issue 12 holds the imported ones. Verification: TDD.
- **AC55** (R56): Given a hand-over whose `files_touched` holds `/etc/passwd`, `../x` or a path in NFD, when submitted, then each is refused with its pointer; given a ruling whose text holds a marker of R38, a run of four backticks and a tracker handle, when the summary is rendered, then the marker is escaped, the fence is longer than four backticks and the handle notifies nobody. Verification: TDD.

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
| `actor` | object `{kind, id}`; `kind` is `engine`, `agent`, `person` or `tracker` | yes | Who caused the event: the engine for its own decisions, the agent holder for validated agent input, a tracker handle for a person's act, the tracker for an observation |
| `session` | string, R47 | no | Session in which the event occurred; set for events caused by a command or a result of that session, and for the release of a checkout of that session |
| `issue` | integer | no | Tracker number of the issue concerned |
| `input_hash` | string, 64 lower-case hexadecimal digits | for events caused by a command | `meta.input_hash` of the command (SPEC-0004 R11); the idempotency key of R52 |
| `dispatch` | string, 64 lower-case hexadecimal digits | for events caused by a result | Hash of the `step_started` event the result answers (SPEC-0002 R21) |
| `external_id` | string, 1 to 200 characters | for events caused by an observation | Identity of the observed fact, used to ignore duplicates (R30) |
| `payload` | object | yes, may be `{}` | Fields of the kind |
| `prev_hash` | string, 64 lower-case hexadecimal digits | yes | Hash of the previous event; 64 zeros for the genesis event (R2) |
| `hash` | string, 64 lower-case hexadecimal digits | yes | Hash of this event (R15) |

An event carries at most one of `input_hash`, `dispatch` and `external_id`. Events returned by
`due_events`, the integrity events of the writer and `summary_published` carry none.

`external_id` is built by the adapter that observes the fact, so that a repeated delivery of one
fact gives the same value and two facts never share one:

| Source | `external_id` |
| --- | --- |
| Pull request opened, merged | `pull_request:<number>:opened`, `pull_request:<number>:merged` |
| Pull request closed, reopened | `pull_request:<number>:closed:<k>`, `pull_request:<number>:reopened:<k>`, where `k` is the occurrence of that act on the pull request, from 1 |
| New head | `pull_request:<number>:head:<sha>` |
| Checks | `check_suite:<id>` or `check_run_attempt:<id>`, the identifier the tracker gives to one run of the checks |
| Review | `review:<id>` |
| Comment of a person (approval, answer, request) | `comment:<id>`, or `comment:<id>:<edit>` for an edited comment |
| Plan observation | `plan:<plan_sha256>:<part>` |
| Branch changes | `branch:<branch>:<head_sha>` |
| Imported publication | `comment:<id>:<sha256>` |

A publication by the engine (`summary_published`) is not an observation: its event carries no
`external_id`; the marker and hash of R38 make the publication idempotent. Every number is an
integer within the exact range of a double (R29). Amounts of money are integer micro-units
(`cost_micros`) with an ISO 4217 `currency`; durations are integer milliseconds (`duration_ms`).
Floating-point numbers would make canonical JSON depend on the formatting of the platform.

Content larger than an event, such as a rejected executor output or a test log, is stored once
under `blobs/<sha256>` and referenced as `{file, sha256}`; the event never inlines it (R16).

### Event catalogue

This is the one catalogue of the ledger. Every kind recorded by the engine is listed here, with
the specification that owns its payload schema. A kind owned by another specification is
reserved here, and its payload is specified by its owner. Adding a kind is an additive change of
the catalogue, preceded by `min_engine_version_raised` in a ledger that holds it (R53); an
engine that meets a kind it does not know stops loading (R22), because the state would
otherwise differ from that of a newer engine. Consumers of the outputs of the engine, such as
agents and SDKs, accept enum values they do not know (ADR 0009 C5).

| Kind | Role | Payload owner | Consumed by |
| --- | --- | --- | --- |
| `ledger_created` | Genesis of the ledger (R14, R23) | SPEC-0003 | Loader |
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
| `pull_request_merged` | Observation (R9) | SPEC-0003 | SPEC-0002 I18, I24; roll-up |
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
| `run_started` | Run of the runtime with its account | SPEC-0002 | SPEC-0002 R9 |
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
| `issue_done` | Issue cycle completed by a merge | SPEC-0002 | SPEC-0002; roll-up |
| `issue_superseded` | Issue cycle ended by a superseded task | SPEC-0002 | SPEC-0002; roll-up |
| `artifact_status_changed` | Status of an artifact | SPEC-0002 | SPEC-0002 phase and readiness |
| `budget_exhausted` | Budget scope exhausted | SPEC-0002 | SPEC-0002 R29; SPEC-0006 R6 |
| `budget_restored` | Budget scope raised again | SPEC-0002 | SPEC-0002 R29 |
| `reconciliation_required` | Inconsistent observation | SPEC-0002 | SPEC-0007 reconciliation |
| `policy_evaluated` | Evaluation of the policy on a pull request | SPEC-0021 R7 | SPEC-0021 R8 |
| `waiver_used` | Waiver of a policy rule | SPEC-0021 R7 | SPEC-0021 R8 |

The catalogue holds 54 kinds. A stopped ledger is not an event: a broken chain cannot be
appended to, and SPEC-0002 reports it with the stop reason `ledger_broken`.

### Payloads

Payloads owned by this specification, version 1. Payloads are objects at most four levels deep
(ADR 0009 C4); lists are written `[]` when empty.

| Kind | Fields |
| --- | --- |
| `ledger_created` | `ledger_id` (string, UUID chosen at creation), `format_version` (1), `hash_algorithm` (`sha256`), `canonicalization` (`rfc8785`), `line_limit_bytes` (integer, at least 4,096), `min_engine_version` (semantic version), `previous_ledger` (optional `{ledger_id, last_hash}`, set when a ledger replaces an earlier one) |
| `min_engine_version_raised` | `min_engine_version`, `kind`, `schema_version` (the kind and version that require it) |
| `session_started` | `holder`, `executor` (snake_case name, as in the hand-over record), `mode` (`in_session` or `unattended`) |
| `session_ended` | `outcome` (`handed_over`, `completed` or `abandoned`), `handover` (identifier, when `handed_over`) |
| `checkout_acquired` | `holder`, `executor`, `branch` (valid under SPEC-0001 R11), `worktree` (name of the git worktree, never a path), `resumed_from` (optional hand-over identifier) |
| `lease_renewed` | none |
| `checkout_released` | `holder`, `reason` (`handed_over`, `awaiting_merge`, `merged`, `pull_request_closed`, `lease_expired`, `superseded`), `lease_ended_at` (when `lease_expired`, computed as in section Clock times) |
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
| `executor_call_recorded` | `role`, `executor`, `model`, `effort`, `step`, `phase`, `turns` (executor turns of the call), `tokens_in`, `tokens_out`, `duration_ms`, `cost_source` (`provider_reported`, `price_table` or `none`), `cost` (`{cost_micros, currency}`, absent when `cost_source` is `none`) |
| `result_rejected` | `step`, `errors` (at most 20 objects `{code, pointer, reason}`), `errors_omitted` (count of further errors), `output` (`{file, sha256}`, absent when the output failed the secret check), `output_withheld` (the code of SPEC-0020, when it failed) |
| `summary_recorded` | `level`, the subject field of the level (`session`, `issue` or `milestone`), `file`, `sha256`, `tokens`, `budget`, `summary_schema_version` |
| `summary_published` | `level` (`session`, `issue`, `milestone` or `handover`), the subject field (`handover` for a hand-over), `sha256`, `target` (`issue_comment`), `issue` (the issue that holds the comment), `comment` (tracker identifier) |
| `publication_imported` | `level`, the subject field, `sha256`, `issue`, `comment`, `author`, `revision` (the revision the publication cites), `file` (where the document was written) |
| `snapshot_taken` | `snapshot_seq`, `sha256`, `state_schema_version`, `fold_version`, `file` |
| `torn_tail_discarded` | `bytes`, `sha256`, `file` (in the quarantine) |
| `ledger_recovered` | `reason` (`chain_broken` or `regressed`), `broken_seq` (optional), `last_kept_hash`, `file` (in the quarantine), `approved_by` (handle), `channel` (`terminal` or `tracker`) |

The event header carries the issue (`issue`) for every kind that concerns one. The pull request
events link their issue through the header `issue` and their branch: R9 requires both, and the
writer refuses a pull request event whose branch names another issue (SPEC-0001 R10).

`lease_ended_at` is the only time computed by the engine; it is derived from recorded times and
`limits.lease_minutes`, never from the clock.

### Schema versions and migration

- Each kind has its own schema snapshot `schemas/events/<kind>/v<N>.json`, which includes the
  common fields. A change of the common fields is a new version of every kind.
- Released snapshots are immutable (SPEC-0001 AC3). Every change is a new version, and the
  first event written in a new version raises the minimum engine version (R53).
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
2. Serialise it as canonical JSON (RFC 8785): members sorted by the UTF-16 code units of their
   names, no insignificant white space, strings in the minimal escaping of the standard,
   integers without exponent or fraction.
3. Compute the SHA-256 of the UTF-8 bytes and write it in lower-case hexadecimal as `hash`.
4. Write the line as the canonical JSON of the whole event, `hash` included, followed by one
   line feed (`0x0A`). The stored line is therefore canonical, and every reader computes the
   same bytes on every platform.

Strings are UTF-8 on disk. A lone surrogate, in an escape or in the bytes, and a byte sequence
that is not UTF-8 are refused with the reason `syntax`. `1.0`, `1e3` and every number with a
fraction or an exponent are refused with the reason `type`, even when their value is an
integer (R29). `examples/canonicalization.vectors.json` holds the vectors of AC49: key order by
UTF-16 code units, with a key outside the Basic Multilingual Plane; escapes of control
characters; accented letters written as UTF-8; and the refused inputs.

Verification of a line, in this order, stopping at the first line that fails and reporting
every error of that line:

1. It ends with a line feed; the last line of the last segment without one is a torn tail (R20).
2. Its size is within the line limit, checked before it is parsed (R16).
3. It is UTF-8, parses as JSON, holds only integers (R29) and equals its canonical form (R16).
4. Its `hash` is correct (R15).
5. Its `seq` is one more than the previous (R17); the first is the genesis event (R14).
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
  quarantine/                                         torn tails, broken lines and regressed files (R20, R34)
  index.sqlite                                        index, a cache (R26)
```

- The git common directory is shared by every worktree of a clone (ADR 0007, ADR 0013), so the
  orchestrator and every lane read one ledger. It is never committed.
- The ledger is the source of truth. The projection, the snapshots, the summary files, the
  hand-over files and the index are derived and can be rebuilt from the ledger; the summary
  files are checked against their events (R41).
- A segment is closed by a `snapshot_taken` event, which is its last line; the next event opens
  a new segment. Closed segments are never written again and are kept uncompressed; they are
  deleted only by a person, through a command recorded as an event (Q1).
- Paths in events and errors, including `source.file`, are relative to the git common
  directory, such as `wayloop/ledger/000000000000.jsonl`; an absolute path never enters the
  ledger or an envelope.
- The projection is the YAML rendering of the canonical JSON of the state: members in canonical
  order, block style, two-space indentation, every string double-quoted, line feeds, one final
  line feed. Its first members are `revision` and `seq`.
- A snapshot file is `{schema_version, fold_version, engine_version, seq, revision, state}` in
  canonical JSON. Its `revision` equals the `prev_hash` of the `snapshot_taken` event that
  anchors it.
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
  writer reads the tail, repairs a torn tail (R20), appends the due events (R28), checks the
  idempotency key (R52) and the expected revision (R19), appends, flushes, writes the derived
  files and releases the lock. The operating system releases the lock of a process that dies,
  so a crash never leaves a stale lock. A writer that waits longer than `lock_timeout_seconds`
  fails with the code `ledger_locked` (exit code 3, ADR 0009 C7).
- Flushing to stable storage means `fsync` of the segment on Linux, `fcntl(F_FULLFSYNC)` on
  macOS, where `fsync` does not reach the disk, and `FlushFileBuffers` on Windows; after a new
  segment or a renamed file is created, the directory is flushed too on Linux and macOS.
- The lock serialises writes; the expected revision detects a decision taken on an older state;
  the lease of a session stops two agents from working on one issue; the idempotency key makes
  a retried command safe.
- An event is acknowledged only after its line is flushed to stable storage. A command
  interrupted before the acknowledgement may still have left a complete line; its retry in the
  same session finds the `input_hash` and appends nothing (R52).
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
| Partial last line | `ledger_torn_tail` (warning) | `syntax` | Ignored by readers; quarantined and recorded by the writer (R20); never acknowledged, so nothing is lost |
| Line over the limit | `ledger_line_malformed` | `length` | Broken chain |
| Line not UTF-8, not JSON or not canonical | `ledger_line_malformed` | `syntax` | Broken chain |
| Number with a fraction or an exponent | `ledger_line_malformed` | `type` | Broken chain |
| Stored `hash` wrong | `ledger_hash_mismatch` | `conflict` | Broken chain |
| `seq` not one more than the previous | `ledger_sequence_gap` | `range` | Broken chain |
| `prev_hash` not the previous `hash` | `ledger_chain_broken` | `conflict` | Broken chain |
| First event not genesis, or a second genesis | `ledger_genesis` | `missing`, `conflict` | Broken chain |
| `recorded_at` earlier than the previous | `ledger_time_order` | `range` | Broken chain |
| Derived file or publication ahead of the chain | `ledger_regressed` | `conflict` | Appends refused (R49) |

On a broken chain or a regressed ledger the engine refuses every append (R4, R49), answers reads
from the valid prefix with a warning (R33), and the loop stops with the stop reason
`ledger_broken` of SPEC-0002. The report carries `source` `{file, line}` (SPEC-0001, Errors),
the pointer into the event, the `expected` and found values, `seq`, and the revision of the last
valid event.

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

`wayloop ledger recover`, `wayloop init` and the deletion of closed segments are decisions of a
person. The engine accepts them only through these channels:

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
  R11 and the session ends of R43. Every command that changes state appends them first, under
  the lock (R28); a command that only reads never calls it and never appends.
- The lease of a session ends at the `recorded_at` of the latest event of that session plus
  `limits.lease_minutes` of the latest `limits_set` (R46). While a step runs, the runtime of the
  holder renews the lease every quarter of it (SPEC-0002 R22).
- A given time earlier than the last `recorded_at` is raised to it (R27). A clock behind by more
  than the lease would extend leases that already ended, so the append is refused with
  `clock_behind`.
- Envelopes never hold the time of the call (ADR 0028, SPEC-0004 R15). A result may hold the
  `recorded_at` of recorded events or summaries: those values are part of the state at
  `meta.revision`, so the same input on the same revision gives the same envelope at any time
  (AC25). Durations are computed between recorded times, never as an age relative to now.

## Sessions, checkouts and hand-overs

- A session starts with `session_started` and ends with `session_ended`. Its identifier follows
  R47; the short ledger prefix keeps identifiers of two clones apart in the publications on the
  tracker, and the number continues from the highest used on that date in the ledger, the
  quarantine and the publications, so that a recovery or a new clone never reuses one.
- Hand-over numbers `HANDOVER-<issue>-<k>` continue the same way: `k` is one more than the
  highest found in the ledger, the quarantine and the publications of the issue.
- A session holds zero or more checkouts. A checkout ends with `checkout_released` and one reason:
  `handed_over` (the session ended with a hand-over), `awaiting_merge` (the issue reached
  `merge`, SPEC-0002 R35), `merged`, `pull_request_closed`, `lease_expired` or `superseded`.
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

| Member of `ledger_budgets` | Default, tokens | Bounds |
| --- | --- | --- |
| `handover` | 1,200 | Hand-over record (R12) |
| `session` | 1,500 | Session summary |
| `issue` | 2,000 | Issue summary |
| `milestone` | 4,000 | Milestone summary |

`wayloop init` writes the defaults to `[ledger.budgets]` of `wayloop.toml` and the first
`limits_set` records them; later changes reach the ledger under SPEC-0002 R8. The resume pack of
an issue (R44) is therefore at most 7,200 tokens with the defaults; its tokens are part of the
`tokens_in` of the executor call that reads it, so they count against the budget of the phase,
sprint or milestone (SPEC-0002). Other settings: `line_limit_bytes` (default 65,536), fixed for
the life of the ledger by its genesis event; `snapshot_every` (default 10,000 events) and
`lock_timeout_seconds` (default 10) in `[ledger]` of `wayloop.toml`, which change no recorded
content; the lease of `limits.lease_minutes` (default 120) and its heartbeat every quarter of
the lease (SPEC-0002).

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

Every refusal is the error report of ADR 0009 C3 with the field `source` of SPEC-0001, Errors.
Pointers follow RFC 6901 from the root of the event, the summary or the input sent. A new
meaning gets a new code; codes are never renamed or reused (ADR 0009 C8).

| Requirement | Code | Reasons |
| --- | --- | --- |
| R1, R5, R9, R29, R45, R56 | `event_schema` | `missing`, `unknown_field`, `type`, `enum`, `length`, `range`, `syntax`, `requires` |
| R2, R4 | `ledger_chain_broken` | `conflict` |
| R14 | `ledger_genesis` | `missing`, `conflict` |
| R15 | `ledger_hash_mismatch` | `conflict` |
| R16, R29 | `ledger_line_malformed` | `syntax`, `length`, `type` |
| R17 | `ledger_sequence_gap` | `range` |
| R18 | `ledger_locked` | `state` |
| R19 | `revision_stale` | `stale`, `missing` |
| R20 | `ledger_torn_tail` (warning) | `syntax` |
| R22 | `event_kind_unknown`, `event_version_unknown` | `enum` |
| R23 | `ledger_format_unknown` | `enum` |
| R25 | `projection_mismatch` (warning) | `conflict` |
| R27 | `ledger_time_order` | `range` |
| R27 | `clock_behind` | `range` |
| R30 | `event_duplicate` (warning) | `duplicate` |
| R31, R54 | `snapshot_mismatch` (warning) | `conflict` |
| R33 | `ledger_broken_prefix` (warning) | `state` |
| R34 | `recovery_refused` | `state`, `conflict` |
| R34 | `publication_orphaned` (warning) | `state` |
| R37 | `ledger_budget_config` | `range` |
| R39 | `publication_mismatch` (warning) | `conflict` |
| R39 | `publication_author` (warning) | `requires` |
| R41 | `summary_mismatch` (warning) | `conflict` |
| R42 | `ruling_unknown` | `missing` |
| R10 | `checkout_held` | `conflict` |
| R12 | `handover_budget` | `length` |
| R48 | `checkout_not_held` | `state` |
| R49, R50 | `ledger_regressed` | `conflict` |
| R51 | `ledger_storage_unsupported` | `state` |

R40 uses the codes of SPEC-0020. R52 returns the recorded result, not an error.

## Examples

`examples/` holds fixtures computed with the hashing rule above, and `.gitattributes` keeps
their bytes as written on every platform. The ledgers form one story: issue 12 worked in two
sessions by two executors, a ruling, a failed and a passed check run on two heads, the wait at
`merge`, a review, the merge, the summaries and their publications. They hold the kinds owned by
this specification plus the `limits_set` and `plan_observed` of SPEC-0002 that the leases, the
budgets and the milestone membership read; the other kinds of SPEC-0002, such as the steps and
the gates, change no summary and are left out.

| File | Expected result |
| --- | --- |
| `ledger.valid.jsonl` | Valid; 32 events; revision of AC7; no lease ends between two events of one session |
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
- an issue is claimed by `checkout_acquired` and released by `checkout_released`, whose reasons
  include `lease_expired` and `awaiting_merge`; the lease is computed only by `due_events`;
- every event caused by an input carries its cause: `input_hash`, `dispatch` or `external_id`;
- `result_rejected` counts as one retry of the step, and `step_failed` records the exhaustion of
  the retries; repeated failures of checks are counted by `attempt_failed`;
- approvals and answers arrive as `approval_recorded` and `question_answered` with a person as
  actor; questions are gates of kind `question`; an approved plan change is a `gate_resolved`;
- staleness of an expected revision is scoped as in SPEC-0002 section Revisions;
- a broken or regressed ledger is the stop reason `ledger_broken`.

## Interfaces
- Ledger files under `<git common dir>/wayloop/` with the layout of section Storage. New.
- Schemas: `schemas/events/<kind>/v1.json` for each kind owned here; `schemas/ledger/summary/v1.json`; `schemas/ledger/snapshot/v1.json`. New; every later change is a new snapshot.
- Configuration: `[ledger]` and `[ledger.budgets]` of `wayloop.toml` (ADR 0010), keys of section Budgets; the budgets and the lease reach the ledger through `limits_set` (SPEC-0002). New.
- Commands, to be added to SPEC-0004: `wayloop history`, `wayloop rulings` (existing), `wayloop ledger verify [--full]`, `wayloop ledger snapshot`, `wayloop ledger recover --confirm <hash>` (person only, section Person-only commands), `wayloop index rebuild`, `wayloop checkout`, `wayloop handover`, `wayloop summary <level> <subject>`. All results that read state report `meta.revision` (SPEC-0004 R12).
- Error codes of section Errors. New.
- Every addition is additive. No released contract changes.

## Dependencies
- SPEC-0001 is Approved: the hand-over record, its identifier and `tokens`, and the field `source` of errors come from it.
- SPEC-0002 owns `decide`, `apply` and `due_events`, the payloads of its kinds in the catalogue, and the scoped rule of revisions.
- SPEC-0004 R11, R12 and R15 fix `input_hash`, the revision and the absence of the time of the call in envelopes; this specification relies on them.
- SPEC-0020 R4 provides the secret check called by R40; until SPEC-0020 is approved, R40 cannot be verified, and AC38 waits for it.
- SPEC-0005 performs the tracker calls of publication and reports the author of a comment.
- ADR 0013 is Proposed; the worktree name in a checkout follows it.

## Plan and tasks
`plan.md` and `tasks.md` are written after the maintainer approves this specification
(`specs/README.md`, Agent workflow).

## Open questions
- [x] **Q1. Retention of closed segments after a snapshot.** Decided by the maintainer on 2026-11-17: (a), every segment is kept uncompressed and deleted only on an explicit command of a person, recorded as an event.
- [x] **Q2. Recovery of a broken chain.** Decided by the maintainer on 2026-11-17: (a), `wayloop ledger recover` run by a person, with the confirmation of the last kept hash, and with breaks in closed segments treated by quarantining every later segment (R34, section Integrity).
- [x] **Q3. Where the milestone summary is published.** Decided by the maintainer on 2026-11-17: (b), one comment on the tracking issue of the milestone, on the same path as an issue summary.
- [x] **Q4. Who writes summaries.** Decided by the maintainer on 2026-11-17: (a), the deterministic fill rule of section Roll-up, with prose only from hand-overs.
- [x] **Q5. Default values.** Decided by the maintainer on 2026-11-17: (a), the proposed defaults, with the heartbeat every quarter of the lease and the line limit fixed in the genesis event; to be revised with the measures of the benchmark (SPEC-0019).
- [x] **Q6. Verification on load.** Decided by the maintainer on 2026-11-17: verify the events after the latest snapshot on every load, and every segment when a session starts and with `wayloop ledger verify --full`. Load time stays bounded by `snapshot_every`, and an edit before the snapshot is found before the next session acts on the state, in either mode of the runtime, not only when `wayloop run` starts (R32).
- [x] **Q7. Identity of a ruling.** Decided by the maintainer on 2026-11-17: (a), the hash of its `ruling_recorded` event, with no new identifier type.
- [x] **Q8. Repair of a torn tail.** Decided by the maintainer on 2026-11-17: (a), automatic, by the writer only, under its lock; readers ignore an unterminated last line (R20).
