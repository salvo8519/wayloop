# SPEC-0002: State machine of the project and of the issue cycle

| | |
| --- | --- |
| Status | Draft |
| Milestone | M1 · Engine core |
| Owner | @salvo8519 |
| Related | #11, #48, #49, #58, #228; ADR 0002, 0003, 0006, 0007, 0008, 0009, 0011, 0012, 0013, 0026, 0028; SPEC-0001, SPEC-0003, SPEC-0004, SPEC-0005, SPEC-0006, SPEC-0007, SPEC-0008, SPEC-0013, SPEC-0021 |
| Commit scope | state |

## Problem
The engine decides every step of the loop. Without an explicit state machine the decision moves into prompts, where it cannot be tested, replayed or audited. Maintainers need the loop to stop exactly at the gates they own; agents need the engine to tell them the next allowed step; parallel workers need a single, deterministic answer to the question "what is ready".

## Scope
**In scope:** The state of a project (its phase), of a gate and of the issue cycle of a planned task. Every transition with its trigger, guard, actor and recorded event. The approval states the loop cannot pass. Holds (the `blocked` state): how one is entered and left. Limits on turns, attempts, executor retries and token budgets, and the rule for repeated failure. The transition function and the next-step function, both pure. Ready work for parallel lanes (ADR 0006, ADR 0013). Stale revisions (ADR 0009 C6) and stale dispatches. The names of the events the machine consumes and emits. The engine-taken transitions of artifact statuses listed in SPEC-0001.

**Out of scope:** Event schemas, the hash chain, persistence and replay mechanics (SPEC-0003). Execution of steps, worktrees and the worker pool (SPEC-0006). Executors, briefs and result schemas of each step (SPEC-0008). The tracker mapping of labels and approvals (SPEC-0005, SPEC-0007). The content of gates and questions (SPEC-0007, SPEC-0012). Generation of plan items (SPEC-0013). Artifact fields and the allowed status transitions themselves (SPEC-0001). Policy rules and risk tiers as data (SPEC-0021).

## User stories
- As a maintainer, I want the loop to stop exactly at the gates I own, so that nothing irreversible happens without me.
- As a maintainer, I want every stop to carry its reason, so that I know in one read what the loop needs from me.
- As an agent, I want the engine to tell me the next allowed step, so that I never decide the process myself.
- As a runtime, I want the list of ready work in a fixed order, so that parallel workers never take the same issue or the same lane.

## Requirements (EARS)
- **R1**: THE ENGINE SHALL compute every transition with a function of the current state and one consumed event that returns either the next state and the events to record, or a refusal, and SHALL read no clock, file, network, environment or random source in it.
- **R2**: THE ENGINE SHALL compute the next steps (actions, ready work and stop reasons) with a function of the state, the time and the number of free workers given as inputs, and SHALL return them in the total order of section Ready work and lanes.
- **R3**: IF a command or a result fails one or more guards of its transition, THEN THE ENGINE SHALL refuse it, leave the state unchanged and report every unmet guard in one pass with its code from section Errors.
- **R4**: THE ENGINE SHALL accept each consumed event only in the input class given by the table Consumed events: a command with an expected revision, a result with a dispatch, or an observation with a source key.
- **R5**: THE ENGINE SHALL apply the limits, budgets and approvers recorded by the latest `limits_set` event; WHEN a run starts with an effective configuration whose limits, budgets or approvers differ from the recorded ones, THE ENGINE SHALL record `limits_set` before any other event of the run.
- **R6**: THE ENGINE SHALL record, with every emitted event, the consumed event that caused it, so that applying the recorded consumed events in order to the initial state reproduces every emitted event and the state.
- **R7**: THE ENGINE SHALL derive the phase of the project from the statuses of its artifacts with the function of section Project phase, and record `phase_changed` whenever a transition changes the derived phase.
- **R8**: WHILE the project is in phase `council` without a draft council report, or in phase `planning` without a pending plan proposal, THE ENGINE SHALL propose the action `run_council` or `generate_plan` respectively, within the budget of the phase.
- **R9**: WHEN a subject reaches one of the approval states of section Gates and approval states, THE ENGINE SHALL open a gate, record `gate_opened`, and propose no action that passes the gate while it is pending.
- **R10**: THE ENGINE SHALL resolve a gate only with an `approval_given` or `answer_given` observation whose actor is an authorised approver of the gate kind; IF the actor is an agent, a bot, the account of the engine or a person who is not an approver, THEN THE ENGINE SHALL record `input_ignored` with the code `actor_not_authorised` and keep the gate pending.
- **R11**: IF an approval of a gate arrives while a subject guard of the gate does not hold, THEN THE ENGINE SHALL record `input_ignored` with the code of the unmet guard and keep the gate pending.
- **R12**: WHEN the subject of a pending gate is superseded, THE ENGINE SHALL withdraw the gate and record `gate_withdrawn`.
- **R13**: THE ENGINE SHALL change the issue-cycle state of a planned task only through a transition of the table Issue transitions; IF a command or a result names a transition that the table does not allow from the current state, THEN THE ENGINE SHALL refuse it with the code `transition_not_allowed` and the states that allow it in `expected`.
- **R14**: THE ENGINE SHALL consider an issue ready only when every condition of the table Readiness holds, and SHALL report, for an issue that is not ready, every unmet condition with its code.
- **R15**: THE ENGINE SHALL propose at most as many claims as free workers, at most one checked-out issue per lane, and the first ready issues in the order of section Ready work and lanes.
- **R16**: WHEN a claim of a ready issue is accepted, THE ENGINE SHALL record `issue_claimed` with the holder, session, executor, branch and lease expiry, move an issue in state `open` to `claim`, keep the state of an issue that was claimed before, and record the artifact transitions of section Artifact transitions taken by the engine.
- **R17**: WHEN a step of a checked-out issue is requested, THE ENGINE SHALL record `step_started` as the dispatch of the step and count one turn if an executor runs the step; IF a result cites a dispatch that is not the open dispatch of the issue, THEN THE ENGINE SHALL refuse it with the code `dispatch_stale`.
- **R18**: IF a result does not validate against the schema of its step or fails a guard of its transition, or an executor fails or times out, THEN THE ENGINE SHALL record `result_rejected` or the failure, close the dispatch and, while executor retries remain, propose the same step again; WHEN the retries of the step are exhausted, THE ENGINE SHALL hold the issue with the reason `executor_failure`.
- **R19**: THE ENGINE SHALL complete the step `counter_review` only when every objection has a ruling or a question, the step `checklist` only when the checklist names every requirement cited by the task, and the step `review` only when every finding is verified or rejected with a reason.
- **R20**: WHERE the risk tier of an issue requires no counter-review or no review under the project policy, THE ENGINE SHALL skip that step and record the skip in `step_completed`.
- **R21**: WHEN a verification or the required checks of the pull request fail and the limits of section Limits and budgets are not reached, THE ENGINE SHALL record `attempt_failed` with the failure note and the failing checks, increase the attempt count, and move the issue to `build`.
- **R22**: WHEN the same check fails on the configured number of consecutive attempts, or the attempts of an issue reach their limit, THE ENGINE SHALL open a question of category `repeated_failure` with the options `retry` and `stop` and hold the issue instead of starting another attempt.
- **R23**: WHEN an executor step of an issue is requested and the turns of the issue equal its limit, THE ENGINE SHALL hold the issue with the reason `turn_limit` instead of starting the step, and release the hold only on a `limit_raised` observation that sets a limit above the turns.
- **R24**: WHEN the tokens recorded in a budget scope reach its budget, THE ENGINE SHALL record `budget_exhausted`, propose no new step in that scope and still accept the results of steps already started; WHEN the budget is raised above the recorded tokens, THE ENGINE SHALL record `budget_restored`.
- **R25**: WHEN an issue enters a hold, THE ENGINE SHALL set the issue-cycle state given by the transition of the table Issue transitions, which is the current state unless the row says otherwise, add the reason to its holds, record `issue_blocked` with the reason, and propose no action for the issue while a hold remains.
- **R26**: THE ENGINE SHALL remove a hold only with the observation that the table Holds lists for its reason, from an authorised approver; WHEN the last hold of an issue is removed, THE ENGINE SHALL record `issue_unblocked` and resume the issue at the state named by the table.
- **R27**: THE ENGINE SHALL never refuse an observation for its state: IF an observation is not consistent with the state of the issue it names, THEN THE ENGINE SHALL record it, apply the fact transition of the table Issue transitions when one exists, and otherwise hold the issue with the reason `reconciliation_required`.
- **R28**: WHEN an observation arrives whose source key is already recorded, THE ENGINE SHALL leave the state unchanged and record nothing.
- **R29**: WHEN a session ends or a lease expires, THE ENGINE SHALL clear the checkout of the issue, keep its issue-cycle state and holds, close its open dispatch, and record `issue_released` with the cause `session_ended` or `lease_expired`.
- **R30**: THE ENGINE SHALL move an issue to `done` only on the observation `pull_request_merged` of its closing pull request; WHEN the merge was not preceded by an approval of the merge gate by an authorised approver, THE ENGINE SHALL also record `gate_bypassed` and `reconciliation_required`.
- **R31**: WHEN the pull request of an issue is closed without merge, THE ENGINE SHALL hold the issue with the reason `pull_request_closed`.
- **R32**: WHEN the task of an issue is superseded, THE ENGINE SHALL move the issue to `superseded`, clear its checkout and holds, withdraw its gates, and record `issue_superseded`.
- **R33**: IF a command carries an expected revision that is stale under the rule of section Revisions, THEN THE ENGINE SHALL refuse it with the code `revision_stale`, the current revision in `expected`, and record nothing.
- **R34**: THE ENGINE SHALL report, for the project and for every issue that cannot progress without a person or a new budget, its stop reasons from the enum `stop_reason`, with the gate, question or scope each one names.
- **R35**: THE ENGINE SHALL take the engine transitions of artifact statuses of SPEC-0001 only on the triggers listed in section Artifact transitions taken by the engine, and record each as `artifact_status_changed`.

