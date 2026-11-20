# SPEC-0002: State machine of the project and of the issue cycle

| | |
| --- | --- |
| Status | Draft |
| Milestone | M1 · Engine core |
| Owner | @salvo8519 |
| Related | #11, #48, #49, #58, #228; ADR 0002, 0003, 0006, 0007, 0008, 0009, 0010, 0011, 0012, 0013, 0026, 0028, 0029; SPEC-0001, SPEC-0003, SPEC-0004, SPEC-0005, SPEC-0006, SPEC-0007, SPEC-0008, SPEC-0009, SPEC-0013, SPEC-0021 |
| Commit scope | state |

## Problem
The engine decides every step of the loop. Without an explicit state machine the decision moves into prompts, where it cannot be tested, replayed or audited. Maintainers need the loop to stop exactly at the gates they own; agents need the engine to tell them the next allowed step; parallel workers need a single, deterministic answer to the question "what is ready".

## Scope
**In scope:** The state of a project (its phase), of a gate and of the issue cycle of a planned task. The four pure functions of the core: `decide`, `apply`, `due_events` and `next_steps`. Every transition with its input, guard, actor and recorded events. The approval states the loop cannot pass, and the merge waiver of ADR 0029. Holds (the `blocked` state): how one is entered and left. The configuration that the state depends on (approvers, limits, budgets, policy, plan) and how it enters the state. Limits on turns, attempts, executor retries, leases and token budgets, and the rule for repeated failure. Ready work for parallel lanes (ADR 0006, ADR 0013). Stale revisions (ADR 0009 C6, as amended) and stale dispatches. The inputs the machine decides and the payloads of the event kinds it owns; the catalogue of every kind is in SPEC-0003. The engine-taken transitions of artifact statuses listed in SPEC-0001.

**Out of scope:** The event envelope, the hash chain, persistence, replay mechanics and the payloads of the kinds owned by SPEC-0003. Execution of steps, worktrees and the worker pool (SPEC-0006). Executors, briefs and result schemas of each step (SPEC-0008). The tracker mapping of labels, approvals, answers and person requests (SPEC-0005, SPEC-0007). The content of gates and questions (SPEC-0007, SPEC-0012). Generation of plan items (SPEC-0013). Artifact fields and the allowed status transitions themselves (SPEC-0001). Transitions of change deltas (SPEC-0009). Policy rules as data and their evaluation on a pull request (SPEC-0021).

## User stories
- As a maintainer, I want the loop to stop exactly at the gates I own, so that nothing irreversible happens without me.
- As a maintainer, I want every stop to carry its reason, so that I know in one read what the loop needs from me.
- As a maintainer, I want only the people I listed to approve, and every widening of limits or policy to pass through them, so that an agent cannot give itself more room.
- As an agent, I want the engine to tell me the next allowed step, so that I never decide the process myself.
- As a runtime, I want the list of ready work in a fixed order, so that parallel workers never take the same issue or the same lane.

## Requirements (EARS)
- **R1**: THE ENGINE SHALL decide every input with a function `decide(state, input, at)` that returns either the events to record or a refusal, and SHALL change the state only with a function `apply(state, event)` of one recorded event; neither function SHALL read a clock, a file, the network, the environment or a random source.
- **R2**: THE ENGINE SHALL rebuild the state only by applying the recorded events in ledger order to the initial state, and SHALL never call `decide` during a replay.
- **R3**: THE ENGINE SHALL record in every event it emits for an input the cause of that input: `input_hash` for a command, `dispatch` for a result, `external_id` for an observation.
- **R4**: THE ENGINE SHALL accept each input only in the input class given by the table Inputs, and refuse a command without `expected_revision`, a result without `dispatch` or an observation without `external_id` with the code `input_class` and the reason `missing`.
- **R5**: THE ENGINE SHALL return the events due by time only from a function `due_events(state, now)`, and the next steps (actions, ready work and stop reasons) only from a function `next_steps(state, now, free_workers)`, both pure, with the outputs in the total order of section Ready work and lanes.
- **R6**: IF an input fails one or more guards of its transition, THEN THE ENGINE SHALL refuse it, report every unmet guard in one pass with its code from section Errors, and record nothing, except the cost of a stale result under R21.
- **R7**: THE ENGINE SHALL apply the approvers, limits, budgets and policy recorded by the latest `limits_set` and `policy_set` events, never the configuration file read at the time of a decision.
- **R8**: WHEN `run_start_requested` carries an effective configuration that differs from the recorded one, THE ENGINE SHALL record at once the changes that only remove approvers, lower limits or budgets, or tighten the policy, and SHALL open a gate of kind `configuration` for every change that adds an approver, raises a limit or a budget, or loosens the policy.
- **R9**: THE ENGINE SHALL record `run_started` with the account of the runtime and, WHILE that account is a recorded approver, propose no action and report the stop reason `engine_account_is_approver`.
- **R10**: THE ENGINE SHALL derive the phase of the project from the statuses of its artifacts with the function of section Project phase, and record `phase_changed` whenever an event changes the derived phase.
- **R11**: WHILE an open brief has no council report in `draft`, or an approved council report has no pending plan proposal, THE ENGINE SHALL propose the action `run_council` or `generate_plan` for that subject within the budget of the phase, and dispatch it through `project_step_requested`.
- **R12**: WHEN a gate of kind `council_report` or `roadmap` is rejected, THE ENGINE SHALL move its subject to `superseded`, propose no new `run_council` or `generate_plan` for that brief, and report the stop reason `rejected_awaiting_input` until an approver requests a new run or the brief is superseded.
- **R13**: WHEN a subject reaches one of the approval states of section Gates and approval states, THE ENGINE SHALL open a gate, record `gate_opened`, and propose no action that passes the gate while it is pending.
- **R14**: THE ENGINE SHALL resolve a gate only with an `approval_recorded` or `question_answered` whose actor is a recorded approver; IF the actor is not a recorded approver, THEN THE ENGINE SHALL record `input_ignored` with the code `actor_not_authorised` and keep the gate pending.
- **R15**: IF an approval of a gate arrives while a subject guard of the gate does not hold, THEN THE ENGINE SHALL record `input_ignored` with the code of the unmet guard and keep the gate pending.
- **R16**: WHEN the subject of a pending gate is superseded, or the pull request of a merge gate gets a new head or is closed, THE ENGINE SHALL withdraw the gate and record `gate_withdrawn` with the reason.
- **R17**: WHERE the recorded policy waives the merge gate for the tier `small`, WHEN an issue of tier `small` reaches `merge`, THE ENGINE SHALL record `gate_waived` with the policy revision and the approver of that revision instead of opening the gate, and SHALL never record the waiver as `gate_resolved`.
- **R18**: THE ENGINE SHALL change the issue-cycle state of a planned task only through a transition of the table Issue transitions; IF a command or a result names a transition that the table does not allow from the current state, THEN THE ENGINE SHALL refuse it with the code `transition_not_allowed`, or `step_mismatch` when the requested step is not the current state.
- **R19**: THE ENGINE SHALL consider an issue ready only when every condition of the table Readiness holds, and SHALL report, for an issue that is not ready, every unmet condition with its code.
- **R20**: WHEN a claim of a ready issue is accepted, THE ENGINE SHALL record `checkout_acquired` with the holder, session, executor, branch and worktree, move an issue in state `open` to `claim`, keep the state of an issue that was claimed before, and record the artifact transitions of section Artifact transitions taken by the engine.
- **R21**: WHEN a step of a checked-out issue is requested, THE ENGINE SHALL record `step_started` as the dispatch of the step and count one turn if an executor runs the step; IF a result cites a dispatch that is not the open dispatch of its subject, THEN THE ENGINE SHALL refuse it with the code `dispatch_stale` and record only the `executor_call_recorded` the result carries.
- **R22**: THE ENGINE SHALL extend the lease of a session with every event of that session, including `lease_renewed` sent by its holder; WHEN `due_events` finds a session whose lease has ended, THE ENGINE SHALL record `checkout_released` with the reason `lease_expired` for each of its checkouts and `session_ended` with the outcome `abandoned`, keep the issue-cycle state and holds, and close the open dispatch.
- **R23**: IF a result does not validate against the schema of its step or fails a guard of its transition, or an executor fails or times out, THEN THE ENGINE SHALL record `result_rejected` or `step_retried`, close the dispatch and count one retry; WHEN a failure finds the retries of the step at their limit, THE ENGINE SHALL record `step_failed` and hold the issue with the reason `executor_failure`.
- **R24**: THE ENGINE SHALL complete the step `counter_review` only when every objection has a ruling or a question, the step `checklist` only when the checklist names every requirement cited by the task, and the step `review` only when every finding is verified or rejected with a reason.
- **R25**: THE ENGINE SHALL compute the risk tier of an issue from the recorded policy, its area and the paths of the latest `branch_changes_observed` at its recorded head, and WHERE that tier skips `counter_review` or `review` under the policy, SHALL skip the step and record the skip in `step_completed`.
- **R26**: WHEN a verification or a required check of the pull request fails and neither limit of R27 is reached, THE ENGINE SHALL record `attempt_failed` with the failure note and the failing checks, increase the attempt number by one, and move the issue to `build`.
- **R27**: WHEN a failure leaves one check failed on `limits.same_failure` consecutive attempts, or the failed attempt is attempt number `limits.attempts`, THE ENGINE SHALL record `attempt_failed`, open a question of category `repeated_failure` with the options `retry` and `stop`, and hold the issue instead of starting another attempt.
- **R28**: WHEN an executor step of an issue is requested and the turns of the issue are at least `limits.turns_per_issue`, THE ENGINE SHALL hold the issue with the reason `turn_limit` instead of starting the step; WHEN a `limits_set` sets the limit above the turns, THE ENGINE SHALL remove that hold.
- **R29**: WHEN the tokens recorded in a budget scope reach its budget, THE ENGINE SHALL record `budget_exhausted`, propose no new step in that scope and still accept the results of steps already started; WHEN a `limits_set` raises the budget above the recorded tokens, THE ENGINE SHALL record `budget_restored`.
- **R30**: WHEN an issue enters a hold, THE ENGINE SHALL set the issue-cycle state given by the row of the table Issue transitions, add the reason to its holds, record `issue_blocked` with the reason, and propose no action for the issue while a hold remains.
- **R31**: THE ENGINE SHALL remove a hold only with the input that the table Holds lists for its reason, from a recorded approver where the table says so; WHEN the last hold of an issue is removed, THE ENGINE SHALL record `issue_unblocked` and resume the issue at the state named by the table.
- **R32**: THE ENGINE SHALL never refuse an observation for its state: IF an observation is not consistent with the state of a non-terminal issue, THEN THE ENGINE SHALL apply the fact transition of the table Issue transitions when one exists, and otherwise hold the issue with the reason `reconciliation_required`; IF the issue is `done` or `superseded`, THEN THE ENGINE SHALL record `reconciliation_required` and leave the issue unchanged.
- **R33**: WHEN an observation arrives whose `external_id` is already recorded, THE ENGINE SHALL leave the state unchanged and record nothing, before any other rule is evaluated.
- **R34**: THE ENGINE SHALL move an issue to `done` only on the observation `pull_request_merged` of its closing pull request; WHEN the merge gate was neither approved by a recorded approver nor waived under R17, THE ENGINE SHALL also record `gate_bypassed` and `reconciliation_required`.
- **R35**: WHEN an issue reaches `merge`, THE ENGINE SHALL record `checkout_released` with the reason `awaiting_merge`, keep its task in `implementing`, and count the issue as occupying its lane from its first claim until it is `done` or `superseded`.
- **R36**: WHEN the pull request of an issue is closed without merge, THE ENGINE SHALL hold the issue with the reason `pull_request_closed`, clear its recorded pull request, withdraw its merge gate and release its checkout; WHEN the same pull request is reopened while that hold remains, THE ENGINE SHALL restore the pull request, remove the hold and resume at `pull_request`.
- **R37**: WHEN an open pull request of an issue gets a new head, THE ENGINE SHALL record the head and, at `merge`, withdraw the merge gate and return the issue to `pull_request`; IF a check result or a review names a head other than the recorded head, THEN THE ENGINE SHALL record `input_ignored` with the code `head_stale` and leave the issue unchanged.
- **R38**: THE ENGINE SHALL record the conclusion of every required check at the head it ran on, treat `timed_out` as failed, ignore `cancelled`, and open the merge gate only when every required check of the recorded policy has the conclusion `success` at the recorded head.
- **R39**: THE ENGINE SHALL propose the action `merge` for an issue only while its merge gate is approved or waived and every required check has the conclusion `success` at the recorded head.
- **R40**: WHEN the task of an issue is superseded, THE ENGINE SHALL move the issue to `superseded`, release its checkout, clear its holds, withdraw its gates, and record `issue_superseded`.
- **R41**: IF a command carries an expected revision that is stale under the rule of section Revisions, THEN THE ENGINE SHALL refuse it with the code `revision_stale` and the current revision in `expected`; THE ENGINE SHALL evaluate the staleness and every guard under the lock of the writer, on the latest state.
- **R42**: THE ENGINE SHALL report, for the project and for every issue that cannot progress without a person or a new budget, its stop reasons from the enum `stop_reason`, with the gate, question, issue or scope each one names, and WHILE the ledger is broken or regressed (SPEC-0003 R4, R49) SHALL propose no action and report `ledger_broken`.
- **R43**: THE ENGINE SHALL take the engine transitions of artifact statuses of SPEC-0001 only on the triggers listed in section Artifact transitions taken by the engine, and record each as `artifact_status_changed`.
- **R44**: THE ENGINE SHALL take the approvers only from the explicit list recorded by `wayloop init` and by later `limits_set` events, and SHALL never treat an agent, a bot or the account of the runtime as an approver.

