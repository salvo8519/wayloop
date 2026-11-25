# SPEC-0002 and SPEC-0003: Plan of the smallest vertical slice of the issue cycle

| | |
| --- | --- |
| Status | Draft |
| Specifications | SPEC-0002 and SPEC-0003, Draft, reopened on 2026-11-25; this plan starts when the maintainer approves the amended versions |
| Issues | #48 (transition table and guards), #49 (next step and ready work), #50 (append-only writer), #51 (projection and replay, in part), #58 (dry run on local fixtures), #161 (pull request events), #162 (checkout and lease, in part), #347 (artifact transitions as data) |
| Prerequisites | #228 (done); SPEC-0001 T1 to T12 (done): canonical JSON, SHA-256, envelope, schema interpreter, contract generator |

This plan builds the smallest vertical slice of the issue cycle, decided by the maintainer on
2026-11-25 from section 4 of the counter-review of SPEC-0002 and SPEC-0003: its requirements in
and out, and its order of steps 0 to 9, ending with #58. One plan covers both specifications,
because the slice is one path through both: `specs/0003-ledger/spec.md` points here. The
decisions D1 to D3 were adopted by the maintainer on 2026-11-25.

## What the slice proves

One issue of tier `substantial`:

- claim, then reanalysis, counter-review and checklist;
- build, a failed verify, build again, a passing verify;
- review with one verifier on the `proof` lens;
- pull request, checks, merge gate, approval, merge, `done`.

It also covers a lease expiry and a resumed claim; a stale dispatch, a stale revision and a
duplicate observation; the tier `small` merged under the policy waiver; and the replay of the
produced ledger to a byte-identical state. That covers the four scenarios of
`specs/0002-state-machine/examples/`, rewritten to the trace format of SPEC-0002 R58.

## Scope of the slice

### SPEC-0002

| In the slice | Notes |
| --- | --- |
| R1 to R7 | R5 without `now`; R6 for commands only; R7 with the configuration recorded by `wayloop init` only, given by fixtures |
| R9, R10, R11 | Tiers `substantial` and `small` only |
| R13 to R16 | For the merge gate and question gates only |
| R17 to R27 | |
| R28 | The hold only; its removal by `limits_set` is given by a fixture |
| R29 | Provider tokens for the sprint and milestone scopes, and `budget_exhausted` |
| R30 to R39 | |
| R41, R42 | R42 with the stop reasons of the slice, `ledger_broken` and `awaiting_checks` included |
| R43 | The claim cascade and `done` |
| R44, R49 (tier `substantial`), R52, R53 | |
| R54 to R59 | The patterns of section Plan rules |
| Sessions S1 to S6 | Checkout, lease and hand-over of #162 |
| The baseline `plan_observed` | Readiness, lanes and `coverage_planned`; later plan observations (items 16 to 19) are out |

Out of the slice, built later in M1 or M2: R8 (panel of tier `contract`); R12, and R13 to R16 for
gates other than merge and question; R40 (superseded task); R45 to R48 (configuration gate,
runs, phase, council and plan); R50 and R51 (stale coverage); later plan observations; the
project transitions P1 to P13. Since R46 is out, the slice records no `run_started` and
`next_steps` does not yet report `run_not_started`; the scenarios gain a `run_started` when R46
is built.

### SPEC-0003

| In the slice | Notes |
| --- | --- |
| R1 to R4 | |
| R9 to R12 | Pull request events of #161; checkout, lease and hand-over of #162 |
| R14, R15 | `coverage_planned` from the baseline plan or fixtures |
| R17 to R25 | R22 with the kinds of the slice only (counter-review item 78) |
| R27 to R30 | |
| R40 | An interface whose check finds nothing; AC38 waits for SPEC-0020 |
| R43 | Without the session summary |
| R45 to R48 | R47 without the quarantine |
| R52, R56 to R63 | |
| R54 | The fold version constant and its golden test (AC67) only |

Out of the slice: R5 and R42 (rulings); R6, R35 to R39, R41, R44 and R55 (summaries, resume pack
and publication, #52); R7, R31, R32 and the rest of R54 (snapshots: they can follow #58 within
M1, because the gate of M1 needs only a full replay); R8 and R53 (migration); R13 (resume pack);
R16, R26, R33, R34, R49, R50 and R51.