## Acceptance criteria
- **AC1** (R1, R13, R27): Given every pair of issue-cycle state and consumed event in the table Issue transitions, generated with random guards, when applied, then the next state, the holds and the recorded events match the table, and every pair absent from the table is refused with `transition_not_allowed` or, for an observation, held with `reconciliation_required`. Verification: property test.
- **AC2** (R1): Given a random state and a random event, when the transition is applied twice, then both outputs are byte-identical in canonical JSON, and the core crate has no dependency that reads a clock, the environment or a random source. Verification: property test.
- **AC3** (R2, R15): Given a random set of issues, lanes and free workers, when the next steps are computed for the same state with the issues stored in different orders, then the output is identical and ordered as section Ready work and lanes says. Verification: property test.
- **AC4** (R3, R14): Given an issue with one dependency not done, a hold and a checkout held by another session, when a claim is requested, then it is refused with `dependency_not_done`, `issue_on_hold` and `checkout_held` in one pass, each with its pointer, and the ledger is unchanged. Verification: TDD.
- **AC5** (R4): Given a `claim_requested` without an expected revision, a `step_result_submitted` without a dispatch and a `pull_request_merged` without a source key, when submitted, then each is refused with the reason `missing` at the missing field. Verification: TDD.
- **AC6** (R5): Given a ledger with `limits_set` at 20 turns and a configuration changed to 30 turns after it, when the ledger is replayed, then the limit applied is 20; given a run started with the new configuration, then `limits_set` with 30 turns is the first event of the run. Verification: TDD.
- **AC7** (R6): Given a random ledger, when its consumed events are applied again from the initial state, then the emitted events and the state equal the recorded ones. Verification: property test.
- **AC8** (R7): Given the fixture states of the table Project phase, when the phase is derived, then each gives its phase; given a project with an approved milestone and no brief, then the phase is `delivery`. Verification: TDD.
- **AC9** (R8, R24): Given a project in phase `council` with an approved brief and no council report, when the next steps are computed, then `run_council` is proposed; given the budget of the phase exhausted, then it is not proposed and the stop reason is `budget_exhausted` with the scope. Verification: TDD.
- **AC10** (R9, R10): Given any sequence of random events in which no authorised approver acts, when applied, then no gate is resolved as `approved`, no issue reaches `done` except through a merge observation with `gate_bypassed`, and no artifact passes a person transition of SPEC-0001. Verification: property test.
- **AC11** (R10): Given an issue at `merge` and an `approval_given` from an agent account, when processed, then `input_ignored` with `actor_not_authorised` is recorded and the gate stays pending. Verification: TDD.
- **AC12** (R11): Given a draft council report with one question without an answer, when an authorised approval arrives, then `input_ignored` with `question_unanswered` is recorded and the gate stays pending. Verification: TDD.
- **AC13** (R12, R32): Given an issue at `build` with an open question, when its task is superseded, then the issue is `superseded`, its checkout and holds are cleared and the question gate is withdrawn. Verification: TDD.
- **AC14** (R14): Given one ready issue, when each readiness condition is made false in turn, then the issue is not ready and the report names exactly that condition's code. Verification: TDD.
- **AC15** (R15): Given an active sprint with two lanes of three ready issues each and three free workers, when the next steps are computed, then two claims are proposed, the first ready issue of each lane; given one lane with a checked-out issue, then no claim is proposed for that lane. Verification: TDD.
- **AC16** (R15): Given random sprints, workers and event sequences, when the runtime follows the proposed actions, then no lane ever has two checked-out issues and the number of checkouts never exceeds the workers. Verification: property test.
- **AC17** (R16, R35): Given the first claim of a task whose story, specification, epic and milestone are `approved`, when accepted, then the issue is at `claim` and the task, story, specification, epic and milestone move to `implementing`, each with `artifact_status_changed`. Verification: TDD.
- **AC18** (R17): Given an issue at `build` with dispatch D2 after a retry of dispatch D1, when a result citing D1 arrives, then it is refused with `dispatch_stale` and nothing is recorded; given a result citing D2, then it is accepted. Verification: TDD.
- **AC19** (R18): Given executor retries set to 2, when the executor of a step times out three times, then two `step_retried` events and one `issue_blocked` with `executor_failure` are recorded; given a result that does not validate, then `result_rejected` is recorded and counts as one failure. Verification: TDD.
- **AC20** (R19): Given a checklist result that omits requirement R2 cited by the task, a counter-review result with an objection without a ruling and a review result with a finding neither verified nor rejected, when each is submitted, then each is recorded as `result_rejected` with `checklist_incomplete`, `objection_unruled` or `finding_unverified`. Verification: TDD.
- **AC21** (R20): Given a policy in which the tier `small` requires no counter-review and no review, when an issue of tier `small` completes `reanalysis` and `verify`, then it moves to `checklist` and `pull_request`, and each `step_completed` lists the skipped step. Verification: TDD.
- **AC22** (R21): Given an issue at `verify` with attempts below the limit, when the verification fails on check `unit`, then `attempt_failed` names `unit` and carries a failure note, the attempt count is 2 and the issue is at `build`. Verification: TDD.
- **AC23** (R22): Given the same-check limit 2, when check `unit` fails on two consecutive attempts, then a question of category `repeated_failure` with the options `retry` and `stop` is opened and the issue is held; when an approver answers `retry`, then the failure counts are reset and the issue resumes at `build`. Verification: TDD.
- **AC24** (R23): Given a limit of N turns, when N executor steps have started and another is requested, then the issue is held with `turn_limit` and no `step_started` is recorded; given `limit_raised` to N + 5, then the hold is removed and the step can be requested. Verification: TDD.
- **AC25** (R24): Given a sprint budget of N tokens, when a result brings the recorded tokens of the sprint to N, then `budget_exhausted` is recorded, no step of the sprint is proposed, and a result of a step already started is still accepted; given `budget_raised`, then `budget_restored` is recorded and steps are proposed again. Verification: TDD.
- **AC26** (R25, R26): Given an issue at `counter_review`, when its result raises a question of category `security_sensitive_action`, then the issue keeps `counter_review`, is held with `question` and no action is proposed for it; given an answer from a person who is not an approver, then `input_ignored` is recorded; given an answer from an approver, then `issue_unblocked` is recorded and the issue resumes at `counter_review`. Verification: TDD.
- **AC27** (R27, R30): Given an issue at `pull_request` before its checks complete, when `pull_request_merged` is observed, then the issue is `done` and `gate_bypassed` and `reconciliation_required` are recorded; given an issue at `build`, when `issue_closed` without a merge is observed, then it is held with `reconciliation_required`. Verification: TDD.
- **AC28** (R28): Given any observation delivered k times with the same source key, k from 1 to 5, when applied, then the state and the ledger equal those after one delivery. Verification: property test.
- **AC29** (R29, R35): Given an issue at `build` whose lease expires, when `lease_expired` is observed, then its checkout is cleared, its state is still `build`, `issue_released` with `lease_expired` is recorded and its task moves to `approved`; given a session that ends with a hand-over, then the cause is `session_ended` and the task stays `implementing`. Verification: TDD.
- **AC30** (R30): Given an issue at `merge` with an approved merge gate, when its pull request is merged, then the issue is `done`, its task is `done` and no `gate_bypassed` is recorded; given a review by an approver that requests changes at `merge`, then the gate is resolved as `rejected` and the issue moves to `build`. Verification: TDD.
- **AC31** (R31): Given an issue at `merge`, when its pull request is closed without merge, then it is held with `pull_request_closed`; given `unblock_requested` by an approver, then it resumes at `build`. Verification: TDD.
- **AC32** (R33): Given a revision R1 and a later event on the same issue, when a command with the expected revision R1 is sent, then it is refused with `revision_stale` and `expected` holds the current revision, and the ledger is unchanged. Verification: TDD.
- **AC33** (R34): Given a project with a pending council report gate, an issue held with `turn_limit` and a sprint budget exhausted, when the status is computed, then the stop reasons are `approval_required`, `turn_limit` and `budget_exhausted`, each naming its gate, issue or scope, and every value matches `^[a-z][a-z0-9_]*$`. Verification: contract test.
- **AC34** (R35): Given the last task of a story merged while another task of the story is superseded, when processed, then the story moves to `done`; given an epic whose only task is superseded, then the epic does not move. Verification: TDD.
- **AC35** (R1, R2, R13, R16, R21, R30): Given the scenarios of `examples/`, when each is applied event by event to its initial state, then every expected state, hold and recorded event matches. Verification: end-to-end.