## Acceptance criteria
- **AC1** (R1, R18, R32): Given every pair of issue-cycle state and input of the table Issue transitions, generated with random guards, when decided and applied, then the next state, the holds and the recorded events match the table, and every pair absent from the table is refused with `transition_not_allowed` or `step_mismatch` or, for an observation, recorded as the table says for an inconsistent observation. Verification: property test.
- **AC2** (R1, R5): Given a random state and a random input, when `decide`, `apply`, `due_events` and `next_steps` run twice, then both outputs are byte-identical in canonical JSON; and the core crate passes `cargo clippy -- -D clippy::disallowed_methods -D clippy::disallowed_types` with a `clippy.toml` that disallows reading the system clock, the environment, the file system and random sources, and the hash maps and sets whose iteration order is random. Verification: property test.
- **AC3** (R5, R19): Given a random set of issues, lanes and free workers, when the next steps are computed for the same state with the issues stored in different orders, then the output is identical and ordered as section Ready work and lanes says. Verification: property test.
- **AC4** (R6, R19): Given an issue with one dependency not done, a hold and a checkout held by another session, when a claim is requested, then it is refused with `dependency_not_done`, `issue_on_hold` and `checkout_held` in one pass, each with its pointer, and the ledger is unchanged. Verification: TDD.
- **AC5** (R4): Given a `claim_requested` without an expected revision, a `step_result` without a dispatch and a `pull_request_merged` without an external identifier, when submitted, then each is refused with `input_class` and the reason `missing` at the missing field. Verification: TDD.
- **AC6** (R7): Given a ledger whose latest `limits_set` sets 20 turns and a configuration file changed to 30 turns after it, when the ledger is replayed and the next steps are computed, then the limit applied is 20. Verification: TDD.
- **AC7** (R1, R2, R3): Given a random sequence of inputs decided one after the other by a writer, when the recorded events are applied again from the initial state, then the state is byte-identical to the state the writer held after each event, and no call of `decide` happens during the replay; and given any state and input, when `decide` runs, then every emitted event carries the `input_hash`, the `dispatch` or the `external_id` of the input, according to its class. Verification: property test.
- **AC8** (R8, R44): Given a recorded configuration with approvers `maintainer`, 20 turns and a sprint budget of 100,000 tokens, when a run starts with the approver `other` added, 30 turns and a sprint budget of 50,000, then `limits_set` records the budget of 50,000 at once, a gate of kind `configuration` holds the new approver and the 30 turns, the limit stays 20 while it is pending, an approval by `other` is ignored with `actor_not_authorised`, and an approval by `maintainer` records `limits_set` with both changes. Verification: TDD.
- **AC9** (R9): Given recorded approvers `maintainer` and `wayloop-bot`, when a run starts with the account `wayloop-bot`, then `run_started` is recorded, no action is proposed and the stop reason is `engine_account_is_approver`. Verification: TDD.
- **AC10** (R10): Given the fixture states of the table Project phase, when the phase is derived, then each gives its phase; given a project with an approved milestone and no brief, then the phase is `delivery`; given a `complete` project and a new brief approved, then the phase is `council`. Verification: TDD.
- **AC11** (R11, R29): Given a project in phase `council` with an approved brief and no council report, when the next steps are computed, then `run_council` is proposed; given the budget `budgets.council` exhausted, then it is not proposed and the stop reason is `budget_exhausted` with the scope. Verification: TDD.
- **AC12** (R12): Given a draft council report at its gate, when an approver rejects it, then the report is `superseded`, no `run_council` is proposed and the stop reason is `rejected_awaiting_input`; given `unblock_requested` by an approver on the brief, then `run_council` is proposed again. Verification: TDD.
- **AC13** (R13, R14, R44): Given any sequence of random inputs in which no recorded approver acts, when decided and applied, then no gate is resolved as `approved`, no `limits_set` or `policy_set` widens the configuration, no issue reaches `done` except through a merge observation with `gate_bypassed` or a recorded `gate_waived`, and no artifact passes a person transition of SPEC-0001. Verification: property test.
- **AC14** (R14): Given an issue at `merge` and an approval from an agent account, when processed, then `input_ignored` with `actor_not_authorised` is recorded and the gate stays pending. Verification: TDD.
- **AC15** (R15): Given a draft council report with one question without an answer, when an approver approves it, then `input_ignored` with `question_unanswered` is recorded and the gate stays pending. Verification: TDD.
- **AC16** (R16, R40): Given an issue at `build` with an open question, when its task is superseded, then the issue is `superseded`, its checkout is released with the reason `superseded`, its holds are cleared and the question gate is withdrawn. Verification: TDD.
- **AC17** (R17, R39): Given a policy that waives the merge gate for `small`, an issue whose area and changed paths are `small`, and green required checks at the recorded head, when the checks are observed, then `gate_waived` names the policy revision and the approver of that revision, no `gate_resolved` is recorded, and the action `merge` is proposed; given a changed path of tier `substantial`, then a merge gate is opened instead. Verification: TDD.
- **AC18** (R19): Given one ready issue, when each readiness condition is made false in turn, then the issue is not ready and the report names exactly that condition's code. Verification: TDD.
- **AC19** (R19, R35): Given an active sprint with two lanes of three ready issues each and three free workers, when the next steps are computed, then two claims are proposed, the first ready issue of each lane; given one lane whose first issue waits at `merge` without a checkout, then no claim is proposed for that lane. Verification: TDD.
- **AC20** (R19): Given random sprints, workers and input sequences, when the runtime follows the proposed actions, then no lane ever has two issues in states from `claim` to `merge` and the number of checkouts never exceeds the workers. Verification: property test.
- **AC21** (R19): Given no active sprint and a policy with the areas `core` and `docs`, when the next steps are computed, then each area is a lane and an issue without area is reported with `not_in_lane`; given two areas whose paths overlap, then the policy is refused with `area_paths_overlap`. Verification: TDD.
- **AC22** (R20, R43): Given the first claim of a task whose story, specification, epic and milestone are `approved`, when accepted, then the issue is at `claim` with a checkout naming its worktree, and the task, story, specification, epic and milestone move to `implementing`, each with `artifact_status_changed`. Verification: TDD.
- **AC23** (R21): Given an issue at `build` with dispatch D2 after a retry of dispatch D1, when a result citing D1 arrives with an executor call record, then it is refused with `dispatch_stale` and only `executor_call_recorded` is recorded; given a result citing D2, then it is accepted. Verification: TDD.
- **AC24** (R22): Given a lease of 120 minutes and a session whose latest event was recorded at 10:00, when `due_events` runs at 11:59, then it returns nothing; when it runs at 12:00, then it returns `checkout_released` with `lease_expired` and `session_ended` with `abandoned`, the issue keeps its state and its task moves to `approved`; given `lease_renewed` at 11:30, then nothing is due at 12:00. Verification: TDD.
- **AC25** (R23): Given `limits.executor_retries` set to 2, when the executor of a step times out three times, then two `step_retried`, one `step_failed` and one `issue_blocked` with `executor_failure` are recorded; given a result that does not validate, then `result_rejected` is recorded and counts as one retry. Verification: TDD.
- **AC26** (R24): Given a checklist result that omits requirement R2 cited by the task, a counter-review result with an objection without a ruling and a review result with a finding neither verified nor rejected, when each is submitted, then each is recorded as `result_rejected` with `checklist_incomplete`, `objection_unruled` or `finding_unverified`. Verification: TDD.
- **AC27** (R25): Given a policy in which the tier `small` skips `counter_review` and `review`, when an issue of area tier `small` completes `reanalysis` and `verify`, then it moves to `checklist` and `pull_request`, and each `step_completed` lists the skipped step; given a changed path of tier `substantial` observed before `verify` completes, then `review` is not skipped. Verification: TDD.
- **AC28** (R26): Given an issue at `verify` at attempt 1 of 3, when the verification fails on check `unit`, then `attempt_failed` names `unit` and carries a failure note, the attempt number is 2 and the issue is at `build`. Verification: TDD.
- **AC29** (R27): Given `limits.attempts` 3 and `limits.same_failure` 5, when attempts 1, 2 and 3 fail on different checks, then the failures of attempts 1 and 2 move the issue to `build` and the failure of attempt 3 opens the question `repeated_failure`; no attempt 4 is started. Verification: TDD.
- **AC30** (R27, R31): Given `limits.same_failure` 2, when check `unit` fails on two consecutive attempts, then a question of category `repeated_failure` with the options `retry` and `stop` is opened and the issue is held; when an approver answers `retry`, then the attempt number is 1, the failure counts are reset and the issue resumes at `build`; when an approver answers `stop` instead, then the hold becomes `stopped_by_person`. Verification: TDD.
- **AC31** (R28): Given a limit of N turns, when N executor steps have started and another is requested, then the issue is held with `turn_limit` and no `step_started` is recorded; given `limits_set` with N + 5 turns after an approved configuration gate, then the hold is removed and the step can be requested. Verification: TDD.
- **AC32** (R29): Given a sprint budget of N tokens, when an executor call brings the recorded tokens of the sprint to N, then `budget_exhausted` is recorded, no step of the sprint is proposed, and a result of a step already started is still accepted; given `limits_set` with a budget above N, then `budget_restored` is recorded and steps are proposed again. Verification: TDD.
- **AC33** (R30, R31): Given an issue at `counter_review`, when its result raises a question of category `security_sensitive_action`, then the issue keeps `counter_review`, is held with `question` and no action is proposed for it; given an answer from a person who is not an approver, then `input_ignored` is recorded; given a free-text answer from an approver, then `question_answered` and `issue_unblocked` are recorded and the issue resumes at `counter_review`. Verification: TDD.
- **AC34** (R32, R34): Given an issue at `pull_request` before its checks complete, when `pull_request_merged` is observed, then the issue is `done` and `gate_bypassed` and `reconciliation_required` are recorded; given an issue at `build`, when `issue_closed` without a merge is observed, then it is held with `reconciliation_required`; given a `done` issue, when `issue_reopened` is observed, then `reconciliation_required` is recorded, no hold is added and the issue stays `done`. Verification: TDD.
- **AC35** (R33): Given any observation delivered k times with the same external identifier, k from 1 to 5, in any interleaving with other inputs, when decided and applied, then the state and the ledger equal those after one delivery. Verification: property test.
- **AC36** (R34, R35): Given an issue at `merge` with an approved merge gate, when its pull request is merged, then the issue is `done`, its task is `done` and no `gate_bypassed` is recorded; given an issue that reached `merge`, then `checkout_released` with `awaiting_merge` was recorded and its task stayed `implementing`. Verification: TDD.
- **AC37** (R36): Given an issue at `merge`, when its pull request is closed without merge, then it is held with `pull_request_closed`, its pull request is cleared and the merge gate withdrawn; given `pull_request_reopened` of the same number, then the hold is removed and the issue is at `pull_request`; given `unblock_requested` by an approver instead, then it resumes at `build`. Verification: TDD.
- **AC38** (R37, R38): Given an issue at `merge` with an approved gate at head H1, when the head H2 is observed, then the gate is withdrawn and the issue is at `pull_request`; given then a check result and a review at H1, then each is recorded as `input_ignored` with `head_stale`; given required checks at H2 with one `cancelled` and later `success`, then the merge gate opens; given one `timed_out`, then `attempt_failed` is recorded. Verification: TDD.
- **AC39** (R39): Given an issue at `merge` with an approved gate, when the next steps are computed before the required checks at the recorded head are all `success`, then no `merge` action is proposed; after they are, then one is. Verification: TDD.
- **AC40** (R41): Given a revision R1, an event on issue 13 and a command on issue 12 with the expected revision R1, when decided, then it is accepted; given an event on issue 12 after R1, then the command is refused with `revision_stale`, `expected` holds the current revision and the ledger is unchanged; given a project command after a `gate_opened` on another issue, then it is refused. Verification: TDD.
- **AC41** (R42): Given a project with a pending council report gate, an issue held with `turn_limit` and a sprint budget exhausted, when the status is computed, then the stop reasons are `approval_required`, `turn_limit` and `budget_exhausted`, each naming its gate, issue or scope, and every value matches `^[a-z][a-z0-9_]*$`; given a state loaded from a broken ledger, then the only stop reason is `ledger_broken` and no action is proposed. Verification: contract test.
- **AC42** (R43): Given the last task of a story merged while another task of the story is superseded, when processed, then the story moves to `done`; given an epic whose only task is superseded, then the epic does not move. Verification: TDD.
- **AC43** (R1, R5, R18, R20, R26, R34): Given the scenarios of `examples/`, when each is applied input by input to its initial state, then every expected state, hold, refusal and recorded event matches. Verification: end-to-end.

