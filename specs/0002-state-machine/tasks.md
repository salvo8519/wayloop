# SPEC-0002 and SPEC-0003: Tasks of the smallest vertical slice

Rules: one task per pull request; `[P]` tasks touch disjoint files and can run in parallel; every
task names its verification technique; tick a task only when `uv run just check` passes. Tests are
written and committed first, and fail for the expected reason before the code exists.

Size: S up to about 300 changed lines with tests, M up to about 800. No task is larger.

Merge: a task marked **Maintainer** changes `crates/` or `schemas/` and waits for the
maintainer's confirmation (AGENTS.md, Merging); one with new snapshots under `schemas/` is a
contract change and needs the maintainer's approval. The decisions D1 to D3, adopted by the
maintainer on 2026-11-25, are in `plan.md`; no open question blocks a task.

Steps: the order 0 to 9 of the slice decided by the maintainer on 2026-11-25 (counter-review
section 4). A step may hold more than one task, so that no task exceeds size M.

Issues: under D1, a task marked "new issue" gets one with `scripts/roadmap.py new` before it
starts; "part of" names the issue that the last task of the group closes.

Prerequisites outside these specifications:
- The maintainer approves the amended SPEC-0002 and SPEC-0003 (after T0). Every question of the
  slice was answered on 2026-11-25.
- #267 (SPEC-0001 T17, the producer of branch names) before T6a.
- The body of #58 names #161 before it (D2, an edit by the orchestrator).

## Step 0: specifications

- [ ] **T0 Amend SPEC-0002 and SPEC-0003 with the answers to the counter-review, and write the plan of the slice**
  - Covers: every item of the counter-review with a recommendation; the SPEC-0001 amendments of items 25 and 27; every item of the second counter-review (B1 to B12, S1 to S12, N1 to N5), with the SPEC-0001 amendments of B1 and B10 and the amendments of ADR 0009 of S5; `plan.md` and `tasks.md`. Issue: new issue (D1); `Refs: #48`.
  - Files: `specs/0001-artifact-model/spec.md`, `specs/0002-state-machine/spec.md`, `specs/0002-state-machine/plan.md`, `specs/0002-state-machine/tasks.md`, `specs/0003-ledger/spec.md`, `docs/en/project/adr/0009-contract-guidelines.md` and its Italian edition.
  - Tests first: none (specification).
  - Verification: none; `uv run python scripts/docs_sync.py coverage`, `uv run python scripts/roadmap.py check` and `uv run just check` pass.
  - Depends on: the decisions of the maintainer of 2026-11-25.
  - Size: not measured in code: specification text only. Small tier (documentation); the maintainer approves the specifications.

- [ ] **T1 Align the descriptions of IssueStep with the issue states and ADR 0029**
  - Covers: SPEC-0001 (amended on 2026-11-25), counter-review section 2 (step descriptions); SPEC-0002 table Issue states. Issue: new issue (D1).
  - Files: `crates/wayloop-schemas/src/cycle.rs`, the generated snapshots of `artifacts/handover` (`schemas/artifacts/handover/`, `crates/wayloop-schemas/snapshots/`).
  - Tests first: a test that the description of every `IssueStep` value equals the column Meaning of the table Issue states of `specs/0002-state-machine/spec.md`, read from the file; the description of `merge` names the waiver of ADR 0029.
  - Verification: contract test.
  - Depends on: T0 approved. Not parallel with T2a or T2b: they regenerate the embedded snapshots of `wayloop-schemas`.
  - Size: S. Maintainer (crates, contract change).

## Step 1: contracts (#50, part 1)

T2 is split in two, so that no task exceeds size M (second counter-review S10).