## Model

### Machines and state

The engine runs three machines over one state, which is a projection of the ledger (SPEC-0003):

| Machine | Instances | State | Section |
| --- | --- | --- | --- |
| Project phase | One per project | `phase`, derived from artifact statuses | Project phase |
| Gate | One per approval or question | `gate_state` and its resolution | Gates and approval states |
| Issue cycle | One per planned task issue | `issue_state`, checkout, holds, dispatch, counters | Issue cycle |

The issue-cycle state is not the status of the task artifact (SPEC-0001 R5). A task in status
`implementing` may be at any non-terminal issue-cycle state, checked out or not, held or not.
`blocked` is not a value of any enum: an issue is blocked while its list of holds is not empty.

The state of one issue:

| Field | Meaning |
| --- | --- |
| `issue_state` | One of the values of the table Issue states |
| `checkout` | Absent, or `{holder, session, executor, branch, lease_expires_at}` (SPEC-0003 R10) |
| `holds` | List of `{reason, question}`; `question` names the gate of a hold with the reason `question` |
| `dispatch` | Absent, or the identifier of the open `step_started` event |
| `turns` | Executor steps started for the issue |
| `attempts` | Attempts started, from 1 |
| `failures` | For each check name, the number of consecutive attempts on which it failed |
| `retries` | Executor failures of the open step |
| `tier` | Risk tier, from the policy (section Risk tier) |
| `pull_request` | Absent, or the number and head commit of the closing pull request |