## Model

### Machines and state

The engine runs three machines over one state, which is a projection of the ledger (SPEC-0003):

| Machine | Instances | State | Section |
| --- | --- | --- | --- |
| Project phase | One per project | `phase`, derived from artifact statuses | Project phase |
| Gate | One per approval, question or configuration change | `gate_state` and its resolution | Gates and approval states |
| Issue cycle | One per planned task issue | `issue_state`, checkout, holds, dispatch, counters | Issue cycle |

The state also holds the recorded configuration (approvers, limits, budgets, policy), the plan
as last observed, the sessions and their latest event, the project dispatches, the recorded
`external_id` values, the tokens recorded per budget scope and the account of the runtime.

The issue-cycle state is not the status of the task artifact (SPEC-0001 R5). A task in status
`implementing` may be at any non-terminal issue-cycle state, checked out or not, held or not.
`blocked` is not a value of any enum: an issue is blocked while its list of holds is not empty.

The state of one issue:

| Field | Meaning |
| --- | --- |
| `issue_state` | One of the values of the table Issue states |
| `checkout` | Absent, or `{holder, session, executor, branch, worktree}` from `checkout_acquired` (SPEC-0003) |
| `holds` | List of `{reason, gate}`; `gate` names the question of a hold with the reason `question` |
| `dispatch` | Absent, or the hash of the open `step_started` event |
| `turns` | Executor steps started for the issue |
| `attempt` | Number of the current attempt, from 1 |
| `failures` | For each check name, the number of consecutive attempts on which it failed |
| `retries` | Failures of the open step: executor failures and rejected results |
| `pull_request` | Absent, or `{number, head_sha, checks, head_pending}`; `checks` maps each check name to its latest conclusion at `head_sha`; `head_pending` is true from a failed required check (I20) until the new head is observed (I21) |
| `changed_paths` | The paths of the latest `branch_changes_observed` and the head they were observed at |

### Time

Time enters the core only as data.

- `decide` takes `at`, the time the writer gives for the events of this input after the rule of
  SPEC-0003 R27; it is written as `recorded_at` of every event it returns, and no guard compares
  it with any other time.