### Event kinds registered by 0.1.0

The slice registers only these kinds (SPEC-0003 R22, counter-review item 78): `ledger_created`,
`min_engine_version_raised` (so that 0.1.0 reads the minimum before a kind it does not know:
second counter-review B9, decided by the maintainer on 2026-11-25), `session_started`, `session_ended`, `checkout_acquired`, `lease_renewed`, `checkout_released`,
`handover_recorded`, the seven pull request kinds, `executor_call_recorded`, `result_rejected`,
`torn_tail_discarded`, `limits_set`, `policy_set`, `plan_observed`, `branch_changes_observed`,
`gate_opened`, `gate_resolved`, `gate_withdrawn`, `gate_waived`, `gate_bypassed`,
`approval_recorded`, `question_answered`, `input_ignored`, `step_started`, `step_completed`,
`step_retried`, `step_failed`, `attempt_failed`, `issue_blocked`, `issue_unblocked`,
`issue_done`, `artifact_status_changed`, `budget_exhausted`, `budget_restored`,
`reconciliation_required`, `coverage_planned` and `coverage_verdict`: 44 kinds. Every other kind
of the catalogue raises the minimum engine version when it is first written (SPEC-0003 R53).

### Questions that block the slice

The maintainer answered on 2026-11-25 every question that blocked the slice: SPEC-0002 Q14,
Q15, Q16, Q17, Q19 and Q20, SPEC-0003 Q15 and Q16, and D1 to D3 below, each with the
recommendation of the specification, and Q21 with its option (a) (decided by the maintainer on
2026-11-25). SPEC-0002 Q18 stays open, outside the slice. No question blocks the slice:

| Question | Blocks |
| --- | --- |
| None | |

## Design

### Boundary