Time enters only as data. Every recorded event carries `recorded_at`, set by the single writer
(SPEC-0006 R5); the next-step function takes `now` as an input. A lease expires when
`lease_expires_at` is not later than `now`; the lease is extended to `recorded_at` plus the
lease duration by every event of the holder.

### Inputs

Every consumed event belongs to one class (R4). The class decides how staleness, refusal and
recording work.

| Class | Sent by | Carries | When it does not fit the state |
| --- | --- | --- | --- |
| Command | Runtime or agent, through a command (SPEC-0004) | `expected_revision` | Refused (R3, R13, R33); nothing recorded |
| Result | Runtime, for an open dispatch: executor output or check results | `dispatch` | Stale dispatch: refused, nothing recorded (R17). Invalid or failing a guard: recorded as `result_rejected` (R18), because model output that does not validate becomes an error event (ADR 0003) |
| Observation | Adapters: tracker, git, clock; persons act through the tracker (ADR 0012) | `source_key`, `actor` when a person acts | Never refused for its state (R27); duplicates ignored (R28); unauthorised actors recorded as `input_ignored` (R10) |

The ledger records what happened, not what was asked: an accepted input and the events it
causes are recorded; a refused command or a stale result is not.

### The two functions

```text
transition(state, event)          -> Ok { state, emitted: [event] } | Refused { errors }
next_steps(state, now, free_workers) -> { actions, ready, stops }
```

- `transition` is the only way the state changes (R1). It is total: every pair of state and
  event gives a next state or a refusal, never a panic. The same state and the same event give
  the same output, byte for byte in canonical JSON (constitution II).
- `next_steps` changes nothing (R2). `actions` lists what the runtime may start now, each of
  kind `claim`, `run_step`, `run_council`, `generate_plan` or `expire_lease`; `ready` lists
  ready issues that get no action for lack of workers; `stops` lists the stop reasons (R34).
  The runtime turns an action into a command or an observation; it never chooses among actions
  (ADR 0006: the core decides, the runtime executes).
- Limits, budgets and approvers come from the state, set by `limits_set` (R5), so that a replay
  never depends on the configuration of the machine that replays.

### Project phase

The phase is a function of artifact statuses, evaluated in this order; the first row that
holds gives the phase (R7). Deriving it, instead of storing it, keeps it consistent with the
artifacts by construction, and lets a project adopt Wayloop with an existing roadmap: it starts
in `delivery`.

| Phase | Holds when |
| --- | --- |
| `delivery` | At least one milestone is `approved` or `implementing` |
| `complete` | At least one milestone is `done` and every milestone is `done`, `superseded` or `draft` |
| `planning` | A council report is `approved` |
| `council` | A brief is `approved` |
| `intake` | Otherwise |

Transitions of the phase (each records `phase_changed` when the phase changes):

| # | From | Trigger | Guard | Actor | To | Recorded |
| --- | --- | --- | --- | --- | --- | --- |
| P1 | `intake` | `brief_submitted` | Brief valid (SPEC-0001), in `draft` | Person | `intake` | `gate_opened` (kind `brief`) |
| P2 | `intake` | `approval_given` on the brief gate | Gate guards (G2) | Person | `council` | `gate_resolved`, `artifact_status_changed` (brief `approved`, gate issue `done`), `phase_changed` |
| P3 | `council` | `council_report_submitted` | Report valid, its brief `approved` | Agent (result of `run_council`) | `council` | `gate_opened` (kind `council_report`) |
| P4 | `council` | `approval_given` on the council report gate | G2; every question answered (SPEC-0001) | Person | `planning` | `gate_resolved`, `artifact_status_changed`, `phase_changed` |
| P5 | `planning` | `plan_proposal_submitted` | Every item valid (SPEC-0001), at least one milestone, from the approved report | Agent (result of `generate_plan`) | `planning` | `gate_opened` (kind `roadmap`) |
| P6 | `planning`, `complete` | `approval_given` on a `roadmap` or `roadmap_change` gate that approves a milestone | G2 | Person | `delivery` | `gate_resolved`, `artifact_status_changed` for each item, `phase_changed` |
| P7 | `delivery` | `status_change_requested`: milestone to `done` (gate met) | SPEC-0001 transition allowed; actor authorised | Person | `complete` when the derived phase says so, else `delivery` | `artifact_status_changed`, `phase_changed` |
| P8 | `council`, `planning` | `status_change_requested`: the approved brief or council report to `superseded` | SPEC-0001 transition allowed; actor authorised | Person | Derived: `intake` or `council` | `artifact_status_changed`, `gate_withdrawn` for its gates, `phase_changed` |

In `delivery`, reconciliation (SPEC-0007 R3) submits `roadmap_change_proposed`, which opens a
gate of kind `roadmap_change`; the change is applied only when the gate is approved (SPEC-0007
R4). The survey (SPEC-0010, SPEC-0011) may run in any phase and changes no phase by itself.

### Gates and approval states

A gate is a pending decision of a person (constitution IV, ADR 0012). The loop cannot pass it:
no transition resolves a gate without an observation from an authorised approver (R10), and
`next_steps` proposes no action that depends on a pending gate (R9).

Approval states:

| Gate kind | Opened when | Subject guards checked on approval | Effect of `approved` | Effect of `rejected` |
| --- | --- | --- | --- | --- |
| `brief` | A brief is submitted (P1) | Brief valid | Brief `approved` (P2) | Brief stays `draft`; a new submission opens a new gate |
| `council_report` | A report is submitted (P3) | Every question answered (`question_unanswered`) | Report `approved` (P4) | Report stays `draft` |
| `roadmap` | A plan proposal is submitted (P5) | Every item valid | Items `approved` (P6) | Items stay `draft` |
| `roadmap_change` | A reconciliation proposal is submitted | Change applies to the current plan | Change applied (SPEC-0007 R4) | Nothing changes |
| `specification` | A specification in `draft` or `surveyed` is submitted for approval | Specification valid | Specification `approved` | Stays as it was |
| `merge` | An issue reaches `merge` (I16) | Required checks green at the head commit of the pull request | The merge may happen; `done` follows the merge (I17) | Issue to `build` (I18) |
| `question` | An agent or the engine raises a question | Answer among the options (`option_unknown`) | Not applicable: resolution `answered` | Not applicable |

Gate transitions:

| # | From | Trigger | Guard | Actor | To | Recorded |
| --- | --- | --- | --- | --- | --- | --- |
| G1 | (none) | Subject reaches an approval state | None | Engine | `pending` | `gate_opened` |
| G2 | `pending` | `approval_given` with decision `approve` | Actor authorised (R10); subject guards (R11) | Person | `resolved`, resolution `approved` | `gate_resolved` |
| G3 | `pending` | `approval_given` with decision `reject` | Actor authorised | Person | `resolved`, resolution `rejected` | `gate_resolved` |
| G4 | `pending` | `answer_given` | Actor authorised; option among the options of the question | Person | `resolved`, resolution `answered` | `question_answered`, `gate_resolved` |
| G5 | `pending` | Subject superseded, or issue `done` through a bypass | None | Engine, on the trigger | `withdrawn` | `gate_withdrawn` |
| G6 | `pending` | `approval_given` or `answer_given` failing G2 to G4 | None | Any | `pending` | `input_ignored` with the code of the failure |

`gate_state` is `pending`, `resolved` or `withdrawn`; `resolution` is `approved`, `rejected` or
`answered`. A gate issue of SPEC-0001 is `done` when its gate is `approved` and `superseded` when
its gate is withdrawn. How a person expresses an approval on the tracker (comment, reaction or
label) is decided by SPEC-0007; this machine consumes the normalised observation.

An authorised approver of a gate kind is a person listed by `limits_set` for that kind (R5,
Q9). Agents, bots and the account of the engine are never approvers (SPEC-0007 R6).

### Issue cycle

Issue states, in order. `Actor` is who does the work of the step; `Turn` says whether starting
it counts a turn (R17).

| `issue_state` | Meaning | Actor | Turn |
| --- | --- | --- | --- |
| `open` | Planned, never claimed | None | No |
| `claim` | First checkout in progress: branch and worktree being prepared | Engine | No |
| `reanalysis` | The issue, story, specification and code are read again | Agent | Yes |
| `counter_review` | A second agent looks for gaps; every objection ends in a ruling or a question | Agent | Yes |
| `checklist` | The checklist is generated from the specification and published | Agent, validated by the engine | Yes |
| `build` | Tests where they pay, then the code | Agent | Yes |
| `verify` | Deterministic checks decide | Engine | No |
| `review` | Review proportional to risk, by an agent other than the implementer (SPEC-0008 R14) | Agent | Yes |
| `pull_request` | The pull request is opened, then its required checks run | Engine | No |
| `merge` | Approval state: the merge gate is pending | Person | No |
| `done` | Closing pull request merged; terminal | None | No |
| `superseded` | Task superseded; terminal | None | No |

The values from `claim` to `merge` are the `step` values of the hand-over (SPEC-0001) and the
identifiers of `IssueStep` in `crates/wayloop-core`, in `snake_case` after #228.

Issue transitions. `X` is any state from `claim` to `pull_request`; `N` is any non-terminal
state. A row without a checkout guard applies whether or not the issue is checked out.

| # | From | Trigger (class) | Guard | Actor | To | Recorded |
| --- | --- | --- | --- | --- | --- | --- |
| I1 | `open` | `claim_requested` (command) | Ready (R14) | Engine | `claim`, checked out | `issue_claimed`, artifact transitions (R35) |
| I2 | `N` except `open`, not checked out | `claim_requested` (command) | Ready (R14) | Engine | Same state, checked out | `issue_claimed`, task to `implementing` if `approved` |
| I3 | `X`, checked out, no dispatch, no hold | `step_requested` (command) | Requester is the holder (`not_holder`); turns below the limit for an executor step; budget not exhausted; for `pull_request`, the pull request not yet opened | Engine | Same state, dispatch open | `step_started` |
| I4 | `X`, executor step, as I3 | `step_requested` (command) | Turns equal the limit | Engine | Same state, hold `turn_limit` | `issue_blocked` |
| I5 | `claim` | `step_result_submitted` (result) | Dispatch current; branch and worktree reported | Engine | `reanalysis` | `step_completed` |
| I6 | `reanalysis` | `step_result_submitted`, outcome `completed` | Dispatch current; result valid | Agent | `counter_review`, or `checklist` when the tier skips it (R20) | `step_completed` |
| I7 | `counter_review` | `step_result_submitted`, outcome `completed` | As I6; every objection ruled (R19) | Agent | `checklist` | `step_completed` |
| I8 | `checklist` | `step_result_submitted`, outcome `completed` | As I6; checklist names every cited requirement (R19) | Agent | `build` | `step_completed` |
| I9 | `build` | `step_result_submitted`, outcome `completed` | As I6 | Agent | `verify` | `step_completed` |
| I10 | `verify` | `verification_completed` (result), every check passed | Dispatch current | Engine | `review`, or `pull_request` when the tier skips it (R20) | `step_completed`; passing checks reset in `failures` |
| I11 | `verify` | `verification_completed`, a check failed | Limits of R22 not reached | Engine | `build`, attempts plus one | `attempt_failed` |
| I12 | `verify`, `pull_request` | `verification_completed` or `checks_completed`, a check failed | Limits of R22 reached | Engine | `build`, hold `question` | `attempt_failed`, `gate_opened` (question `repeated_failure`), `issue_blocked` |
| I13 | `review` | `step_result_submitted`, outcome `completed` | As I6; every finding verified or rejected (R19) | Agent | `pull_request` | `step_completed` |
| I14 | `review` | `step_result_submitted`, outcome `changes_requested` | As I13; at least one verified finding | Agent | `build` | `step_completed` |
| I15 | `pull_request` | `pull_request_opened` (observation) | Branch of the issue; body closes the issue (SPEC-0001 R10) | Engine | `pull_request`, pull request recorded | `step_completed` for the opening; dispatch closed |
| I16 | `pull_request` | `checks_completed` (observation), every required check passed | Head commit equals the recorded head | Engine | `merge` | `gate_opened` (kind `merge`) |
| I17 | `merge` | `pull_request_merged` (observation) | Merge gate `approved` | Person | `done` | `gate_resolved` if not yet, `issue_done`, `issue_released`, task to `done` (R35) |
| I18 | `merge` | `review_submitted` (observation), changes requested | Reviewer authorised | Person | `build` | `gate_resolved` (`rejected`) |
| I19 | `pull_request` | `checks_completed`, a required check failed | Limits of R22 not reached | Engine | `build`, attempts plus one, same branch | `attempt_failed` |
| I20 | `N` | `pull_request_merged` of the closing pull request | Merge gate not `approved` | Any (fact) | `done` | `issue_done`, `gate_bypassed`, `gate_withdrawn` for pending gates, `reconciliation_required`, task to `done` |
| I21 | `pull_request`, `merge` | `pull_request_closed` (observation) | None | Any (fact) | Same state, hold `pull_request_closed` | `issue_blocked`, `gate_withdrawn` for the merge gate |
| I22 | An executor step (`reanalysis`, `counter_review`, `checklist`, `build`, `review`) with a dispatch | `step_result_submitted`, outcome `question_raised` | Dispatch current; question valid: context, options with cost, recommendation (SPEC-0007 R1) | Agent | Same state, hold `question` | `step_completed` (`question_raised`), `gate_opened` (kind `question`), `issue_blocked` |
| I23 | Any state with a dispatch | `step_result_submitted` invalid or failing a guard, or `executor_failed` (result) | Retries of the step not exhausted | Engine | Same state, dispatch closed, retries plus one | `result_rejected` or `step_retried` |
| I24 | As I23 | As I23 | Retries exhausted | Engine | Same state, hold `executor_failure` | `result_rejected` or `step_retried`, `issue_blocked` |
| I25 | `N`, checked out | `session_ended` (command) | Requester is the holder; hand-over valid (SPEC-0001 R22) | Agent | Same state, not checked out | `issue_released` (`session_ended`) |
| I26 | `N`, checked out | `lease_expired` (observation) | `lease_expires_at` not later than the observed time | Engine | Same state, not checked out | `issue_released` (`lease_expired`), task to `approved` |
| I27 | `N`, held | Observation of the table Holds for a reason of the issue | Actor authorised; the observation fits the reason | Person | Hold removed; state of the table Holds when it was the last hold | `issue_unblocked` when no hold remains, else nothing more |
| I28 | `N` | `stop_requested` (observation) | Actor authorised | Person | Same state, hold `stopped_by_person` | `issue_blocked` |
| I29 | `N` | `status_change_requested`: task to `superseded` (observation), or task expanded (SPEC-0013 R3) | SPEC-0001 transition allowed | Person or engine | `superseded` | `issue_superseded`, `issue_released`, `gate_withdrawn` |
| I30 | `N` | `issue_closed` (observation) not caused by the closing pull request | None | Any (fact) | Same state, hold `reconciliation_required` | `issue_blocked`, `reconciliation_required` |
| I31 | `done` | `issue_reopened` (observation) | None | Any (fact) | `done` (terminal, SPEC-0001) | `reconciliation_required` |
| I32 | Any | Any observation with a recorded source key | None | None | Unchanged | Nothing (R28) |