- `due_events(state, now)` is the only function that compares a recorded time with the current
  time. The runtime calls it, and appends what it returns under the lock, before it decides any
  command (SPEC-0003 R28); a read never calls it.
- The lease of a session ends at the `recorded_at` of the latest event of that session plus
  `limits.lease_minutes`. `lease_renewed` is that event when nothing else happens: the runtime of
  the holder sends `lease_renewal_requested` every quarter of the lease while a step runs.

### Inputs

Every input belongs to one class (R4). The class decides the cause recorded, how staleness is
detected and what happens when the input does not fit the state.

| Class | Sent by | Carries | Cause recorded (R3) | When it does not fit the state |
| --- | --- | --- | --- | --- |
| Command | Runtime or agent, through a command (SPEC-0004) | `expected_revision` | `input_hash` | Refused (R6, R18, R41); nothing recorded |
| Result | Runtime, for an open dispatch: executor output, check results, a council report or a plan | `dispatch` | `dispatch` | Stale dispatch: refused, only its cost recorded (R21). Invalid or failing a guard: `result_rejected` (R23), because model output that does not validate becomes an error event (ADR 0003) |
| Observation | Adapters: tracker, git, plan; persons act through the tracker (ADR 0012) | `external_id`, `actor` when a person acts | `external_id` | Never refused for its state (R32); duplicates ignored (R33); unauthorised actors recorded as `input_ignored` (R14) |

The ledger records what happened, not what was asked: an accepted input is recorded through the
events it causes, and a refused command or a stale result is not. An observation always causes
at least one event, so that its `external_id` is recorded; one that changes nothing is recorded
as `input_ignored` with the code `no_effect`.

Persons act only through the tracker. The tracker adapter turns a comment, a review or a label
change of an authenticated handle into an observation whose `actor` is that handle (SPEC-0005,
SPEC-0007); the core compares the handle with the recorded approvers. The commands a person runs
locally, such as `wayloop init` and `wayloop ledger recover`, follow SPEC-0003 section Person-only
commands: they are not exposed through the MCP server, the SDKs or tool definitions.

### The four functions

```text
decide(state, input, at)              -> Ok { events: [event] } | Refused { errors }
apply(state, event)                   -> state
due_events(state, now)                -> [event]
next_steps(state, now, free_workers)  -> { actions, ready, stops }
```

- `decide` validates the input, evaluates the guards and returns the events in the order they
  are recorded. It is total: every pair of state and input gives events or a refusal, never a
  panic. Each event it returns is applied before the next one is computed, so that
  `apply(apply(s, e1), e2)` is the state after the input.
- `apply` is the only way the state changes (R1). It is total over the kinds of the catalogue of
  SPEC-0003 and never refuses: an event is recorded only after `decide` accepted it, and a
  recorded event is a fact for every later replay. A replay is a fold of `apply` (R2); the
  events carry everything the fold needs, including the tier inputs, the plan and the policy.
- `due_events` returns the events due at `now`, in the order of section Ready work and lanes.
- `next_steps` changes nothing. `actions` lists what the runtime may start now, each of kind
  `run_step`, `merge`, `run_council`, `generate_plan`, `reconcile` or `claim`; `ready` lists
  ready issues that get no action for lack of workers; `stops` lists the stop reasons (R42). The
  runtime turns an action into a command; it never chooses among actions (ADR 0006: the core
  decides, the runtime executes).
- The same arguments give the same output, byte for byte in canonical JSON (constitution II).

### Configuration in the state

Approvers, limits, budgets and policy come from the state (R7), so that a replay never depends
on the configuration of the machine that replays.

- `wayloop init`, run by a person at an interactive terminal, records the first `limits_set`,
  with the explicit list of approvers it is given, and the first `policy_set`, each with that
  person as `approved_by` (R44). Agents, bots and the account of the runtime are never
  approvers; `wayloop init` refuses an empty list.
- `run_start_requested` carries the effective configuration (ADR 0010) and the account of the
  runtime. `decide` compares it with the recorded one (R8). Removing an approver, lowering a
  limit or a budget, and tightening the policy are recorded at once in `limits_set` or
  `policy_set`. Adding an approver, raising a limit or a budget, and loosening the policy open
  one gate of kind `configuration` that holds the proposed values; only an already recorded
  approver resolves it, and its approval records `limits_set` or `policy_set` with that approver
  as `approved_by`. A configuration that removes every approver is refused with
  `approvers_empty`.
- Loosening the policy means: widening the tier `small` (a lower tier for an area or a path),
  adding a skipped step or a merge waiver, or removing a required check. Every other policy
  change tightens it.
- `run_started` records the account of the runtime (R9). While that account is an approver, the
  engine proposes nothing: the runtime would otherwise approve its own work.
- The plan enters the state through `plan_observed`, which the plan adapter emits from the
  tracker and the repository: items with their type, status, links, area and dependencies, and
  the active sprint with its lanes. The first `plan_observed` after `ledger_created` is the
  baseline and is taken as it is. Later, a new item is taken only in `draft`, and a status that
  differs from the state is not taken: the ledger is the authority on statuses (SPEC-0001 R17),
  and `decide` records `reconciliation_required` with the code `status_conflict` for each such
  item. Statuses change through gates, person requests and the transitions of R43.
- The paths changed on a branch enter through `branch_changes_observed`, which the git adapter
  emits for the head of the branch. The engine computes the tier from them (R25); the agent
  never reports its own tier.

### Project phase

The phase is a function of artifact statuses, evaluated in this order; the first row that
holds gives the phase (R10). Deriving it, instead of storing it, keeps it consistent with the
artifacts by construction, and lets a project adopt Wayloop with an existing roadmap: it starts
in `delivery`.

An open brief is an approved brief, not superseded, for which no milestone generated from one of
its council reports is `approved`, `implementing` or `done`. Several briefs may be open at once;
each is a subject of its own.

| Phase | Holds when |
| --- | --- |
| `delivery` | At least one milestone is `approved` or `implementing` |
| `planning` | An open brief has an `approved` council report |
| `council` | A brief is open |
| `complete` | At least one milestone is `done` |
| `intake` | Otherwise |

A project in `complete` returns to `council` when a new brief is approved: the phase is derived,
so it never reaches a state without a way out.

Project transitions (each records `phase_changed` when the derived phase changes):

| # | From | Input (class) | Guard | Actor | To | Recorded |
| --- | --- | --- | --- | --- | --- | --- |
| P1 | Any | `brief_submitted` (observation) | Brief valid (SPEC-0001), in `draft` | Person | Same | `gate_opened` (kind `brief`) |
| P2 | Any | `approval` approving the brief gate | G2 | Person | Derived: `council` unless `delivery` holds | `approval_recorded`, `gate_resolved`, `artifact_status_changed` (brief `approved`, gate issue `done`), `phase_changed` |
| P3 | Any | `project_step_requested` (command), step `council` or `plan` | R11 applies to the subject; budget of the phase not exhausted; no open project dispatch for the subject | Engine | Same | `step_started` (project dispatch) |
| P4 | `council` | `council_report` (result) | Dispatch current; report valid, its brief open | Agent | `council` | `step_completed`, `gate_opened` (kind `council_report`) |
| P5 | Any | `approval` approving the council report gate | G2; every question answered (SPEC-0001) | Person | Derived: `planning` | `approval_recorded`, `gate_resolved`, `artifact_status_changed`, `phase_changed` |
| P6 | `planning` | `plan_proposal` (result) | Dispatch current; every item valid (SPEC-0001), at least one milestone, from the approved report | Agent | `planning` | `step_completed`, `gate_opened` (kind `roadmap`) |
| P7 | Any | `approval` approving a `roadmap` or `roadmap_change` gate that approves a milestone | G2 | Person | Derived: `delivery` | `approval_recorded`, `gate_resolved`, `artifact_status_changed` for each item, `phase_changed` |
| P8 | Any | `approval` rejecting a `council_report` or `roadmap` gate | G3 | Person | Derived | `approval_recorded`, `gate_resolved` (`rejected`), `artifact_status_changed` (report or plan items to `superseded`); stop `rejected_awaiting_input` (R12) |
| P9 | Any | `unblock_requested` on a brief with a rejected report or plan | Actor is an approver | Person | Same | `issue_unblocked` naming the brief; R11 applies again |
| P10 | `delivery` | `status_change_requested`: milestone to `done` (gate met) | SPEC-0001 transition allowed; actor is an approver | Person | Derived: `complete` when no milestone remains `approved` or `implementing` | `artifact_status_changed`, `phase_changed` |
| P11 | Any | `status_change_requested`: a brief or council report to `superseded` | SPEC-0001 transition allowed; actor is an approver | Person | Derived | `artifact_status_changed`, `gate_withdrawn` for its gates, `phase_changed` |
| P12 | Any | `project_step_requested`, step `reconcile`, then `roadmap_change_proposal` (result) | Dispatch current; change valid against the current plan | Agent | Same | `step_started`; then `step_completed`, `gate_opened` (kind `roadmap_change`) |

Reconciliation (SPEC-0007 R3) runs in any phase. Its proposal opens a gate of kind
`roadmap_change`, and the change is applied by the `gate_resolved` that approves it, through the
`artifact_status_changed` events it records (SPEC-0007 R4). The survey (SPEC-0010, SPEC-0011)
may run in any phase and changes no phase by itself.

### Gates and approval states

A gate is a pending decision of a person (constitution IV, ADR 0012). The loop cannot pass it:
no transition resolves a gate without an input from a recorded approver (R14), and `next_steps`
proposes no action that depends on a pending gate (R13). The identity of a gate is the hash of
its `gate_opened` event.