- [ ] **T2a Add the value types, the event header and the members found and Source to the contracts**
  - Covers: SPEC-0003 section Common fields, Contracts of the ledger (value forms, `Event<P>`, `found` with one meaning and its shape per code, `Source` `{ledger, line}`), R61; SPEC-0002 section Events (the header and the value types), counter-review items 56, 57, 65, 66 and plan rule 5; ADR 0009 C3 as amended on 2026-11-25 (`found`); the contract enums of `plan.md`; SPEC-0003 AC61. Issue: new issue (D1), part of #50.
  - Files: `crates/wayloop-schemas/src/{time,hash,tokens,state}.rs`, `crates/wayloop-schemas/src/event/mod.rs`, `crates/wayloop-schemas/src/error.rs` (`found`, `Source` `{ledger, line}`), `crates/wayloop-schemas/src/contract/` (registry, the new areas `events` and `ledger`, the helper of `issue_state`), `schemas/envelope/result/`, `crates/wayloop-schemas/snapshots/`, `docs/en/reference/errors.md` and its Italian edition.
  - Tests first: `Revision`, `DispatchRef` and `GateRef` do not convert into each other; `ProviderTokens` and `SizeTokens` do not either; a `Timestamp` with a fraction of a second, or without `Z`, does not deserialise; `issue_state` lists `open`, the values of `IssueStep::ALL` and `done`, `superseded`, from one source; `Error.found` is left out when absent and never `null`; the version rule of AC61 of SPEC-0003.
  - Verification: contract test.
  - Depends on: T0 approved.
  - Size: M. Maintainer (crates, contract change).