Notes on the table:

- Engine steps (`claim`, `verify`, `pull_request`) are dispatched like executor steps (I3) so
  that every result names its dispatch; they count no turn.
- After a release (I25, I26) the issue keeps its state, so that a new claim (I2) resumes where
  the work stopped and the brief starts from the latest hand-over (SPEC-0003 R13). Q4 asks
  whether `reanalysis` should be repeated instead.
- An issue at `merge`, or at `pull_request` after its pull request is opened, needs a person or
  the tracker, not an agent: it is not ready (R14) and may stay released until the merge.
- I11 and I19 differ on the branch. A failed verification before the pull request ends the
  attempt; the runtime discards the branch and worktree (SPEC-0006 R8). A failed required check
  after the pull request is opened keeps the branch, so that the pull request stays open.

### Holds

A hold is the `blocked` state of the conventions page: the issue keeps its state and cannot
progress until every hold is removed (R25, R26). Holds survive a release: a held issue is not
ready, whoever holds its checkout.

| Reason | Entered by | Removed by (observation, authorised approver) | Resumes at |
| --- | --- | --- | --- |
| `question` | I12, I22 | `answer_given` on its question gate (G4) | The same state; for `repeated_failure`: answer `retry` resets `attempts` and `failures` and resumes at `build`, answer `stop` replaces the hold with `stopped_by_person` |
| `turn_limit` | I4 | `limit_raised` with a limit above `turns` | The same state |
| `executor_failure` | I24 | `unblock_requested` | The same state, retries reset |
| `pull_request_closed` | I21 | `unblock_requested` | `build` |
| `reconciliation_required` | I30, and R27 for an inconsistent observation | `unblock_requested` | The same state |
| `stopped_by_person` | I28, or answer `stop` | `unblock_requested` | The same state |

Question categories (`question_category`): `decision`, `irreversible_action`,
`security_sensitive_action`, `outside_worktree`, `broken_plan`, `methodology_choice`,
`repeated_failure`, `invalid_tracker_block`. Agents raise the first six (the stops of the
concept page Gates); the engine raises `repeated_failure` (R22) and, on a structured block that
does not validate, `invalid_tracker_block` (SPEC-0005 R3). The engine-raised question
`repeated_failure` has the fixed options `retry` and `stop`, because the engine must know the
effect of each answer; the answers to other questions carry no effect on the state and reach
the next brief.

The tracker adapter spells the state as labels (SPEC-0005): `in-progress` while a checkout
exists, `blocked` while a hold exists, `needs-decision` while a question is pending. The core
never reads a label.

### Ready work and lanes

An issue is ready when every condition holds (R14). Each condition has the code reported when
it fails.

| Condition | Code |
| --- | --- |
| The issue is a planned task issue in state `open`, `claim` or `reanalysis` to `pull_request` before its pull request is opened | `awaiting_external` for `merge` and an opened pull request; `transition_not_allowed` for `done` and `superseded` |
| The task is `approved` or `implementing` | `task_not_approved` |
| Its epic and milestone are `approved` or `implementing` | `plan_not_approved` |
| Every cited specification is `approved`, `implementing` or `done` | `specification_not_approved` |
| Every task in `depends_on` is `done` | `dependency_not_done` |
| No hold | `issue_on_hold` |
| No checkout | `checkout_held` |
| No other issue of its lane is checked out | `lane_busy` |
| The issue belongs to a lane (section below) | `not_in_lane` |
| The budget of every scope of the issue is not exhausted | `budget_exhausted` |

Lanes. While a sprint is active, the lanes are those of the sprint file, whose paths are
disjoint (SPEC-0006 R4, ADR 0013), and an issue outside every lane is not ready. Without an
active sprint, the lane rule is the subject of Q5. A lane runs one issue at a time: lanes are
the unit of parallelism, and two issues of one lane may change the same files.

Order. `next_steps` lists actions in this order, and stops when `free_workers` actions that
need a worker are listed:

1. `expire_lease` for every expired lease, by issue number (needs no worker).
2. `run_step` for checked-out issues with no dispatch and no hold, by issue number; their
   workers are already counted as busy by the runtime, so they need no free worker.
3. `run_council` or `generate_plan` when R8 applies.
4. `claim` for the first ready issue of each free lane: lanes in the order of the sprint file,
   issues in the order of the lane; then by issue number.

Issues that are ready but get no `claim` are listed in `ready` in the same order.

### Limits and budgets