Approval states:

| Gate kind | Opened when | Subject guards checked on approval | Effect of `approved` | Effect of `rejected` |
| --- | --- | --- | --- | --- |
| `brief` | A brief is submitted (P1) | Brief valid | Brief `approved` (P2) | Brief stays `draft`; a new submission opens a new gate |
| `council_report` | A report is returned (P4) | Every question answered (`question_unanswered`) | Report `approved` (P5) | Report `superseded`; stop `rejected_awaiting_input` (P8) |
| `roadmap` | A plan proposal is returned (P6) | Every item valid | Items `approved` (P7) | Items `superseded`; stop `rejected_awaiting_input` (P8) |
| `roadmap_change` | A reconciliation proposal is returned (P12) | Change applies to the current plan (`plan_changed`) | Change applied (SPEC-0007 R4) | Nothing changes |
| `specification` | `specification_submitted` for a specification in `draft` or `surveyed` | Specification valid | Specification `approved` | Stays as it was |
| `merge` | An issue reaches `merge` and the policy does not waive it (I16) | Every required check `success` at the recorded head (`checks_not_green`) | The merge may happen; `done` follows the merge (I18) | Issue to `build` (I19) |
| `configuration` | A run starts with a wider configuration (R8) | The proposal still differs from the recorded configuration | `limits_set` or `policy_set` with the proposal | Recorded configuration kept |
| `question` | An agent or the engine raises a question (table Questions) | For `repeated_failure`: answer `retry` or `stop` (`option_unknown`) | Not applicable: resolution `answered` | Not applicable |

Gate transitions:

| # | From | Input | Guard | Actor | To | Recorded |
| --- | --- | --- | --- | --- | --- | --- |
| G1 | (none) | Subject reaches an approval state | None | Engine | `pending` | `gate_opened` |
| G2 | `pending` | `approval` with decision `approve` | Actor is a recorded approver (R14); subject guards (R15) | Person | `resolved`, resolution `approved` | `approval_recorded`, `gate_resolved` |
| G3 | `pending` | `approval` with decision `reject` | Actor is a recorded approver | Person | `resolved`, resolution `rejected` | `approval_recorded`, `gate_resolved` |
| G4 | `pending`, kind `question` | `answer` | Actor is a recorded approver; for `repeated_failure`, the answer is an option | Person | `resolved`, resolution `answered` | `question_answered`, `gate_resolved` |
| G5 | `pending` | Subject superseded; new head or close of the pull request of a merge gate; issue `done` through a bypass | None | Engine, on the trigger | `withdrawn` | `gate_withdrawn` with the reason |
| G6 | `pending` | `approval` or `answer` failing G2 to G4 | None | Any | `pending` | `input_ignored` with the code of the failure |
| G7 | (none) | Issue of tier `small` reaches `merge`; policy waives the merge gate | Tier computed by the engine (R25) | Engine, under the policy | No gate: waived | `gate_waived` with the policy revision and its approver |

`gate_state` is `pending`, `resolved` or `withdrawn`; `resolution` is `approved`, `rejected` or
`answered`. A gate issue of SPEC-0001 is `done` when its gate is `approved` and `superseded` when
its gate is withdrawn. How a person expresses an approval or an answer on the tracker is decided
by SPEC-0007; this machine consumes the normalised observation.

The merge waiver (G7) follows ADR 0029: for the tier `small`, merge is not an approval state
when the recorded policy waives it. The person accountable is the approver of the policy
revision that holds the waiver, and `gate_waived` names both. A waiver is never recorded as
`gate_resolved` by a person, and a policy change that widens `small` or adds a waiver passes the
`configuration` gate (R8).

### Issue cycle

Issue states, in order. `Actor` is who does the work of the step; `Turn` says whether starting
it counts a turn (R21).

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
| `pull_request` | The pull request is opened or updated, then its required checks run | Engine | No |
| `merge` | Approval state: the merge gate is pending, approved or waived; the merge is awaited | Person | No |
| `done` | Closing pull request merged; terminal | None | No |
| `superseded` | Task superseded; terminal | None | No |

The values from `claim` to `merge` are the `step` values of the hand-over (SPEC-0001) and the
identifiers of `IssueStep` in `crates/wayloop-core`, in `snake_case` after #228.

Issue transitions. `X` is any state from `claim` to `pull_request`; `N` is any non-terminal
state. A row without a checkout guard applies whether or not the issue is checked out. Row I0 is
evaluated before every other row (R33).

| # | From | Input (class) | Guard | Actor | To | Recorded |
| --- | --- | --- | --- | --- | --- | --- |
| I0 | Any | Any observation whose `external_id` is recorded | None | None | Unchanged | Nothing |
| I1 | `open` | `claim_requested` (command) | Ready (R19); the session is open | Engine | `claim`, checked out | `checkout_acquired`, artifact transitions (R43) |
| I2 | `N` except `open` and `merge`, not checked out | `claim_requested` (command) | Ready (R19); the session is open | Engine | Same state, checked out | `checkout_acquired`, task to `implementing` if `approved` |
| I3 | `X`, checked out, no dispatch, no hold | `step_requested` (command) | Sender is the holder (`checkout_not_held`); the step is the current state (`step_mismatch`); turns below the limit for an executor step; no budget scope of the issue exhausted | Engine | Same state, dispatch open | `step_started` |
| I4 | `X`, executor step, as I3 | `step_requested` (command) | Turns at least the limit | Engine | Same state, hold `turn_limit` | `issue_blocked` |
| I5 | `claim` | `step_result` (result) | Dispatch current; branch and worktree reported | Engine | `reanalysis` | `step_completed` |
| I6 | `reanalysis` | `step_result`, outcome `completed` | Dispatch current; result valid | Agent | `counter_review`, or `checklist` when the tier skips it (R25) | `step_completed` |
| I7 | `counter_review` | `step_result`, outcome `completed` | As I6; every objection ruled (R24) | Agent | `checklist` | `step_completed` |
| I8 | `checklist` | `step_result`, outcome `completed` | As I6; checklist names every cited requirement (R24) | Agent | `build` | `step_completed` |
| I9 | `build` | `step_result`, outcome `completed` | As I6 | Agent | `verify` | `step_completed` |
| I10 | `verify` | `verification_result` (result), every check passed | Dispatch current | Engine | `review`, or `pull_request` when the tier skips it (R25) | `step_completed`; passing checks reset in `failures` |
| I11 | `verify` | `verification_result`, a check failed | Limits of R27 not reached | Engine | `build`, attempt plus one | `attempt_failed` |
| I12 | `verify`, `pull_request` | `verification_result` or `pull_request_checks_completed` with a required check failed | Limits of R27 reached | Engine | `build`, hold `question` | `attempt_failed`, `gate_opened` (question `repeated_failure`), `issue_blocked` |
| I13 | `review` | `step_result`, outcome `completed` | As I6; every finding verified or rejected (R24) | Agent | `pull_request` | `step_completed` |
| I14 | `review` | `step_result`, outcome `changes_requested` | As I13; at least one verified finding | Agent | `build` | `step_completed` |
| I15 | `pull_request`, dispatch open, no pull request | `pull_request_opened` (observation) | Branch of the issue; body closes the issue (SPEC-0001 R10) | Engine | Same state, pull request recorded, dispatch closed | `pull_request_opened`, `step_completed` |
| I16 | `pull_request`, pull request recorded, no dispatch | `pull_request_checks_completed` (observation): every required check `success` at the recorded head | Tier not waived (G7) | Engine | `merge`, not checked out | `pull_request_checks_completed`, `gate_opened` (kind `merge`), `checkout_released` (`awaiting_merge`) if checked out |
| I17 | As I16 | As I16 | Tier `small` and the policy waives the merge gate | Engine | `merge`, not checked out | `pull_request_checks_completed`, `gate_waived`, `checkout_released` (`awaiting_merge`) if checked out |
| I18 | `merge` | `pull_request_merged` (observation) | Merge gate `approved` or waived | Person or engine | `done` | `pull_request_merged`, `issue_done`, task to `done` and the transitions of R43 |
| I19 | `merge` | `pull_request_review_submitted` (observation), changes requested, at the recorded head | Reviewer is a recorded approver | Person | `build`, pull request kept | `pull_request_review_submitted`, `gate_resolved` (`rejected`) |
| I20 | `pull_request`, pull request recorded | `pull_request_checks_completed` with a required check failed at the recorded head | Limits of R27 not reached | Engine | `build`, attempt plus one, same branch | `pull_request_checks_completed`, `attempt_failed` |
| I21 | `pull_request`, dispatch open, pull request recorded | `pull_request_synchronized` (observation) at the branch of the issue | New head | Engine | Same state, head recorded, dispatch closed | `pull_request_synchronized`, `step_completed` |
| I22 | `pull_request` without dispatch, or `merge` | `pull_request_synchronized` | New head | Any (fact) | `pull_request`, head recorded, checks cleared | `pull_request_synchronized`, `gate_withdrawn` (`head_changed`) for the merge gate |
| I23 | `N` with a pull request | `pull_request_checks_completed` or `pull_request_review_submitted` at another head | None | Any (fact) | Unchanged | `input_ignored` (`head_stale`) |
| I24 | `N` | `pull_request_merged` of the closing pull request | Merge gate neither `approved` nor waived | Any (fact) | `done` | `pull_request_merged`, `issue_done`, `gate_bypassed`, `gate_withdrawn` for pending gates, `checkout_released` (`merged`) if checked out, `reconciliation_required`, task to `done` |
| I25 | `N` with a pull request | `pull_request_closed` (observation) | None | Any (fact) | Same state, pull request cleared, hold `pull_request_closed` | `pull_request_closed`, `gate_withdrawn` (`pull_request_closed`), `checkout_released` (`pull_request_closed`) if checked out, `issue_blocked` |
| I26 | `N`, held with `pull_request_closed` | `pull_request_reopened` (observation) of the cleared number | None | Any (fact) | `pull_request`, pull request restored, hold removed | `pull_request_reopened`, `issue_unblocked` |
| I27 | An executor step with a dispatch | `step_result`, outcome `question_raised` | Dispatch current; question valid: context, options with cost, recommendation (SPEC-0007 R1) | Agent | Same state, hold `question` | `step_completed` (`question_raised`), `gate_opened` (kind `question`), `issue_blocked` |
| I28 | Any state with a dispatch | `step_result` invalid or failing a guard, or `executor_failure` (result) | Retries below `limits.executor_retries` | Engine | Same state, dispatch closed, retries plus one | `result_rejected` or `step_retried` |
| I29 | As I28 | As I28 | Retries at `limits.executor_retries` | Engine | Same state, dispatch closed, hold `executor_failure` | `result_rejected` when the output is invalid, `step_failed`, `issue_blocked` |
| I30 | `N`, checked out | `session_end_requested` (command) with a hand-over | Sender is the holder of the session; hand-over valid (SPEC-0001 R22) | Agent | Same state, not checked out | `handover_recorded`, `session_ended` (`handed_over`), `checkout_released` (`handed_over`) |
| I31 | `N`, checked out | Due: the lease of the session ended (`due_events`) | `now` not earlier than the end of the lease | Engine | Same state, not checked out, dispatch closed | `checkout_released` (`lease_expired`), `session_ended` (`abandoned`), task to `approved` |
| I32 | `N`, held | Input of the table Holds for a reason of the issue | As the table says | Person or engine | Hold removed; state of the table Holds when it was the last hold | `issue_unblocked` when no hold remains |
| I33 | `N` | `stop_requested` (observation) | Actor is an approver | Person | Same state, hold `stopped_by_person` | `issue_blocked` |
| I34 | `N` | `status_change_requested`: task to `superseded` (observation), or task expanded (SPEC-0013 R3) | SPEC-0001 transition allowed; actor is an approver, or the expansion was approved through a `roadmap_change` gate | Person | `superseded` | `issue_superseded`, `checkout_released` (`superseded`) if checked out, `gate_withdrawn` |
| I35 | `N` | `issue_closed` (observation) not caused by the closing pull request | None | Any (fact) | Same state, hold `reconciliation_required` | `reconciliation_required`, `issue_blocked` |
| I36 | `done`, `superseded` | Any observation of the issue not consistent with it, `issue_reopened` included | None | Any (fact) | Unchanged | `reconciliation_required` |
| I37 | `N` | `tracker_block_invalid` (observation) naming the issue | None | Engine | Same state, hold `question` | `gate_opened` (question `invalid_tracker_block`), `issue_blocked` |