| Step | Where | Why |
| --- | --- | --- |
| Contract types: event header, payloads, value types, `found`, `Source` `{ledger, line}` | `wayloop-schemas` | ADR 0030 G1: one source for every contract |
| Canonical bytes, hashing, line and chain verification, finishing events | `wayloop-core::ledger` | Decision logic, pure (SPEC-0003 section One writer) |
| Allowed status transitions of SPEC-0001 | `wayloop-core::artifacts::transitions` (#347) | Used by the guards of SPEC-0002 and by SPEC-0001 R13 |
| State, `apply`, invariants, transition table, guards, `decide`, `genesis`, `due_events`, `next_steps` | `wayloop-core::state` | All decision logic in the core (constitution IV) |
| Clock, lock, files, flush, storage interface, loader, writer loop | `wayloop-runtime::ledger` | I/O only; decides nothing (ADR 0006) |
| Scenario runner, fake executor and fake tracker | `wayloop-runtime` tests | A test harness of #58 |

### Reuse

- `wayloop_core::canonical` and `wayloop_core::sha256` (SPEC-0001 T2) compute every hash; the
  type `InputHash` of `wayloop-schemas` becomes the general SHA-256 type, and `Revision`,
  `DispatchRef` and `GateRef` wrap it.
- `IssueStep` of `wayloop-schemas` (#228) is the source of the steps; `issue_state` is built from
  `IssueStep::ALL` by one helper of `contract` (counter-review item 66).
- The error report, `Code`, `Reason` and `Source` of `wayloop-schemas` carry every refusal; the
  member `found` is added once (counter-review item 65).
- The schema interpreter of the core validates every input and every event read back, so that
  every error is reported in one pass (ADR 0030 G1).
- The producer of branch names of SPEC-0001 R11 (`naming::branch_name`, SPEC-0001 T17, #267)
  derives the branch of a checkout from the `branch_type` of the task, `feat` while the task has
  none (SPEC-0002 Q16, decided by the maintainer on 2026-11-25; the name of the member, second
  counter-review B10). #267 is open, so T6a depends on it.
- `contract_enum!` and the registry `contract::CONTRACTS` declare every new enum and contract.

### Module layout

```text
crates/wayloop-schemas/src/
  time.rs                 Timestamp
  hash.rs                 Sha256 (from InputHash), Revision, DispatchRef, GateRef
  tokens.rs               ProviderTokens, SizeTokens
  event/mod.rs            Event<P>, Actor, the cause of R3
  event/kinds.rs          one payload type per kind of the slice
  state.rs                contract enums: issue_state, hold_reason, stop_reason, reconciliation_code, ...
  error.rs                Error gains `found`; Source gains {ledger, line}
crates/wayloop-core/src/
  ledger/encode.rs        canonical line, hash, finishing of events (R1 of SPEC-0002, R58)
  ledger/verify.rs        line, chain and genesis checks, Integrity
  artifacts/transitions.rs  SPEC-0001 transitions as data (#347)
  state/model.rs          State, Issue, Stage (illegal states unrepresentable)
  state/apply.rs          apply
  state/invariants.rs     State::check_invariants
  state/transitions.rs    TRANSITIONS: &[Transition], TransitionId
  state/guards.rs         one named predicate per guard, each with its Code
  state/decide.rs         decide
  state/due.rs            due_events
  state/next.rs           next_steps
crates/wayloop-runtime/src/ledger/
  storage.rs              trait Storage and the file system implementation
  lock.rs                 flock, LockFileEx
  writer.rs               the writer loop under the lock
  loader.rs               load, verify, fold, Integrity
  projection.rs           state.yaml
```

### Signatures

```rust
// wayloop-core::state
pub fn decide(state: &State, input: &Input, at: Timestamp) -> Result<Vec<Event>, ErrorReport>;
pub fn apply(state: State, event: &Event) -> State;
pub fn due_events(state: &State, now: Timestamp) -> Vec<Event>;
pub fn next_steps(state: &State, integrity: Integrity, free_workers: u32) -> NextSteps;
pub fn genesis(configuration: &Configuration, ledger_id: LedgerId, at: Timestamp)
    -> Result<Vec<Event>, ErrorReport>; // ledger_created, limits_set, policy_set (S2)

pub enum Integrity { Valid, Broken, Regressed }

pub struct Transition {
    pub id: TransitionId,          // I0 to I51, S1 to S6, G1 to G7, P1 to P13
    pub from: &'static [From],     // source states, as in the From column
    pub input: InputKind,
    pub selectors: &'static [SelectorId], // choose the row; no code (R55)
    pub guards: &'static [GuardId],
    pub to: Target,
    pub records: &'static [EventKind],
}

pub struct Guard { pub id: GuardId, pub code: Code, pub check: fn(&State, &Input) -> bool }
pub struct Selector { pub id: SelectorId, pub check: fn(&State, &Input) -> bool }

// wayloop-runtime::ledger
pub trait Storage {
    fn read(&self, path: &LedgerPath) -> io::Result<Vec<u8>>;
    fn append(&mut self, path: &LedgerPath, bytes: &[u8]) -> io::Result<()>;
    fn flush(&mut self, path: &LedgerPath) -> io::Result<()>;
    fn truncate(&mut self, path: &LedgerPath, len: u64) -> io::Result<()>;
    fn rename(&mut self, from: &LedgerPath, to: &LedgerPath) -> io::Result<()>;
    fn lock(&mut self, timeout_seconds: u32) -> io::Result<LockGuard>;
}
```

The shapes above are the intent; names may change in review, the rules of section Plan rules may
not.

### Order inside `decide`

1. The class of the input (R4); for an observation, I0 (R33).
2. The source `issue_state` of the row and `step_mismatch` (R6, level 1): the first failure
   stops. A condition on the checkout is not checked here (second counter-review B2).
3. The selectors of the rows of that source state and input pick exactly one row (R55).
4. The guards of the row, in the order of the Guard column, collected in one pass (R6, level 2).
5. The effects: the events of the column Recorded, in order; each event is built, its size is
   checked against the line limit before it is hashed (SPEC-0003 R59, item 74), then it is
   finished from the last `(seq, hash)` and applied before the next is computed (R1, item 1;
   the order of the size check decided by the maintainer on 2026-11-25, second counter-review
   S12).

The writer, in the runtime, calls in this order under the lock: catch up (SPEC-0003 R18),
repair a torn tail (R20), `due_events` and append (R59 of SPEC-0002), the retry check (R52),
the revision check (R41), `decide`, append, flush, projection, release.

## Plan rules

The code patterns of section 4 of the counter-review, decided by the maintainer on 2026-11-25.
Patterns 1, 3, 4, 6, 7 and 9 are also requirements (SPEC-0002 R54 to R58, SPEC-0003 R62); each
task applies them, and the review checks them.

1. **Transitions as data.** One constant table `TRANSITIONS` in `state/transitions.rs`, one row
   per transition with its `TransitionId`. `decide` looks the row up, evaluates its guards and
   applies its effects; it never encodes a transition in a `match` of its own. A test reads the
   tables of `spec.md` and keeps the code table equal to them (AC54, R54).
2. **Exhaustive matches.** The modules `state` and `ledger` of the core deny
   `clippy::wildcard_enum_match_arm`: a new state, input or event kind fails to compile until
   every match names it.
3. **Illegal states unrepresentable.** The data of an issue hangs on its stage: no checkout at
   `merge`, `done` or `superseded`; a dispatch only from `claim` to `pull_request`; holds as a
   `BTreeMap` keyed by `(reason, gate)`. `State::check_invariants` checks the invariants of
   SPEC-0002 section Invariants and runs after every `apply` in tests (R56, AC56).
4. **Guards as named predicates.** Each guard is a `Guard` with its `GuardId` and one `Code`
   constant; each selector is a `Selector` without a code; a test checks the map from guards to
   the table Errors both ways (R55, AC55), and AC1 checks that the selectors of one source state
   and input are exclusive and complete (second counter-review B6).
5. **Distinct types.** `Timestamp`, `Revision`, `DispatchRef`, `GateRef`, `SessionId`,
   `ProviderTokens` and `SizeTokens` are newtypes of `wayloop-schemas`; no function takes a bare
   string or integer for one of them (SPEC-0003 section Contracts of the ledger).
6. **Property tests for the invariants**, over sequences of inputs decided and applied from the
   initial state (never over random states): replay equals the state of the writer (AC47, AC1 of
   SPEC-0003); `apply` never panics (AC57); the output does not depend on the order of maps
   (AC3); one issue per lane and no more checkouts than workers (AC20); no new dispatch while an
   issue is held (AC56, invariant of R30); no gate resolved without an approver (AC13);
   observations are idempotent (AC35); attempts are bounded (invariant 7).
7. **A storage interface** in the runtime for fault injection: every file operation of the writer
   goes through `Storage` (SPEC-0003 R62, AC63); the tests inject failures at each operation and
   at each byte of an append (AC22, AC48, AC64, AC65).
8. **A mutation-score floor** for `state/transitions.rs` and `state/guards.rs`, checked by
   `just mutants` before the pull requests of T6a, T6b, T7a, T7b and T7c: no surviving mutant, except the
   equivalent mutants listed in the configuration of `cargo-mutants` of the repository, each with
   a comment that says why it is equivalent, reviewed in the pull request that adds it (D3).
9. **A trace schema and a runner** for `examples/` (R58, AC43), built in T11; the schema is a test
   fixture, `specs/0002-state-machine/examples/trace.schema.json`.

Also: `BTreeMap` and `BTreeSet` only in the pure crates; the `clippy.toml` of AC2 disallows the
clock, the environment, the file system, random sources and the hashed collections in the core.

## Decisions

Decided by the maintainer on 2026-11-25: every recommendation below is adopted (D1 (a), D2,
D3 (a)).

- **D1. Issues for the tasks.** A pull request closes exactly one issue (AGENTS.md). (a) One new
  issue per task that has none, created with `scripts/roadmap.py new` under the epics #21 and
  #22, as SPEC-0001 D7 did; #48 (T6b), #49 (T8), #50 (T9), #58 (T11), #161 (T7b) and #347 (T4)
  are closed by the tasks named in `tasks.md` (T2, T6 and T7 split into T2a, T2b, T6a, T6b,
  T7a, T7b and T7c, each with its own issue: second counter-review S10), and #51 and #162 stay open for the parts after the
  slice. (b) Intermediate pull requests cite `Refs #<n>` and close nothing. Decided: (a). The pull
  request of T0 cites `Refs: #48` and does not close #48.
- **D2. Roadmap order.** #161 and #162 are needed before #58 (counter-review section 4). The
  slice folds their minimal parts into T2b, T7a to T7c and T9; #161 closes with T7b, #162 stays open for
  the resume pack (SPEC-0003 R13, R44) and the publication of hand-overs (#52). Decided: #347 is
  in the roadmap; the order "#161 before #58" is written in the body of #58 by the orchestrator
  with `scripts/roadmap.py edit`.
- **D3. Mutation floor of the transition module (plan rule 8).** (a) No surviving mutant in
  `state/transitions.rs` and `state/guards.rs`, with a reviewed list of equivalent mutants;
  (b) 90 percent of mutants caught; (c) measure in T6 and set the floor in T7. Decided: (a): the
  table is data and every row and guard has a positive and a negative case (AC1), so a surviving
  mutant is a missing test or an equivalent mutant to list.

## Contract changes

All new or additive; no released contract changes. Every task that adds a snapshot under
`schemas/` is a contract change: review plus the maintainer's approval.

- `envelope/result` v1, which no release holds: `Error.found` (optional, any JSON value) and the
  form `{ledger, line}` of `Source` (SPEC-0003 Q10). Additive.
- `events/<kind>/v1` for the 44 kinds of the slice: new contracts of the new area `events`.
- `artifacts/handover` v1 and `v1.input`: the descriptions of the `step` values (T1), under the
  version rules of ADR 0030.
- Contract enums `issue_state`, `hold_reason`, `stop_reason`, `reconciliation_code`,
  `gate_kind`, `gate_state`, `resolution`, `question_category`, `action_kind`, `risk_tier`,
  `integrity`: new, inside the event contracts and, later, the outputs of `wayloop next` and
  `wayloop status` (SPEC-0004).
- Error codes: the codes of SPEC-0002 Errors and SPEC-0003 Errors used by the slice, as constants
  of `Code`; listed in the reference page of errors.
- No command contract is added: `wayloop next`, `status` and `history` belong to SPEC-0004
  (#57). The slice drives the core through the runtime in tests. The clauses of SPEC-0002 AC45
  and AC61 that need the input schema of a command at the entry point are therefore out of the
  slice (second counter-review S3).

## Test strategy

- **Test first.** Every acceptance criterion is a test named after it (`ac23_stale_dispatch`),
  written and committed before the code, failing for the expected reason. Expected values come
  from the specifications, never from running the code.
- **Techniques.** TDD for the rows and guards; property tests for the invariants of plan rule 6;
  contract tests for the snapshots, the table equality (AC54) and the fixtures of SPEC-0003
  `examples/`; end-to-end for the scenario runner (AC43) and the two-process writer (AC13 of
  SPEC-0003); manual check for AC52 of SPEC-0003.
- **Oracles.** The fixtures of `specs/0003-ledger/examples/` are computed with the hashing rule
  independently of the code; T3 regenerates the ledger fixtures with a script that does not use
  the crates it tests, before it writes the code (B11); the golden ledger of T10 is small and every value is reviewed by a
  person before it is committed (AC21, counter-review section 3).
- **Fault injection.** The test storage of plan rule 7 fails or stops at every operation and
  every byte offset.
- **Core quality.** The line coverage floor of the pure crates (95 percent) holds after every
  task; `just mutants` meets the floor of D3 on the transition module.

Acceptance criteria per task:

| Task | SPEC-0002 | SPEC-0003 |
| --- | --- | --- |
| T1 | (descriptions of `IssueStep`) | |
| T2a | (value types, header, `found` and `Source`) | AC61 |
| T2b | (payloads of the 44 kinds) | AC46 |
| T3 | AC58 (finishing part) | AC2, AC10, AC11, AC12 (genesis and format), AC17, AC26, AC49, AC50, AC56, AC57, AC58, AC65 (malformed line), each on the regenerated fixtures (B11) |
| T4 | (dependency of P10, P11, I34, R43) | |
| T5 | AC56, AC57, AC2 (`apply`) | AC16 (state part) |
| T6a | AC1 (rows I0 to I4), AC18, AC22, AC31 (first half), AC40, AC44, AC45 (first two clauses), AC47 (causes), AC52, AC53, AC54, AC55, AC58, AC62, AC63, AC64 (except the clause of `branch_type` `fix`), AC79 | AC14 (revision rule) |
| T6b | AC1 (rows I5 to I14, I27 to I29, I38, I46, I48, I50), AC5, AC6, AC7, AC23, AC25, AC26, AC27, AC28, AC29, AC31 (second half, by a fixture `limits_set`), AC32 (second half and the clause of S8, by a fixture `limits_set`), AC51, AC59, AC60, AC67, AC78 (except the clause of the answer), AC80 | AC9, AC16, AC43 (refusal), AC59, AC72 |
| T7a | AC1 (rows I15 to I17, I20 to I23, I39 to I44, I47, I49, I51), AC35, AC38, AC65, AC66, AC68 (except its last clause), AC74, AC75 | AC27 |
| T7b | AC1 (rows I18, I19, I24 to I26, I32 to I37, I45, G1 to G7 for merge and question gates), AC13, AC14, AC17, AC30, AC33, AC34, AC36, AC37, AC68 (last clause), AC69, AC70 (first and third parts), AC72, AC73, AC76, AC78 (clause of the answer, and the clause of `review_left`), AC84 (its clauses of I16 and I39 rest on T7a) | |
| T7c | AC1 (rows I30, I31, S1 to S6), AC71, AC81 | AC44, AC45, AC47 (without the quarantine) |
| T8 | AC3, AC19, AC20, AC21, AC24, AC39, AC41, AC77, AC82 | AC5 (decision part), AC41 (due events part), AC70 |
| T9 | AC61 (except its last clause) | AC13, AC14, AC15, AC22 (rewrite), AC24, AC48, AC63, AC64, AC65, AC68, AC71 |
| T10 | AC46, AC47 (replay) | AC1, AC21, AC22 (mismatch), AC67, AC69 |
| T11 | AC43 | |
| T13 (after the slice) | AC64 (clause of `branch_type` `fix`) | |

Every row of the tables Issue transitions and Session transitions, and every row G1 to G7 for
merge and question gates, is in exactly one AC1 list above (second counter-review S11).

Out of the slice, with their requirements: SPEC-0002 AC4, AC8 to AC12, AC83 (Q18 and Q22, with the member `unassigned_provider_tokens` of the state), AC15, AC16, AC42, AC45
(last clause: the input schema of a command at the entry point, SPEC-0004), AC48, AC49, AC50,
AC61 (last clause: an input time with a fraction refused by the schema of its command), AC70
(superseded part); SPEC-0003 AC3, AC4, AC6 to AC8, AC18 to AC20, AC23, AC25,
AC28 to AC37, AC38 (waits for SPEC-0020), AC39, AC40, AC41 (summary part), AC42, AC47 (quarantine
part), AC51 to AC55, AC60, AC62, AC66.

## Documentation

- `docs/en/reference/errors.md` (or the error page of the reference): every code of the slice
  with its reason and severity (T2a, T6a to T7c, T9); Italian edition stamped.
- `docs/en/concepts/architecture.md`: the state module, the transition table as data, the
  storage interface of the runtime (T5, T9); Italian edition.
- The concept pages of the issue cycle and of the ledger belong to #59 (engine documentation),
  after the slice.
- A fragment in `changes/` for every task with a visible effect.

## Risks and alternatives

- **The table drifts from the specification.** Mitigation: AC54 reads `spec.md`; a change of a
  row needs a change of the specification first (specs/README.md, rule 6).
- **`decide` grows beyond one task.** The rows are split between T6a (table and guard machinery,
  claims and step requests), T6b (steps, results, retries), T7a (pull request and checks), T7b
  (merge gate and holds) and T7c (sessions), each at most size M; all use the table, guard and
  selector types of T6a (second counter-review S10).
- **The projection is slow under the lock.** Every append rewrites `state.yaml`, and the state
  grows with every `external_id`. Mitigation: T10 measures the rewrite on a ledger of 10,000
  events against `lock_timeout_seconds` (10 seconds) and reports it in its pull request;
  SPEC-0003 Q16 decides the content.
- **Approval first.** Every task can start once the maintainer approves the amended
  specifications; no question of the slice is open.
- **Scenarios written before the amendments.** The files of `examples/` are not normative until
  T11 rewrites them; T6a to T7c test the rows with their own cases. The ledger fixtures of
  SPEC-0003 are not normative until T3 regenerates them (SPEC-0003 section Examples, second
  counter-review B11).
- **Rejected alternative: `decide` as nested matches without a table.** It cannot be compared with
  the specification by a test, and AC1 could not enumerate its rows (counter-review section 3).
- **Rejected alternative: `apply` that refuses inconsistent events.** A recorded event is a fact;
  refusing it would make replay depend on the order of checks (counter-review item 8).