| Limit | Key (proposed, ADR 0010) | Counts | On reaching it |
| --- | --- | --- | --- |
| Turns per issue | `limits.turns_per_issue` | Executor steps started (I3) | Hold `turn_limit` (I4, R23) |
| Attempts per issue | `limits.attempts` | Attempts started; a failed verification or required check ends one | Question `repeated_failure` (I12, R22) |
| Same failure | `limits.same_failure` | Consecutive attempts on which one check fails | Question `repeated_failure` (I12, R22) |
| Executor retries per step | `limits.executor_retries` | Executor failures and rejected results of the open step | Hold `executor_failure` (I24, R18) |
| Lease | `limits.lease_minutes` | Minutes since the latest event of the holder | Release (I26) |
| Budget per scope | `budgets.<scope>` | Tokens recorded by results and failures in the scope (SPEC-0008 R4) | `budget_exhausted` (R24) |

Budget scopes are the project phase and the active sprint, subject to Q8. Usage and rate limits
of a provider are not failures: the runtime waits or asks (SPEC-0008 R16) and the dispatch stays
open. Defaults are the subject of Q6.

### Risk tier

The tier of an issue is the highest of the tier of its area and the tiers of the paths changed
on its branch, as the project policy maps them (SPEC-0021). Tiers are `small`, `substantial` and
`contract`, as in the table of AGENTS.md. The tier decides which steps are skipped (R20, Q1) and
whether the merge gate admits a standing approval (Q2). A tier that rises after `counter_review`
does not repeat it; it raises the review required at `review`.

### Revisions

A command carries `expected_revision`, the hash of the latest ledger event the sender read
(ADR 0009 C6, SPEC-0004 R12). The rule that makes it stale is Q3. Under the recommended rule, a
revision is stale when an event recorded after it names the same issue, or, for a command on
the project, changes the phase, a gate or the plan. Under the strict rule, any later event makes
it stale. A stale command is refused with the current revision in `expected` and nothing is
recorded (R33); the sender reads the state again and decides again.

Results use their dispatch instead of a revision (R17): a worker in one lane must not be
refused because another lane recorded an event. Observations carry no revision: they report
facts that happened, which the writer applies in order.

### Events

Names only; schemas, envelopes and versions belong to SPEC-0003. Every name is `snake_case`.

Consumed events:

| Event | Class | Source | Used by |
| --- | --- | --- | --- |
| `run_started` | Command | Runtime | R5 |
| `claim_requested` | Command | Runtime or agent | I1, I2 |
| `step_requested` | Command | Runtime or agent | I3, I4 |
| `session_ended` | Command | Agent, holder | I25 |
| `step_result_submitted` | Result | Runtime: validated executor output, outcome `completed`, `changes_requested` or `question_raised` | I5 to I9, I13, I14, I22 to I24 |
| `verification_completed` | Result | Runtime: check names and outcomes | I10 to I12 |
| `executor_failed` | Result | Runtime: `error` or `timeout` | I23, I24 |
| `council_report_submitted` | Result | Runtime, for `run_council` | P3 |
| `plan_proposal_submitted` | Result | Runtime, for `generate_plan` | P5 |
| `roadmap_change_proposed` | Result | Runtime, for reconciliation (SPEC-0007) | Gate `roadmap_change` |
| `brief_submitted` | Observation | Person | P1 |
| `approval_given` | Observation | Person, decision `approve` or `reject` | G2, G3, P2, P4, P6 |
| `answer_given` | Observation | Person | G4, I27 |
| `unblock_requested` | Observation | Person | I27 |
| `stop_requested` | Observation | Person | I28 |
| `limit_raised` | Observation | Person | I27, R23 |
| `budget_raised` | Observation | Person | R24 |
| `status_change_requested` | Observation | Person: a person transition of SPEC-0001 | P7, P8, I29 |
| `sprint_started`, `sprint_closed` | Observation | Person, through the sprint file | Lanes, budget scope |
| `pull_request_opened`, `checks_completed`, `review_submitted`, `pull_request_merged`, `pull_request_closed` | Observation | Tracker (SPEC-0003 R9) | I15 to I21 |
| `issue_closed`, `issue_reopened` | Observation | Tracker | I30, I31 |
| `lease_expired` | Observation | Runtime, on `expire_lease`, with the observed time | I26 |

Emitted events: `limits_set`, `phase_changed`, `gate_opened`, `gate_resolved`,
`gate_withdrawn`, `gate_bypassed`, `question_answered`, `issue_claimed`, `step_started`,
`step_completed`, `step_retried`, `result_rejected`, `attempt_failed`, `issue_blocked`,
`issue_unblocked`, `issue_released`, `issue_done`, `issue_superseded`,
`artifact_status_changed`, `budget_exhausted`, `budget_restored`, `reconciliation_required`,
`input_ignored`. Each carries the identifier of the consumed event that caused it (R6).

### Artifact transitions taken by the engine

The engine transitions of SPEC-0001 and their triggers (R35). Person transitions arrive as
`approval_given` or `status_change_requested` and pass G2 or the authorisation guard.

| Artifact | Transition | Trigger |
| --- | --- | --- |
| Task | `approved` to `implementing` | I1, I2 |
| Task | `implementing` to `approved` | I26 (lease expired) |
| Task | `implementing` to `done` | I17, I20 |
| Milestone, epic | `approved` to `implementing` | First claim of a task of it |
| Story | `approved` to `implementing` | First claim of a task that names it |
| Specification | `approved` to `implementing` | First claim of a task that cites it |
| Story, epic | `implementing` to `done` | A task of it reaches `done` or `superseded` and every task of it is `done` or `superseded`, at least one `done` |
| Gate issue | `draft` to `done`; `draft` to `superseded` | G2 on its gate; G5 on its gate |

### Stop reasons

`stop_reason` (R34): `approval_required` (a pending gate other than a question), each value of
`question_category`, `turn_limit`, `executor_failure`, `pull_request_closed`,
`reconciliation_required`, `stopped_by_person`, `budget_exhausted`. Each stop names its gate,
question, issue or scope. `wayloop next` and `wayloop status` return them (SPEC-0004).

### Errors

Refusals use the report of ADR 0009 C3. Pointers follow RFC 6901 from the root of the input
sent; the field that names the subject (`/issue`, `/dispatch`, `/expected_revision`) carries the
error. A new meaning gets a new code (ADR 0009 C8).

| Requirement | Code | Reason | `expected` |
| --- | --- | --- | --- |
| R4 | `input_class` | `missing` | The field the class requires |
| R13 | `transition_not_allowed` | `state` | The states from which the transition is allowed |
| R14 | `task_not_approved`, `plan_not_approved`, `specification_not_approved` | `state` | The accepted statuses |
| R14 | `dependency_not_done` | `state` | `done` |
| R14 | `issue_on_hold` | `state` | None |
| R14 | `checkout_held`, `lane_busy` | `conflict` | None |
| R14 | `not_in_lane` | `missing` | None |
| R14 | `awaiting_external` | `state` | None |
| R14, R24 | `budget_exhausted` | `range` | None |
| R17 | `dispatch_stale` | `stale` | The open dispatch, when one exists |
| R17 (I3, I25) | `not_holder` | `conflict` | None |
| R10 | `actor_not_authorised` | `requires` | None; carried by `input_ignored` |
| R11 | `question_unanswered` | `requires` | None; carried by `input_ignored` |
| R18, R19 | `checklist_incomplete`, `objection_unruled`, `finding_unverified` | `missing` | The missing requirements, objections or findings; carried by `result_rejected` |
| G4 | `option_unknown` | `enum` | The options of the question; carried by `input_ignored` |
| R23 | `limit_not_raised` | `range` | None; carried by `input_ignored` |
| R33 | `revision_stale` | `stale` | The current revision |