Notes on the table:

- Every result that carries an executor call record records `executor_call_recorded` first,
  whatever its outcome, refused stale results included (R21): the cost of a step is never lost
  (constitution VI).
- Engine steps (`claim`, `verify`, `pull_request`) are dispatched like executor steps (I3) so
  that every result names its dispatch; they count no turn.
- After a release (I30, I31) the issue keeps its state, so that a new claim (I2) resumes where
  the work stopped and the brief starts from the resume pack (SPEC-0003 R44).
- An issue at `merge`, or at `pull_request` after its pull request is opened and before a check
  fails, needs a person or the tracker, not an agent: it is not ready (R19). An issue at `merge`
  is not checked out (R35) and still occupies its lane.
- I11 and I20 differ on the branch. A failed verification before the pull request ends the
  attempt; the runtime discards the branch and worktree (SPEC-0006 R8). A failed required check
  after the pull request is opened keeps the branch, so that the pull request stays open; the
  next `pull_request` step pushes a new head (I21).
- A late hand-over from the last holder of an issue is accepted after the lease ended while no
  other session has claimed the issue (SPEC-0003 R12); it records `handover_recorded` only.

### Holds

A hold is the `blocked` state of the conventions page: the issue keeps its state and cannot
progress until every hold is removed (R30, R31). Holds survive a release: a held issue is not
ready, whoever holds its checkout.

| Reason | Entered by | Removed by | Resumes at |
| --- | --- | --- | --- |
| `question` | I12, I27, I37 | `answer` on its question gate by an approver (G4); for `invalid_tracker_block`, also a later valid `plan_observed` of the item, which withdraws the gate | The same state; for `repeated_failure`: answer `retry` sets `attempt` to 1, resets `failures` and resumes at `build`; answer `stop` replaces the hold with `stopped_by_person` |
| `turn_limit` | I4 | `limits_set` with `limits.turns_per_issue` above `turns` (R28) | The same state |
| `executor_failure` | I29 | `unblock_requested` by an approver | The same state, retries reset |
| `pull_request_closed` | I25 | `pull_request_reopened` (I26); or `unblock_requested` by an approver | `pull_request` after a reopen; `build` after an unblock |
| `reconciliation_required` | I35, and R32 for an inconsistent observation | `unblock_requested` by an approver | The same state |
| `stopped_by_person` | I33, or answer `stop` | `unblock_requested` by an approver | The same state |

Questions (`question_category`):

| Category | Raised by | Answer | Effect of the answer |
| --- | --- | --- | --- |
| `decision`, `irreversible_action`, `security_sensitive_action`, `outside_worktree`, `broken_plan`, `methodology_choice` | Agent, with context, options with cost and a recommendation (SPEC-0007 R1) | Free text; it may name an option | None on the state; the answer reaches the next brief |
| `repeated_failure` | Engine (R27) | One of the fixed options `retry` and `stop` | As the table Holds says |
| `invalid_tracker_block` | Engine, on a structured block of the tracker that does not validate (SPEC-0005 R3), for the issue it names, or for the project when it names no issue | Free text | None on the state; the gate is also withdrawn by a later valid `plan_observed` of the item |

The engine knows the effect of an answer only for `repeated_failure`, which is why only that
category has fixed options.

The tracker adapter spells the state as labels (SPEC-0005): `in-progress` while a checkout
exists, `blocked` while a hold exists, `needs-decision` while a question is pending. The core
never reads a label.

### Ready work and lanes

An issue is ready when every condition holds (R19). Each condition has the code reported when
it fails.

| Condition | Code |
| --- | --- |
| The issue is a planned task issue in state `open`, from `claim` to `review`, or `pull_request` before its pull request is opened or, after a failed check, before its new head is observed | `awaiting_external` for `merge` and for `pull_request` awaiting checks; `transition_not_allowed` for `done` and `superseded` |
| The task is `approved` or `implementing` | `task_not_approved` |
| Its epic and milestone are `approved` or `implementing` | `plan_not_approved` |
| Every cited specification is `approved`, `implementing` or `done` | `specification_not_approved` |
| Every task in `depends_on` is `done` | `dependency_not_done` |
| No hold | `issue_on_hold` |
| No checkout | `checkout_held` |
| No other issue of its lane is in a state from `claim` to `merge` | `lane_busy` |
| The issue belongs to a lane | `not_in_lane` |
| No budget scope of the issue is exhausted | `budget_exhausted` |

Lanes. While a sprint is active, the lanes are those of the sprint in the latest
`plan_observed`, whose paths are disjoint (SPEC-0006 R4, ADR 0013), and an issue outside every
lane is not ready. Without an active sprint, each area of the recorded policy is a lane with the
paths of the area, and the issues of a lane are the issues whose task names that area; a task
without an area is not in a lane. `policy_set` is refused with `area_paths_overlap` when two
areas share a path. A lane runs one issue at a time: it is the unit of parallelism, and two
issues of one lane may change the same files. An issue occupies its lane from its first claim
until it is `done` or `superseded`, including while it waits at `merge` (R35); an issue that was
released keeps its lane and is the one claimed next.

Order. `due_events` returns, for every session whose lease ended, by session identifier, its
`checkout_released` events by issue number, each followed by the task transition of I31, then
its `session_ended`. `next_steps` lists
actions in this order, and stops when `free_workers` actions that need a worker are listed:

1. `run_step` for checked-out issues with no dispatch and no hold, by issue number; their
   workers are already counted as busy by the runtime, so they need no free worker.
2. `merge` for issues where R39 holds, by issue number; it needs no worker.
3. `run_council`, `generate_plan` and `reconcile` where they apply, by subject identifier.
4. `claim` for the first ready issue of each lane without an issue from `claim` to `merge`:
   lanes in the order of the sprint, or of the areas of the policy without a sprint, issues in
   the order of the lane, then by issue number.

Issues that are ready but get no `claim` are listed in `ready` in the same order.

### Limits and budgets

| Limit | Key (ADR 0010) | Default | Counts | On reaching it |
| --- | --- | --- | --- | --- |
| Turns per issue | `limits.turns_per_issue` | 20 | Executor steps started (I3) | Hold `turn_limit` (I4, R28) |
| Attempts per issue | `limits.attempts` | 3 | The attempt number; a failed verification or required check ends an attempt | Question `repeated_failure` when attempt `limits.attempts` fails (I12, R27) |
| Same failure | `limits.same_failure` | 2 | Consecutive attempts on which one check fails | Question `repeated_failure` (I12, R27) |
| Executor retries per step | `limits.executor_retries` | 2 | Executor failures and rejected results of the open step | `step_failed`, hold `executor_failure` (I29, R23) |
| Lease | `limits.lease_minutes` | 120 | Minutes since the latest event of the session | Release (I31, R22) |
| Budget of a phase | `budgets.intake`, `budgets.council`, `budgets.planning` | None | Tokens of the executor calls of project steps in that phase | `budget_exhausted` (R29) |
| Budget of a sprint | `budgets.sprint` | None | Tokens of the executor calls of issues of the active sprint | `budget_exhausted` (R29) |
| Budget of a milestone | `budgets.milestone` | None | Tokens of the executor calls of issues of the milestone, in phase `delivery` | `budget_exhausted` (R29) |