- [ ] **T2b Add the payloads of the 44 kinds of the slice to the contracts**
  - Covers: SPEC-0003 section Payloads, R9 (payloads of #161), R10 to R12 (payloads of #162), R45 (`model_turns`); SPEC-0002 section Events (payloads of the kinds of the slice); the 44 kinds of `plan.md`, `min_engine_version_raised` included (second counter-review B9); SPEC-0003 AC46. Issue: new issue (D1), part of #50.
  - Files: `crates/wayloop-schemas/src/event/kinds.rs`, `crates/wayloop-schemas/src/contract/` (one contract per kind), `schemas/events/<kind>/v1.json` for the 44 kinds of `plan.md`, `crates/wayloop-schemas/snapshots/`.
  - Tests first: every payload of the slice serialises to its schema and back; a kind outside the 44 is not registered; a `pull_request_checks_completed` whose checks are not sorted is refused with `syntax` (AC46); `checkout_released` has no reason `awaiting_merge`; `executor_call_recorded` has `model_turns` and no `turns`.
  - Verification: contract test.
  - Depends on: T2a.
  - Size: M. Maintainer (crates, contract change).

## Step 2: ledger encoding in the core (#50, part 2)

- [ ] **T3 Encode, hash, finish and verify ledger lines in the core**
  - Covers: SPEC-0003 R2, R4, R22 (the minimum from the last `min_engine_version_raised`, B9), R23, R29, R57 to R60, hashing rule, integrity table, section Examples (regeneration of the ledger fixtures, second counter-review B11); SPEC-0002 R1 (finished events, item 1); SPEC-0003 AC2, AC10, AC11, AC12 (genesis and format, without `wayloop init`), AC17, AC26, AC49, AC50, AC56, AC57, AC58, AC65 (malformed line that is not the last); SPEC-0002 AC58 (finishing). Issue: new issue (D1), part of #50.
  - Files: `specs/0003-ledger/examples/` (the ledger fixtures `ledger.*.jsonl`, regenerated, and the script that regenerates them), `specs/0003-ledger/spec.md` (the revision of AC56, the line numbers of AC2, AC10, AC11, AC15, AC50, AC58 and of the table of section Examples, and the removal of the note that the fixtures are not normative), `crates/wayloop-core/src/ledger/{mod,encode,verify}.rs`, `crates/wayloop-core/src/lib.rs`, `crates/wayloop-core/tests/` (fixtures of `specs/0003-ledger/examples/`).
  - Tests first: first, as its own commit, regenerate the ledger fixtures to the amended payloads, with the kinds that 0.1.0 registers only (no `ruling_recorded`, `summary_recorded` or `summary_published`), a first `policy_set` with `project_name` after the first `limits_set`, no `budgets.intake`, the external identifier `plan:<previous>:<sha>:<part>` with `previous_plan_sha256`, `model_turns`, and the release `awaiting_checks` caused by the `step_result` of `pull_request` (its `dispatch`, not an `external_id`), with a script that does not use the crates under test, and state the new revision in AC56; then one test per AC above, with the regenerated files as fixtures; `finish(events, last)` gives consecutive `seq`, chained `prev_hash` and hashes that verify; `Integrity` is `broken` with the first broken line in `found`. The summary fixtures stay as they are, outside the slice (#52).
  - Verification: TDD, contract test, property test (AC57).
  - Depends on: T2b.
  - Size: M. Maintainer (crates).

## Step 3: artifact transitions as data (#347)

- [ ] **T4 [P] Hold the allowed status transitions of SPEC-0001 as data in the core**
  - Covers: SPEC-0001 Statuses and transitions of every kind (as amended on 2026-11-25: task `approved` to `done`, gate issue of a rejected gate); SPEC-0002 R43 and the guards `transition_refused` of P10, P11, I34; counter-review section 2 (`STATUSES` not built). Issue: #347.
  - Files: `crates/wayloop-core/src/artifacts/transitions.rs`, `crates/wayloop-core/src/artifacts/checks.rs` (`STATUSES` leaves `NOT_BUILT` only if T15 of SPEC-0001 uses it; otherwise unchanged), `crates/wayloop-core/src/artifacts/mod.rs`.
  - Tests first: a test that reads the lines "Transitions:" of every kind in `specs/0001-artifact-model/spec.md` and compares them with the table, each with its actor (person or engine); a transition outside the table gives `transition_refused` with the reason `state` and the allowed targets in `expected`.
  - Verification: contract test, TDD.
  - Depends on: T0 approved; disjoint from T3.
  - Size: S. Maintainer (crates).

## Step 4: state model and apply (#48, part 1)

- [ ] **T5 Model the state with its invariants and fold events with apply**
  - Covers: SPEC-0002 section Machines and state, Invariants, R2, R56, R57, plan rules 2, 3 and 6; SPEC-0002 AC2 (`apply` and the `clippy.toml` of the core), AC56, AC57; SPEC-0003 AC16 (state part). Issue: new issue (D1), part of #48.
  - Files: `crates/wayloop-core/src/state/{mod,model,apply,invariants}.rs`, `crates/wayloop-core/clippy.toml`, `crates/wayloop-core/src/lib.rs`, `docs/en/concepts/architecture.md` and its Italian edition.
  - Tests first: `check_invariants` refuses each invariant broken by a hand-built state; `apply` of every kind of the slice on the initial state, and of every event with an unknown issue or an absent checkout, returns a state without panic; the module denies `clippy::wildcard_enum_match_arm`.
  - Verification: TDD, property test.
  - Depends on: T2b, T3.
  - Size: M. Maintainer (crates).

## Step 5: decide from the transition table (#48, part 2)

T6 and T7 are split into five tasks, so that no task exceeds size M; each keeps its own issue
under D1 (second counter-review S10). Before the pull request of each, `just mutants` meets the
floor of D3 on `state/transitions.rs` and `state/guards.rs` (plan rule 8).

- [ ] **T6a Build the transition table, its guards and selectors, and decide claims and step requests**
  - Covers: SPEC-0002 R1, R3, R4, R6 (level 1 on `issue_state` only, B2), R18 to R20, R41, R43 (claim), R44, R52 to R55 (selectors without a code, B6), the function `genesis` (S2), rows I0 to I4 (I2 with the claim at `merge` refused by readiness, N3), section Names of a checkout (`feat` while the task has no `branch_type`, Q16 and B10; worktree from `project_name` of the first `policy_set` and the code `project_name_invalid`, Q21 and N4), section Revisions; the acceptance criteria of T6a in `plan.md`. Issue: new issue (D1), part of #48.
  - Files: `crates/wayloop-core/src/state/{transitions,guards,selectors,decide}.rs`, `crates/wayloop-core/src/state/revision.rs`, `crates/wayloop-core/src/state/config.rs` (`genesis`: `ledger_created`, the first `limits_set` and `policy_set` of `wayloop init`), `crates/wayloop-core/tests/state_table.rs` (AC54), `docs/en/reference/errors.md` and its Italian edition, `changes/`.
  - Tests first: one test per AC of T6a; AC54 reads the tables of `spec.md`; AC55 maps guards and codes, and selectors to no code; the cases of AC1 for the rows I0 to I4, with the exclusivity of their selectors.
  - Verification: TDD, property test, contract test (AC54).
  - Depends on: T4, T5; #267 (producer of branch names).
  - Size: M. Maintainer (crates).

- [ ] **T6b Decide the steps, their results and their retries**
  - Covers: SPEC-0002 R9 to R11, R21, R23 to R29 (R29 with the `limits_set` of S8 given by a fixture), R49 (the answer to a zero-unit question is in T7b), rows I5 to I14, I27 to I29, I38, I46, I48 (late skip of `review`, Q17), I50 (changed paths, in parts), the code `result_schema` (B7); SPEC-0003 R17, R21, R45, R59 (oversized results and commands; parts of `branch_changes_observed`, Q15); the acceptance criteria of T6b in `plan.md`. Issue: closes #48 (the transition table and its guards).
  - Files: `crates/wayloop-core/src/state/{transitions,guards,selectors,decide}.rs`, `crates/wayloop-core/src/state/results.rs`, `crates/wayloop-core/tests/state_table.rs`, `docs/en/reference/errors.md` and its Italian edition, `changes/`.
  - Tests first: one test per AC of T6b; the cases of AC1 for its rows.
  - Verification: TDD, property test.
  - Depends on: T6a.
  - Size: M. Maintainer (crates).

- [ ] **T7a Decide the pull request and its checks**
  - Covers: SPEC-0002 R35, R37, R38, rows I15 to I17, I20 to I23, I39 to I44 (I44 with the reason `awaiting_checks`, Q15, and only at `verified_head` without `head_pending`, B5; I22 and I39 at `verified_head` only, Q19; I42 for any conclusion, without counts, B3), I47 (any non-terminal state, B4), I49 (Q20), I51 (B3), I16 and I17 only at `verified_head`, with I39 for a passing check run at another recorded head (final check), the fields `checks`, `pending_checks`, `pull_request_step_completed` (set by I44, N-S2) and `pull_request_pass_completed` (set by I44, cleared on entry to `pull_request` from `verify` or `review`); SPEC-0003 R9, R30; the acceptance criteria of T7a in `plan.md`. Issue: new issue (D1), part of #161.
  - Files: `crates/wayloop-core/src/state/{transitions,guards,selectors,decide}.rs`, `crates/wayloop-core/src/state/pull_request.rs`, `crates/wayloop-core/tests/state_table.rs`, `changes/`.
  - Tests first: one test per AC of T7a; the cases of AC1 for its rows; AC35 as a property test over decided sequences.
  - Verification: TDD, property test.
  - Depends on: T6b.
  - Size: M. Maintainer (crates).

- [ ] **T7b Decide the merge gate, the merge and the holds**
  - Covers: SPEC-0002 R13 to R17 (merge and question gates), R30 to R34, R36, R39, R42 (holds), R43 (`done`), rows I18, I19, I24 to I26, I32 to I37, I45, G1 to G7 for merge and question gates (with the gate issue of S6 in the column Recorded), tables Holds (the answer of S9, kept in `no_unit_acknowledged` until the last hold is removed, N-S2; the reopen of a hold entered before `pull_request` resumes at its step, and a move out of `review` withdraws the question of I46 with `review_left`, final check) and Reconciliation codes; the acceptance criteria of T7b in `plan.md`. Issue: closes #161 (pull request events).
  - Files: `crates/wayloop-core/src/state/{transitions,guards,selectors,decide,holds}.rs`, `crates/wayloop-core/tests/state_table.rs`, `docs/en/reference/errors.md` and its Italian edition, `changes/`.
  - Tests first: one test per AC of T7b; the cases of AC1 for its rows; AC13 as a property test over decided sequences.
  - Verification: TDD, property test.
  - Depends on: T7a.
  - Size: M. Maintainer (crates).

- [ ] **T7c Decide sessions, checkouts and leases**
  - Covers: SPEC-0002 R22 (the task condition of B1), rows I30, I31, S1 to S6 (the checkout as a guard, S4; the end of a session without a checkout, B12; renewals never retries, B8); SPEC-0003 R10, R12, R43, R46 to R48 (without the quarantine), R52 (the exception of `lease_renewal_requested`); the acceptance criteria of T7c in `plan.md`. Issue: new issue (D1), part of #162; #162 stays open for the resume pack and the publication of hand-overs.
  - Files: `crates/wayloop-core/src/state/{transitions,guards,selectors,decide,sessions}.rs`, `crates/wayloop-core/tests/state_table.rs`, `docs/en/reference/errors.md` and its Italian edition, `changes/`.
  - Tests first: one test per AC of T7c; the cases of AC1 for its rows.
  - Verification: TDD.
  - Depends on: T7b.
  - Size: S. Maintainer (crates).

## Step 6: next steps and due events (#49)

- [ ] **T8 Compute the next steps, the ready work and the events due by time**
  - Covers: SPEC-0002 R5, R19, R22 (with the session without a checkout of S6, B12), R35, R38 (`awaiting_checks`), R39, R42 (with the value `no_active_sprint` of the enum `stop_reason` and its description, section Stop reasons; the behaviour of AC83 is outside the slice, decided by the maintainer on 2026-11-25), section Ready work and lanes, section Time; SPEC-0003 R11, R28, R43 (without the summary), R46; the acceptance criteria of T8 in `plan.md`. Issue: closes #49.
  - Files: `crates/wayloop-core/src/state/{next,due,lanes}.rs`, `changes/`.
  - Tests first: one test per AC of T8; AC3 and AC20 as property tests, AC20 with the runtime model of the specification; `next_steps` has no time parameter, and with `Integrity::Broken` returns only `ledger_broken`.
  - Verification: TDD, property test.
  - Depends on: T7c.
  - Size: M. Maintainer (crates).

## Step 7: the writer in the runtime (#50, part 3)

- [ ] **T9 Write the ledger through one locked writer behind a storage interface**
  - Covers: SPEC-0003 R1, R18 to R20, R24, R27, R28, R52, R62, R63, section One writer and concurrency, section Storage (segments, quarantine of a torn tail, genesis by rename); SPEC-0002 R41 (under the lock), R59; the acceptance criteria of T9 in `plan.md`; the lease part of #162. Issue: closes #50.
  - Files: `crates/wayloop-runtime/src/ledger/{mod,storage,lock,writer,loader}.rs`, `crates/wayloop-runtime/Cargo.toml` (lock crate, if one is needed, under `docs/en/project/dependencies.md`), `crates/wayloop-runtime/tests/`, `.github/workflows/ci.yml` (AC13 on Linux, macOS and Windows), `docs/en/concepts/architecture.md` and its Italian edition, `changes/`.
  - Tests first: one test per AC of T9; the test storage injects a failure at every operation and an interruption at every byte of an append (AC48, AC65); AC13 with the test binary and the append path it names.
  - Verification: TDD, property test, end-to-end (AC13).
  - Depends on: T8. Security review if a dependency is added.
  - Size: M. Maintainer (crates).

## Step 8: replay, projection and a golden ledger (#51, part 1)

- [ ] **T10 Replay the ledger, write the projection and check a hand-reviewed golden ledger**
  - Covers: SPEC-0003 R3, R25, R54 (the fold version constant), section Replay and snapshots (full replay only), section Storage (projection); SPEC-0002 R2, R7; the acceptance criteria of T10 in `plan.md`; the measure of the rewrite of the projection (counter-review item 75). Issue: new issue (D1), part of #51; #51 stays open for snapshots (SPEC-0003 R7, R31, R32).
  - Files: `crates/wayloop-runtime/src/ledger/projection.rs`, `crates/wayloop-core/src/state/fold.rs` (`FOLD_VERSION`), `tests/golden/ledger/` (golden ledger and `state.yaml`, each value reviewed by a person), `crates/wayloop-runtime/tests/`, `.github/workflows/ci.yml` (AC21 on three platforms), `changes/`.
  - Tests first: AC21 against the golden files; AC67 fails when the replayed state changes without a new `FOLD_VERSION`; AC69 key order and the two counts that replace the collections (SPEC-0003 Q16); AC1 over 10,000 decided events; a benchmark test that reports the time of the rewrite of `state.yaml` on 10,000 events in the pull request.
  - Verification: end-to-end, property test, TDD.
  - Depends on: T9.
  - Size: M. Maintainer (crates).

## Step 9: dry run on local fixtures (#58)

- [ ] **T11 Run the scenarios of the issue cycle end to end against the writer and replay them**
  - Covers: SPEC-0002 R58, AC43, section Examples; the gate of M1 ("drives the issue cycle in a dry run on local fixtures"); counter-review section 4, code pattern 9. Issue: closes #58.
  - Files: `specs/0002-state-machine/examples/trace.schema.json`, the four scenarios of `specs/0002-state-machine/examples/` rewritten in the trace format and to the amendments of 2026-11-25, `crates/wayloop-runtime/tests/scenarios.rs` (runner, fake executor, fake tracker, temporary git common directory), `changes/`.
  - Tests first: each scenario validates against the trace schema; a scenario with an unbound symbol or a missing field fails; the runner binds symbols at their first appearance; after the last step, a full replay of the produced ledger equals the state of the writer byte for byte.
  - Verification: end-to-end.
  - Depends on: T10.
  - Size: M. Maintainer (crates, specification examples).

## After the slice

- [ ] **T13 Add the member branch_type to the task artifact**
  - Covers: SPEC-0001 Task, member `branch_type` (as amended on 2026-11-25, SPEC-0002 Q16; named `branch_type`, second counter-review B10); SPEC-0002 section Names of a checkout; SPEC-0002 AC64, the clause of the `branch_type` `fix`. Issue: new issue (D1), under the epic of the artifact model (#18).
  - Files: `crates/wayloop-schemas/src/artifact/plan.rs` (`Task`), the generated snapshots of `artifacts/task`, `crates/wayloop-core/src/state/` (the type of the branch from the task), `specs/0001-artifact-model/examples/` (a task with `branch_type`), `tests/contract/validate/v1/`.
  - Tests first: a task with `branch_type` `fix` validates and gives a branch `fix/<issue>-<slug>` (AC64); a `branch_type` that is not a type of the policy is refused with `artifact_schema` and the reason `enum`; a task without `branch_type` still gives `feat`; a `plan_observed` item of type `task` with a `branch_type` keeps `type` as the artifact type.
  - Verification: contract test, TDD.
  - Depends on: T6a, #267.
  - Size: S. Maintainer (crates, contract change).

## Verification

- [ ] T12 Review by risk tier of every task with an agent other than the implementer; a separate
  `simplifier` pass on T3 to T8, which touch the pure crates; the `contract-auditor` role on T1,
  T2a and T2b; the mutation floor of D3 on T6a, T6b, T7a, T7b and T7c; every acceptance criterion of the slice ticked in
  `spec.md` with the test that proves it.