### Examples

`examples/` holds scenarios used by AC35: an initial state, then each consumed event with the
expected issue state, holds and recorded events. They name events and fields of this
specification only; payload shapes come from SPEC-0003, so the files are traces, not fixtures of
an event schema.

- `issue-cycle.happy-path.yaml`: claim to `done` for an issue of tier `substantial`.
- `issue-cycle.repeated-failure.yaml`: two failures of one check, the question, the answer `retry`.
- `issue-cycle.release-and-stale.yaml`: a stale command, a lease expiry, a new claim that resumes.

## Interfaces
- Core: `transition` and `next_steps` of section The two functions, in `crates/wayloop-core`; enums `issue_state`, `gate_kind`, `gate_state`, `resolution`, `hold_reason`, `question_category`, `stop_reason`, `action_kind`, `phase`. All new.
- `wayloop next` returns `actions`, `ready` and `stops`; `wayloop status` returns the phase, the gates, and the state, holds and stop reasons of every issue (SPEC-0004). Both report `meta.revision`. New output fields: additive.
- Commands that change state (`claim`, step requests, session end) take `expected_revision` (SPEC-0004 R5) and refuse with `revision_stale`.
- Events: the names of section Events, with schemas in SPEC-0003. New.
- Configuration: the keys of section Limits and budgets and `gates.<kind>.approvers` (Q9), in the project file of ADR 0010. New.
- Error codes of section Errors. New.

## Dependencies
- SPEC-0001 (Approved): statuses, allowed transitions, hand-over `step` values.
- #228 aligns the `IssueStep` identifiers with the `snake_case` values used here.
- ADR 0011, ADR 0012 and ADR 0013 are Proposed; executors, gates on the tracker and lanes depend on them.
- Follow-up changes in other specifications are listed in the report of the pull request that proposes this specification for approval.

## Plan and tasks
`plan.md` and `tasks.md` are written after the maintainer approves this specification
(`specs/README.md`, Agent workflow). Tasks #48 (transition table and guards), #49 (next step and
ready work) and #58 (dry run on local fixtures) implement it.

## Open questions
- [ ] **Q1. Which steps does the tier `small` skip?** Options: (a) none, every tier runs counter-review and review; (b) `small` skips both `counter_review` and `review`; (c) `small` skips `review` only. Recommendation: (b), set per tier in the policy so a project can choose (a). AGENTS.md already gates small changes with the checks and a reading of the diff; an agent counter-review of a documentation task costs more than the gaps it finds.
- [ ] **Q2. May the loop merge an issue of tier `small` without a person at the gate?** Options: (a) no, every merge waits for an approval; (b) yes, through a standing approval: the policy names the tier and the approver who granted it, and every such merge records `gate_resolved` with that approver and the policy revision; (c) yes, without a gate for `small`. Recommendation: (b). It matches the Merging section of AGENTS.md, keeps a person accountable for every approval, and keeps the gate visible in the ledger. Constitution IV says the loop cannot pass an approval state; (b) reads a standing approval as an approval by a person, and the maintainer should confirm that reading or amend the constitution through an ADR.
- [ ] **Q3. When is an expected revision stale?** Options: (a) strict: any event after the revision; (b) scoped: an event after the revision that names the same issue, or for a project command changes the phase, a gate or the plan. Recommendation: (b). With parallel lanes, (a) refuses most commands for events that do not touch them and costs a turn each time; (b) keeps the guarantee of ADR 0009 C6 for the state the command reads. ADR 0009 C6 defines the revision but not staleness; (b) needs a clarifying amendment of that ADR.
- [ ] **Q4. Where does a new claim resume after a release?** Options: (a) at the state reached; (b) always at `reanalysis`; (c) at the state reached, except at `reanalysis` when the default branch changed files cited by the issue since the release. Recommendation: (a). The hand-over carries the context (SPEC-0003 R13), the state stays a function of the ledger, and (c) needs a git observation that this machine does not consume yet; (c) can be added later as a new transition.
- [ ] **Q5. Which issues are ready without an active sprint?** Options: (a) none: work runs only inside a sprint; (b) each area of the policy is a lane, with the paths of the area; (c) one lane for the whole project. Recommendation: (b). Areas already declare paths in the policy (SPEC-0021) and epics and tasks carry an area (SPEC-0001); (a) stops the loop between sprints and (c) wastes parallel workers. It requires the policy to check that area paths are disjoint.
- [ ] **Q6. Default limits.** Options: (a) turns 20, attempts 3, same failure 2, executor retries 2, lease 120 minutes; (b) no defaults: `wayloop init` writes explicit values and a missing value is a configuration error. Recommendation: (a), listed in `meta.defaults_applied` (ADR 0009 C2). An attempt uses about five executor turns, so 20 turns cover three attempts with one review round; a lease of 120 minutes outlasts a long `build` step without events.
- [ ] **Q7. What happens when the pull request of an issue is closed without merge?** Options: (a) hold with `pull_request_closed` until a person unblocks the issue, which then resumes at `build`; (b) return to `build` at once and open a new pull request; (c) supersede the task. Recommendation: (a). A person closed the pull request for a reason the engine cannot read; (b) can loop against that person and (c) decides the plan for them.
- [ ] **Q8. What is a budget scope?** Options: (a) the project phase and the active sprint; (b) the step kind (reanalysis, build, review and so on) and the sprint; (c) the project phase, the sprint and the issue. Recommendation: (a). It matches "every phase has a token budget" (constitution VI) and SPEC-0006 R6; the per-step budget of SPEC-0008 R2 bounds the size of a brief, not the spend, and the turn limit already bounds one issue.
- [ ] **Q9. Who is an authorised approver?** Options: (a) every account with write access to the repository, as reported by the tracker; (b) a list per gate kind in the project file, defaulting to the owner of the subject (the `Owner` of a specification, the `owner` of a brief) plus the accounts with the maintain or admin role; (c) the owner of the subject only. Recommendation: (b). Write access includes bots and agents' accounts, which constitution IV excludes; an explicit list is reviewed in a pull request and recorded by `limits_set`.