Defaults are listed in `meta.defaults_applied` (ADR 0009 C2) when `wayloop init` writes them, and
recorded by the first `limits_set`; from then on the recorded value applies (R7). Budgets have no
default: every phase has a token budget (constitution VI), so `wayloop init` writes the values the
person gives and refuses a missing one. Tokens are the
sum of `tokens_in` and `tokens_out` of `executor_call_recorded` (SPEC-0003). The scope of an
issue step is its milestone and, when the issue is in the active sprint, the sprint; the scope of
a project step is the phase. Usage and rate limits of a provider are not failures: the runtime
waits or asks (SPEC-0008 R16) and the dispatch stays open.

### Risk tier

The tier of an issue is the highest of the tier of its area and the tiers of the paths of the
latest `branch_changes_observed` at the recorded head of its branch, as the recorded policy maps
them (SPEC-0021). Before any change is observed, the tier is the tier of the area. Tiers are
`small`, `substantial` and `contract`, as in the table of AGENTS.md. The engine computes the tier
at every decision that uses it (R25); it is never reported by an agent. The tier decides which
steps are skipped (R25) and whether the merge gate is waived (R17). A tier that rises after
`counter_review` does not repeat it; it raises the review required at `review`. A merge waiver
needs a `branch_changes_observed` at the recorded head: without one, the merge gate is opened.

### Revisions

A command carries `expected_revision`, the hash of the latest ledger event the sender read
(ADR 0009 C6, SPEC-0004 R12). A revision is stale when it is not the hash of an event of the
chain, or when an event recorded after it touched the subject of the command:

- for a command on an issue, an event whose header names that issue;
- for a command on the project (`run_start_requested`, `project_step_requested`,
  `specification_submitted`), an event that changes the phase, a gate or the plan
  (`phase_changed`, `gate_opened`, `gate_resolved`, `gate_withdrawn`, `gate_waived`,
  `plan_observed`, `artifact_status_changed`, `limits_set`, `policy_set`).

A command whose `input_hash` is already recorded in its session is a retry: it is answered from
the recorded events before its revision is evaluated (SPEC-0003 R52). A stale command is refused
with the current revision in `expected` and nothing is recorded (R41); the sender reads the state again and decides again. Staleness and every guard are
evaluated by the writer under its lock on the latest state, so that two commands that both read
the same revision cannot both pass a guard that only one of them may pass. ADR 0009 C6 is
amended to this scoped rule.

Results use their dispatch instead of a revision (R21): a worker in one lane must not be refused
because another lane recorded an event. Observations carry no revision: they report facts that
happened, which the writer applies in order.

### Events

The ledger records only events of the catalogue of SPEC-0003, which lists every kind with the
specification that owns its payload. This section lists the inputs, which are not recorded as
such, and the payloads of the kinds this specification owns. Every name and enum value is
`snake_case`; consumers of outputs accept enum values they do not know (ADR 0009 C5).

Inputs:

| Input | Class | Sent by | Used by |
| --- | --- | --- | --- |
| `run_start_requested` | Command | Runtime: account, mode, effective configuration | R8, R9 |
| `session_start_requested` | Command | Runtime or agent: holder, executor, mode | `session_started` (SPEC-0003) |
| `claim_requested` | Command | Runtime or agent | I1, I2 |
| `step_requested` | Command | Runtime or agent, the holder | I3, I4 |
| `project_step_requested` | Command | Runtime: step `council`, `plan` or `reconcile`, subject | P3, P12 |
| `lease_renewal_requested` | Command | Runtime of the holder, every quarter of the lease | R22 |
| `session_end_requested` | Command | Agent, the holder, with its hand-over input (SPEC-0001) | I30 |
| `specification_submitted` | Command | Agent: the normalised specification (SPEC-0001 R12) | Gate `specification` |
| `step_result` | Result | Runtime: validated executor output, outcome `completed`, `changes_requested` or `question_raised`, with the executor call record | I5 to I9, I13, I14, I27 to I29 |
| `verification_result` | Result | Runtime: check names and outcomes, the failure note | I10 to I12 |
| `executor_failure` | Result | Runtime: `error` or `timeout`, with the executor call record when one exists | I28, I29 |
| `council_report`, `plan_proposal`, `roadmap_change_proposal` | Result | Runtime, for a project dispatch | P4, P6, P12 |
| `plan_observed` | Observation | Plan adapter: items and active sprint | Readiness, lanes, phase |
| `branch_changes_observed` | Observation | Git adapter: branch, head, changed paths | R25 |
| `brief_submitted` | Observation | Tracker: a brief submitted on its gate issue | P1 |
| `approval` | Observation | Tracker: person, gate, decision `approve` or `reject` | G2, G3, P2, P5, P7, P8 |
| `answer` | Observation | Tracker: person, question gate, text, option when one is named | G4, I32 |
| `unblock_requested`, `stop_requested` | Observation | Tracker: person, issue or brief | I32, I33, P9 |
| `status_change_requested` | Observation | Tracker: person, a person transition of SPEC-0001 | P10, P11, I34 |
| `pull_request_opened`, `pull_request_synchronized`, `pull_request_checks_completed`, `pull_request_review_submitted`, `pull_request_merged`, `pull_request_closed`, `pull_request_reopened` | Observation | Tracker (SPEC-0003 R9) | I15 to I26 |
| `issue_closed`, `issue_reopened` | Observation | Tracker | I35, I36 |
| `tracker_block_invalid` | Observation | Tracker adapter (SPEC-0005 R3) | I37 |

Payloads of the kinds owned by this specification, version 1. The header fields (`issue`,
`session`, `actor` and the cause of R3) are those of SPEC-0003 and are not repeated. A gate is
cited by the hash of its `gate_opened` event; a dispatch by the hash of its `step_started`
event.

| Kind | Fields |
| --- | --- |
| `run_started` | `account` (tracker handle of the runtime), `engine_version`, `mode` (`in_session` or `unattended`), `configuration_sha256` (of the effective configuration, canonical JSON) |
| `limits_set` | `approvers` (handles, sorted by code point, at least one), `limits` (`{turns_per_issue, attempts, same_failure, executor_retries, lease_minutes}`), `budgets` (`{intake, council, planning, sprint, milestone}`), `ledger_budgets` (`{handover, session, issue, milestone}`, SPEC-0003), `approved_by` (handle), `gate` (optional, the configuration gate) |
| `policy_set` | `policy_revision` (SHA-256 of the canonical policy), `areas` (list of `{name, paths, tier}`), `path_tiers` (list of `{pattern, tier}`), `skips` (list of `{tier, steps}`), `merge_waived_tiers` (list of tiers), `required_checks` (names, sorted), `approved_by`, `gate` (optional) |
| `plan_observed` | `plan_sha256` (of the whole observed plan), `part`, `parts`, `items` (list of `{id, type, status, issue, epic, milestone, story, specifications, depends_on, area}`, optional fields left out), `sprint` (optional `{id, lanes: [{name, paths, issues}]}`); a plan larger than the line limit is split into parts |
| `branch_changes_observed` | `branch`, `head_sha`, `paths` (relative, NFC, sorted by code point, without duplicates) |
| `phase_changed` | `from`, `to` |
| `gate_opened` | `kind`, `subject` (artifact identifier, issue or `configuration`), `question` (for kind `question`: `{category, text, options, recommendation}`, `options` empty for free-text answers), `proposal` (optional `{file, sha256}` of the report, plan, change or configuration) |
| `gate_resolved` | `gate`, `resolution`, `by` (handle) |
| `gate_withdrawn` | `gate`, `reason` (`subject_superseded`, `head_changed`, `pull_request_closed`, `bypassed`, `block_valid`) |
| `gate_waived` | `kind` (`merge`), `subject` (issue), `tier`, `policy_revision`, `policy_approved_by` |
| `gate_bypassed` | `gate` (optional), `merge_commit` |
| `approval_recorded` | `gate`, `decision` (`approve` or `reject`), `by` |
| `question_answered` | `gate`, `text`, `option` (optional), `by` |
| `input_ignored` | `input` (input name), `code`, `pointer` (optional) |
| `step_started` | `step`, `subject` (for a project step, the brief or report), `executor` (optional), `turn` (boolean), `attempt` |
| `step_completed` | `step`, `outcome` (`completed`, `changes_requested`, `question_raised`), `skipped` (steps skipped after it), `output` (optional `{file, sha256}`) |
| `step_retried` | `step`, `error` (`error` or `timeout`), `retries` |
| `step_failed` | `step`, `cause` (`result_rejected`, `error` or `timeout`), `retries` |
| `attempt_failed` | `attempt`, `source` (`verification` or `pull_request_checks`), `failing_checks` (sorted), `note` (`{tried, failed, first_error, next}`) |
| `issue_blocked` | `reason`, `gate` (optional) |
| `issue_unblocked` | `removed` (reason), `resumed_at` (optional, issue state), `subject` (optional, the brief of P9) |
| `issue_done` | `pull_request`, `merge_commit` |
| `issue_superseded` | `superseded_by` (task identifiers) |
| `artifact_status_changed` | `artifact`, `from`, `to`, `by` (optional, the person) |
| `budget_exhausted`, `budget_restored` | `scope` (`{kind, id}`, kind `phase`, `sprint` or `milestone`), `budget`, `tokens` |
| `reconciliation_required` | `subject`, `code` (`issue_closed_without_merge`, `issue_reopened`, `merged_without_approval`, `status_conflict`, `observation_inconsistent`), `detail` (optional) |

`checkout_acquired`, `lease_renewed`, `checkout_released`, `session_started`, `session_ended`,
`handover_recorded`, `executor_call_recorded`, `result_rejected` and the pull request events are
owned by SPEC-0003; this machine emits them with the fields that specification defines.

### Artifact transitions taken by the engine

The engine transitions of SPEC-0001 and their triggers (R43). Person transitions arrive as
`approval` or `status_change_requested` and pass G2 or the approver guard.

| Artifact | Transition | Trigger |
| --- | --- | --- |
| Task | `approved` to `implementing` | I1, I2 |
| Task | `implementing` to `approved` | I31 (lease expired) |
| Task | `implementing` to `done` | I18, I24 |
| Milestone, epic | `approved` to `implementing` | First claim of a task of it |
| Story | `approved` to `implementing` | First claim of a task that names it |
| Specification | `approved` to `implementing` | First claim of a task that cites it |
| Story, epic | `implementing` to `done` | A task of it reaches `done` or `superseded` and every task of it is `done` or `superseded`, at least one `done` |
| Gate issue | `draft` to `done`; `draft` to `superseded` | G2 on its gate; G5 on its gate |
| Change delta | `approved` to `implementing`; `implementing` to `done` | Owned by SPEC-0009 (R3), not taken by this machine |

### Stop reasons

`stop_reason` (R42): `approval_required` (a pending gate other than a question), each value of
`question_category`, `turn_limit`, `executor_failure`, `pull_request_closed`,
`reconciliation_required`, `stopped_by_person`, `budget_exhausted`, `rejected_awaiting_input`,
`engine_account_is_approver`, `ledger_broken`. Each stop names its gate, question, issue or
scope. `wayloop next` and `wayloop status` return them (SPEC-0004).

### Errors

Refusals use the report of ADR 0009 C3. Pointers follow RFC 6901 from the root of the input
sent; the field that names the subject (`/issue`, `/dispatch`, `/expected_revision`) carries the
error. A new meaning gets a new code (ADR 0009 C8).

| Requirement | Code | Reason | `expected` |
| --- | --- | --- | --- |
| R4 | `input_class` | `missing` | The field the class requires |
| R8 | `approvers_empty` | `missing` | None |
| R18 | `transition_not_allowed` | `state` | The states from which the transition is allowed |
| R18 | `step_mismatch` | `state` | The current state |
| R19 | `task_not_approved`, `plan_not_approved`, `specification_not_approved` | `state` | The accepted statuses |
| R19 | `dependency_not_done` | `state` | `done` |
| R19 | `issue_on_hold` | `state` | None |
| R19 | `checkout_held`, `lane_busy` | `conflict` | None |
| R19 | `not_in_lane` | `missing` | None |
| R19 | `awaiting_external` | `state` | None |
| R19 | `area_paths_overlap` | `conflict` | None; the overlapping areas in `pointers` |
| R19, R29 | `budget_exhausted` | `range` | None |
| R21 | `dispatch_stale` | `stale` | The open dispatch, when one exists |
| I3, I30 | `checkout_not_held` | `state` | None; the holder in `message` |
| R14 | `actor_not_authorised` | `requires` | None; carried by `input_ignored` |
| R15 | `question_unanswered`, `checks_not_green`, `plan_changed` | `requires` | None; carried by `input_ignored` |
| R23, R24 | `checklist_incomplete`, `objection_unruled`, `finding_unverified` | `missing` | The missing requirements, objections or findings; carried by `result_rejected` |
| G4 | `option_unknown` | `enum` | `retry`, `stop`; carried by `input_ignored` |
| R37 | `head_stale` | `stale` | The recorded head; carried by `input_ignored` |
| Inputs | `no_effect` | `state` | None; carried by `input_ignored` |
| R41 | `revision_stale` | `stale` | The current revision |

### Examples

`examples/` holds scenarios used by AC43: a recorded configuration and an initial state, then
each input with the time `at` given by the writer and the expected issue state, holds, refusal
and recorded events. Revisions are symbolic: `rev-N` is the hash of the event with `seq` N. They
name inputs, events and fields of this specification; payload shapes of the kinds of SPEC-0003
come from it, so the files are traces, not fixtures of an event schema.

- `issue-cycle.happy-path.yaml`: claim to `done` for an issue of tier `substantial`.
- `issue-cycle.repeated-failure.yaml`: two failures of one check, the question, the answer `retry`.
- `issue-cycle.release-and-stale.yaml`: a scoped revision accepted, a stale command, a lease
  expiry, a stale result whose cost is kept, a new claim that resumes.
- `issue-cycle.small-tier-waiver.yaml`: an issue of tier `small` merged under the policy waiver.

## Interfaces
- Core: `decide`, `apply`, `due_events` and `next_steps` of section The four functions, in `crates/wayloop-core`; enums `issue_state`, `gate_kind`, `gate_state`, `resolution`, `hold_reason`, `question_category`, `stop_reason`, `action_kind`, `phase`, `tier`. All new.
- `wayloop next` returns `actions`, `ready` and `stops`; `wayloop status` returns the phase, the gates, and the state, holds and stop reasons of every issue (SPEC-0004). Both report `meta.revision`. New output fields: additive.
- Commands that change state (`claim`, step requests, lease renewal, session end, run start) take `expected_revision` (SPEC-0004 R5) and refuse with `revision_stale`.
- Events: the payloads of section Events for the kinds owned here, with schemas `schemas/events/<kind>/v1.json` and the catalogue in SPEC-0003. New.
- Configuration: the keys of section Limits and budgets and the list `approvers`, in the project file of ADR 0010, written by `wayloop init`; changes reach the state only under R8. New.
- Error codes of section Errors. New.

## Dependencies
- SPEC-0001 (Approved): statuses, allowed transitions, hand-over `step` values.
- SPEC-0003: the event catalogue, the envelope of events, the payloads of checkout, session, hand-over, cost, rejected results and pull request events, and `due_events` timing.
- ADR 0029 (Proposed) amends constitution IV for the merge waiver of the tier `small` (R17).
- ADR 0009 C6 is amended to the scoped staleness rule of section Revisions (R41).
- #228 aligns the `IssueStep` identifiers with the `snake_case` values used here.
- ADR 0011, ADR 0012 and ADR 0013 are Proposed; executors, gates on the tracker and lanes depend on them.
- Follow-up changes in other specifications are listed in the report of the pull request that proposes this specification for approval.

## Plan and tasks
`plan.md` and `tasks.md` are written after the maintainer approves this specification
(`specs/README.md`, Agent workflow). Tasks #48 (transition table and guards), #49 (next step and
ready work) and #58 (dry run on local fixtures) implement it.

## Open questions
- [x] **Q1. Which steps does the tier `small` skip?** Decided by the maintainer on 2026-11-17: (b), `small` skips both `counter_review` and `review`, set per tier in the policy, with the tier computed by the engine from the area and the observed diff (R25, section Risk tier).
- [x] **Q2. May the loop merge an issue of tier `small` without a person at the gate?** Decided by the maintainer on 2026-11-17: the project policy may waive the merge gate for `small`; ADR 0029 amends constitution IV. The engine records `gate_waived` with the policy revision and the person who approved it, never `gate_resolved`; a policy change that widens `small` or adds a waiver passes the `configuration` gate (R8, R17).
- [x] **Q3. When is an expected revision stale?** Decided by the maintainer on 2026-11-17: (b), scoped to the issue of the command, or to the phase, gates and plan for a project command; guards are evaluated again under the writer lock; ADR 0009 C6 is amended (R41, section Revisions).
- [x] **Q4. Where does a new claim resume after a release?** Decided by the maintainer on 2026-11-17: (a), at the state reached.
- [x] **Q5. Which issues are ready without an active sprint?** Decided by the maintainer on 2026-11-17: (b), each area of the policy is a lane; a policy with overlapping area paths is refused, and an issue without area is not in a lane (`not_in_lane`).
- [x] **Q6. Default limits.** Decided by the maintainer on 2026-11-17: (a), turns 20, attempts 3, same failure 2, executor retries 2, lease 120 minutes under the single key `limits.lease_minutes`, recorded in `limits_set`, with a heartbeat every quarter of the lease (R22).
- [x] **Q7. What happens when the pull request of an issue is closed without merge?** Decided by the maintainer on 2026-11-17: (a), hold with `pull_request_closed`; the pull request is cleared, and `pull_request_reopened` restores it (R36).
- [x] **Q8. What is a budget scope?** Decided by the maintainer on 2026-11-17: phase budgets for `intake`, `council` and `planning`, a budget per sprint, and a budget per milestone in `delivery` (section Limits and budgets).
- [x] **Q9. Who is an authorised approver?** Decided by the maintainer on 2026-11-17: an explicit list written by `wayloop init`, run by a person. Adding an approver or raising a limit or a budget opens a gate of kind `configuration` that only an already recorded approver resolves; decreases apply at once. The runtime reports its account in `run_started`, and the engine proposes nothing while that account is an approver (R8, R9, R44).
