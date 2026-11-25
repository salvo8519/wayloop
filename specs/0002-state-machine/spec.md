# SPEC-0002: State machine of the project and of the issue cycle

| | |
| --- | --- |
| Status | Draft |
| Milestone | M1 · Engine core |
| Owner | @salvo8519 |
| Related | #11, #48, #49, #58, #161, #162, #228, #309, #347; ADR 0002, 0003, 0006, 0007, 0008, 0009, 0010, 0011, 0012, 0013, 0026, 0028, 0029, 0030; SPEC-0001, SPEC-0003, SPEC-0004, SPEC-0005, SPEC-0006, SPEC-0007, SPEC-0008, SPEC-0009, SPEC-0013, SPEC-0021 |
| Commit scope | state |
| History | Reopened on 2026-11-25 for the counter-review; approval pending |

## Problem
The engine decides every step of the loop. Without an explicit state machine the decision moves into prompts, where it cannot be tested, replayed or audited. Maintainers need the loop to stop exactly at the gates they own; agents need the engine to tell them the next allowed step; parallel workers need a single, deterministic answer to the question "what is ready".

## Scope
**In scope:** The state of a project (its phase), of a gate, of a session and of the issue cycle of a planned task. The pure functions of the core: `decide`, `apply`, `due_events`, `next_steps` and `genesis`. Every transition with its input, guard, actor and recorded events, as one table kept as data in the core. The approval states the loop cannot pass, and the merge waiver of ADR 0029. Holds (the `blocked` state): how one is entered and left. The configuration that the state depends on (approvers, limits, budgets, policy, plan) and how it enters the state. Limits on turns, attempts, executor retries, leases and token budgets, and the rule for repeated failure. Ready work for parallel lanes (ADR 0006, ADR 0013). Stale revisions (ADR 0009 C6, as amended) and stale dispatches. The inputs the machine decides and the payloads of the event kinds it owns; the catalogue of every kind is in SPEC-0003. The engine-taken transitions of artifact statuses listed in SPEC-0001. The trace format of the scenarios of `examples/` and their runner.

**Out of scope:** The event envelope, the hash chain, persistence, replay mechanics and the payloads of the kinds owned by SPEC-0003. Execution of steps, worktrees and the worker pool (SPEC-0006). Executors, briefs and result schemas of each step (SPEC-0008). The tracker mapping of labels, approvals, answers and person requests, and the normalisation of handles (SPEC-0005, SPEC-0007). The content of gates and questions (SPEC-0007, SPEC-0012). Generation of plan items (SPEC-0013). Artifact fields and the allowed status transitions themselves (SPEC-0001). Transitions of change deltas (SPEC-0009). Policy rules as data and their evaluation on a pull request (SPEC-0021).

## User stories
- As a maintainer, I want the loop to stop exactly at the gates I own, so that nothing irreversible happens without me.
- As a maintainer, I want every stop to carry its reason, so that I know in one read what the loop needs from me.
- As a maintainer, I want only the people I listed to approve, and every widening of limits or policy to pass through them, so that an agent cannot give itself more room.
- As an agent, I want the engine to tell me the next allowed step, so that I never decide the process myself.
- As a runtime, I want the list of ready work in a fixed order, so that parallel workers never take the same issue or the same lane.

## Requirements (EARS)
Requirement numbers are identities, not an order (SPEC-0001 R20). R8 to R11 and AC4 to AC7 were
merged on `main` by #309 and are cited by SPEC-0006 R11, so they keep their numbers; their
risk levels high, medium and low are written with the names of the tiers (Q10). The
requirements and criteria of this draft that held the same numbers took the next free ones:

| Earlier draft of this pull request | Now |
| --- | --- |
| R8, R9, R10, R11 (configuration gate, account of the runtime, project phase, council and plan) | R45, R46, R47, R48 |
| AC4, AC5, AC6, AC7 (one-pass refusal, input class, recorded limits, replay and causes) | AC44, AC45, AC46, AC47 |

The amendments of 2026-11-25 answer the counter-review of SPEC-0002 and SPEC-0003; each one
names its item. R54 to R59 and AC54 to AC81 are new. The amendments of the second
counter-review, of the same day, name their item as "second counter-review Bn", "Sn" or "Nn";
they add the rows I51 and S6 and the function `genesis`, and no requirement number.

- **R1**: THE ENGINE SHALL decide every input with a function `decide(state, input, at)` that returns either the events to record, each finished with its `seq`, `prev_hash`, `recorded_at` and `hash` (SPEC-0003 R58) from the last event `(seq, hash)` that the state holds, or a refusal, and SHALL change the state only with a function `apply(state, event)` of one recorded event; neither function SHALL read a clock, a file, the network, the environment or a random source (decided by the maintainer on 2026-11-25, counter-review item 1).
- **R2**: THE ENGINE SHALL rebuild the state only by applying the recorded events in ledger order to the initial state, and SHALL never call `decide` during a replay.
- **R3**: THE ENGINE SHALL record in every event it emits for an input the cause of that input: `input_hash` for a command, `dispatch` for a result, `external_id` for an observation.
- **R4**: THE ENGINE SHALL accept each input only in the input class given by the table Inputs, refuse a result without `dispatch` or an observation without `external_id` with the code `input_class` and the reason `missing`, and refuse an input that carries a field its class forbids with the code `input_class` and the reason `unknown_field` (decided by the maintainer on 2026-11-25, counter-review item 64); a command without `expected_revision` is refused by the input schema of its command at the entry point (SPEC-0004 R14, code `command_input`), except the commands that section Revisions lists without one.
- **R5**: THE ENGINE SHALL return the events due by time only from a function `due_events(state, now)`, and the next steps (actions, ready work and stop reasons) only from a function `next_steps(state, integrity, free_workers)`, where `integrity` is the value the loader gives (section The functions of the core), both pure, with the outputs in the total order of section Ready work and lanes; `next_steps` SHALL take no time (decided by the maintainer on 2026-11-25, counter-review items 2 and 3).
- **R6**: IF a command fails one or more guards of its transition, THEN THE ENGINE SHALL refuse it with the codes of section Errors and record nothing, evaluating the guards in two levels: the input class and the state checks of the transition (`input_class`, `transition_not_allowed`, `step_mismatch`) stop the evaluation at the first failure; after them, every other guard is evaluated and every unmet one is reported in one pass, in the order of the Guard column. Only the `issue_state` named in the column From is a state check of level 1; a condition on the checkout, such as "checked out", is a guard of level 2 with its own code (`checkout_held`, `checkout_not_held`), so that it is reported in the same pass as the other guards (decided by the maintainer on 2026-11-25, second counter-review B2). A result or an observation that fails a guard follows the column "When it does not fit the state" of section Input classes (decided by the maintainer on 2026-11-25, counter-review items 4 and 6).
- **R7**: THE ENGINE SHALL apply the approvers, limits, budgets and policy recorded by the latest `limits_set` and `policy_set` events, never the configuration file read at the time of a decision.
- **R8**: WHERE the risk tier of an issue is `contract`, THE ENGINE SHALL run the review step as a panel of three verifiers, one per lens (SPEC-0003, Coverage and verification), none of which wrote or counter-reviewed the change, each instructed that a coverage unit is `uncovered` unless it can cite the code and the test that cover it, and SHALL allow one repetition of the panel per review step: one more dispatch of one lens when its verifier fails or its result is rejected, so that the panel costs at most four invocations (one repetition per review step: decided by the maintainer on 2026-11-25, counter-review item 43).
- **R9**: THE ENGINE SHALL compute the outcome of a panel from the votes alone: `covered` only when every lens votes `covered`; `needs_validation` when no lens votes `uncovered` and at least one votes `needs_validation`; `uncovered` otherwise. No model computes or reports the outcome.
- **R10**: IF a verifier returns a vote without a citation for each claim, THEN THE ENGINE SHALL count it as `uncovered` and record `downgrade` `uncited` in its `coverage_verdict` (SPEC-0003, Payloads).
- **R11**: WHERE the risk tier of an issue is `substantial`, THE ENGINE SHALL run the review step with one verifier on the `proof` lens under R9 and R10; WHERE it is `small`, THE ENGINE SHALL rely on the deterministic checks of the verify step alone, whatever the policy says.
- **R12**: WHEN a gate of kind `council_report` or `roadmap` is rejected, THE ENGINE SHALL move its subject to `superseded`, propose no new `run_council` or `generate_plan` for that brief, and report the stop reason `rejected_awaiting_input` until an approver requests a new run or the brief is superseded.
- **R13**: WHEN a subject reaches one of the approval states of section Gates and approval states, THE ENGINE SHALL open a gate, record `gate_opened`, and propose no action that passes the gate while it is pending.
- **R14**: THE ENGINE SHALL resolve a gate only with an `approval_recorded` or `question_answered` whose actor is a recorded approver; IF the actor is not a recorded approver, THEN THE ENGINE SHALL record `input_ignored` with the code `actor_not_authorised` and keep the gate pending.
- **R15**: IF an approval of a gate arrives while a subject guard of the gate does not hold, THEN THE ENGINE SHALL record `input_ignored` with the code of the unmet guard and keep the gate pending.
- **R16**: WHEN the subject of a pending gate is superseded, or the pull request of a merge gate gets a new head, fails a required check or is closed, THE ENGINE SHALL withdraw the gate and record `gate_withdrawn` with the reason (failed check: decided by the maintainer on 2026-11-25, counter-review item 30).
- **R17**: WHERE the recorded policy waives the merge gate for the tier `small`, WHEN an issue of tier `small` reaches `merge`, THE ENGINE SHALL record `gate_waived` with the policy revision and the approver of that revision instead of opening the gate, and SHALL never record the waiver as `gate_resolved`.
- **R18**: THE ENGINE SHALL change the issue-cycle state of a planned task only through a transition of the table Issue transitions; IF a command names a transition that the table does not allow from the current state, THEN THE ENGINE SHALL refuse it with the code `transition_not_allowed`, or `step_mismatch` when the requested step is not the current state; IF a result is of a kind or carries an outcome that the step of its dispatch does not accept, THEN THE ENGINE SHALL record `result_rejected` with the code `result_kind_mismatch` under R23, after the `executor_call_recorded` the result carries (decided by the maintainer on 2026-11-25, counter-review item 5).
- **R19**: THE ENGINE SHALL consider an issue ready only when every condition of the table Readiness holds, and SHALL report, for an issue that is not ready, every unmet condition with its code.
- **R20**: WHEN a claim of a ready issue is accepted, THE ENGINE SHALL record `checkout_acquired` with the holder, session, executor and the branch and worktree names it derives under section Names of a checkout, move an issue in state `open` to `claim`, keep the state of an issue that was claimed before, and record the artifact transitions of section Artifact transitions taken by the engine; the result of the step `claim` only confirms the names (decided by the maintainer on 2026-11-25, counter-review item 28).
- **R21**: WHEN a step of a checked-out issue is requested, THE ENGINE SHALL record `step_started` as the dispatch of the step and count one turn if an executor runs the step; IF a result cites a dispatch that is not the open dispatch of its subject, THEN THE ENGINE SHALL refuse it with the code `dispatch_stale` and record only the `executor_call_recorded` the result carries and, when that call makes the provider tokens of a scope reach its budget, the `budget_exhausted` of R29 (decided by the maintainer on 2026-11-25, counter-review item 58).
- **R22**: WHEN `due_events` finds a session whose lease has ended under the rule of SPEC-0003 R46, THE ENGINE SHALL return the events of SPEC-0003 R11 and R43 for that session, keep the issue-cycle state and holds of each of its issues, close their open dispatches, and move each of their tasks in `implementing` back to `approved` only while `pull_request_step_completed` is false for the issue, that is while no `step_completed` of the step `pull_request` is recorded for it (decided by the maintainer on 2026-11-25, counter-review item 27; the condition decided by the maintainer on 2026-11-25, second counter-review B1, held in that field by second counter-review check N-S2, so that R22 never undoes R35 after a closed pull request clears the recorded one); a session that holds no checkout ends the same way, with `session_ended` `abandoned` only, `limits.lease_minutes` minutes after its latest event that extends a lease (decided by the maintainer on 2026-11-25, second counter-review B12).
- **R23**: IF a result does not validate against the schema of its step or fails a guard of its transition, or an executor fails or times out, THEN THE ENGINE SHALL record `result_rejected` or `step_retried`, close the dispatch and count one retry; WHEN a failure finds the retries of the step at their limit, THE ENGINE SHALL record `step_failed` and hold the issue with the reason `executor_failure`; THE ENGINE SHALL reset the retries and the repetition of the panel at every `step_completed` and when a hold `executor_failure` is removed (decided by the maintainer on 2026-11-25, counter-review item 43).
- **R24**: THE ENGINE SHALL complete the step `counter_review` only when every objection has a ruling or a question, the step `checklist` only when the checklist names every requirement cited by the task, and a dispatch of the step `review` only when it carries one vote of its lens for every coverage unit of the task (R49).
- **R25**: THE ENGINE SHALL compute the risk tier of an issue from the recorded policy, its area and the paths of `changed_paths` at the head that the decision names: `verified_head` for the skip of `review` and the verifiers of `review`, the recorded head (`pull_request.head_sha`) for the merge waiver; only for a decision that names no head, the skip of `counter_review`, the tier is the tier of the area (decided by the maintainer on 2026-11-25, second counter-review S1, limited by second counter-review check N-B1); WHERE that tier skips `counter_review` under the policy, or the tier is `small` (R11), SHALL skip `counter_review` or `review` and record the skip in `step_completed`; THE ENGINE SHALL decide the skip of `review` only while the changed paths at `verified_head` are recorded (decided by the maintainer on 2026-11-25, counter-review item 33), and the merge waiver only while the changed paths at the recorded head are recorded; without them, `review` runs and the merge gate is opened (decided by the maintainer on 2026-11-25, second counter-review check N-B1).
- **R26**: WHEN a verification fails, a required check of the pull request fails, the tally of a review gives a unit `uncovered`, or the merge gate is rejected, and neither limit of R27 is reached, THE ENGINE SHALL record `attempt_failed` with its source, the failure note and the failing checks, increase the attempt number by one, and move the issue to `build` (review and merge rejection: decided by the maintainer on 2026-11-25, counter-review item 44).
- **R27**: WHEN a failure leaves one check failed on at least `limits.same_failure` consecutive attempts, or the failed attempt number is at least `limits.attempts`, THE ENGINE SHALL record `attempt_failed`, open a question of category `repeated_failure` with the options `retry` and `stop`, and hold the issue instead of starting another attempt; a check is counted by its name, whether it ran in `verify` or as a required check, a success of the check resets its count to 0, and a run that does not report the check leaves its count unchanged (decided by the maintainer on 2026-11-25, counter-review items 13 and 45).
- **R28**: WHEN an executor step of an issue is requested and the turns of the issue are at least `limits.turns_per_issue`, THE ENGINE SHALL hold the issue with the reason `turn_limit` instead of starting the step, except a dispatch of `review` covered by the verification reserve (decided by the maintainer on 2026-11-25, counter-review item 55), and except a request of `review` for a task with no coverage unit, which I46 holds with its question instead (final check, decided by the maintainer on 2026-11-25); WHEN a `limits_set` sets the limit above the turns, THE ENGINE SHALL remove that hold.
- **R29**: WHEN an `executor_call_recorded` makes the provider tokens recorded in a budget scope reach its budget, or WHEN a `limits_set` sets a budget at or below the provider tokens recorded in a scope, THE ENGINE SHALL record `budget_exhausted` once, at that event (for a `limits_set`, right after it, with the same cause; decided by the maintainer on 2026-11-25, second counter-review S8), propose no new step in that scope except a dispatch of `review` covered by the verification reserve (section Limits and budgets, Units), and still accept the results of steps already started; WHEN a `limits_set` raises the budget above the recorded provider tokens, THE ENGINE SHALL record `budget_restored`. Each sprint identifier and each milestone identifier has its own budget, and the provider tokens of a call are charged to the scopes of its issue when the call is recorded (decided by the maintainer on 2026-11-25, counter-review items 58 and 59).
- **R30**: WHEN an issue enters a hold, THE ENGINE SHALL set the issue-cycle state given by the row of the table Issue transitions, add to its holds the entry `{reason, gate}`, with no target stored (decided by the maintainer on 2026-11-25, final check 2; criterion: stability and maintainability), record `issue_blocked` with the reason, and propose no new dispatch for the issue while a hold remains; a hold blocks only new dispatches, and the result of a dispatch already open is decided as usual (decided by the maintainer on 2026-11-25, counter-review items 37 and 38).
- **R31**: THE ENGINE SHALL remove a hold only with the input that the table Holds lists for its reason, from a recorded approver where the table says so, and apply the effects on counters of that removal only when the hold is removed; an `unblock_requested` that names a reason removes the hold of that reason, and one that names none removes every hold that the table lets a person remove; WHEN a hold is removed, THE ENGINE SHALL compute its target from the input that removes it and the state the issue is in at that moment, under the table Holds, and move the issue to that target at once, also while other holds remain; a removal without a target, and a removal by a withdrawal with `review_left`, leaves the state that the row of the input sets (decided by the maintainer on 2026-11-25, final check 2; criterion: stability and maintainability); WHEN the last hold of an issue is removed, and only then, THE ENGINE SHALL record `issue_unblocked` with `resumed_at` the state the issue is then in (decided by the maintainer on 2026-11-25, counter-review item 36; the target computed at removal, final check 2), after moving it to `pull_request` while `no_unit_acknowledged` is set (R49; decided by the maintainer on 2026-11-25, second counter-review check N-S2).
- **R32**: THE ENGINE SHALL never refuse an observation for its state: IF an observation is not consistent with the state of a non-terminal issue, THEN THE ENGINE SHALL apply the fact transition of the table Issue transitions when one exists, and otherwise hold the issue with the reason `reconciliation_required`, recording the `reconciliation_code` of the table Reconciliation codes; IF the issue is `done` or `superseded`, THEN THE ENGINE SHALL record `reconciliation_required` and leave the issue unchanged (codes: decided by the maintainer on 2026-11-25, counter-review item 64).
- **R33**: WHEN an observation arrives whose `external_id` is already recorded, THE ENGINE SHALL leave the state unchanged and record nothing, before any other rule is evaluated.
- **R34**: THE ENGINE SHALL move an issue to `done` only on the observation `pull_request_merged` of its closing pull request; WHEN the merge gate was neither approved by a recorded approver nor waived under R17, THE ENGINE SHALL also record `gate_bypassed` and `reconciliation_required`.
- **R35**: WHEN the result of the step `pull_request` closes its dispatch, whether the step opened or updated the pull request, THE ENGINE SHALL release the checkout of the issue with the reason `awaiting_checks`, keep its task in `implementing` from then on, whatever later releases happen, and count the issue as occupying its lane from its first claim until it is `done` or `superseded` (decided by the maintainer on 2026-11-25, counter-review items 27 and 35; trigger and reason decided by the maintainer on 2026-11-25, Q15).
- **R36**: WHEN the pull request of an issue is closed without merge, THE ENGINE SHALL hold the issue with the reason `pull_request_closed`, clear its recorded pull request, withdraw its merge gate and release its checkout; WHEN the same pull request is reopened while that hold remains, THE ENGINE SHALL restore the pull request as it was recorded at the close, with its head, `heads`, `checks` and `head_pending`, remove the hold and move the issue to `pull_request` when it is at `pull_request` or `merge` at that moment, and otherwise leave it in the state it is in (R31; final check, decided by the maintainer on 2026-11-25; the state at removal, final check 2), and at `pull_request` with no open dispatch SHALL then apply the row that the recorded checks select, as I15, I22 and I44 do (decided by the maintainer on 2026-11-25, final check 2; criterion: stability and maintainability); the merge gate opens or is waived only at `verified_head` (R38).
- **R37**: WHEN an open pull request of an issue gets a new head, THE ENGINE SHALL record the head; at `merge`, withdraw the merge gate and return the issue to `pull_request`; at `build`, `verify` or `review`, change nothing else, except at `review` under I43; IF a check result names a head of the pull request that is not yet observed, THEN THE ENGINE SHALL record it and apply its conclusions when that head is observed; IF a check result or a review names a head the pull request had before its recorded head, THEN THE ENGINE SHALL record `input_ignored` with the code `head_stale` and leave the issue unchanged (decided by the maintainer on 2026-11-25, counter-review items 29, 31 and 32).
- **R38**: THE ENGINE SHALL record the conclusion of every required check at the head it ran on, treat `timed_out` as failed, ignore `cancelled`, and open or waive the merge gate only when every required check of the recorded policy has the conclusion `success` at the recorded head and the recorded head equals `verified_head`, sending the issue back to `verify` otherwise (I16, I17, I39; final check, decided by the maintainer on 2026-11-25; moved here from R36 as its one source, final check 2); WHERE the recorded policy lists no required check, THE ENGINE SHALL move the issue to `merge` once its pull request is recorded at `verified_head`; WHILE an issue waits for required checks, THE ENGINE SHALL report the stop reason `awaiting_checks`, with no timeout in the core (decided by the maintainer on 2026-11-25, counter-review item 46).
- **R39**: THE ENGINE SHALL propose the action `merge` for an issue only while its merge gate is approved or waived and every required check has the conclusion `success` at the recorded head.
- **R40**: WHEN the task of an issue is superseded, THE ENGINE SHALL move the issue to `superseded`, close its open dispatch, release its checkout, clear its holds, withdraw its gates, and record `issue_superseded`; a later result of the closed dispatch is refused with `dispatch_stale` under R21 (decided by the maintainer on 2026-11-25, counter-review item 38).
- **R41**: IF a command carries an expected revision that is stale under the rule of section Revisions for the subject of the table Subjects of commands, THEN THE ENGINE SHALL refuse it with the code `revision_stale` and the current revision in `expected`; THE ENGINE SHALL evaluate the staleness and every guard under the lock of the writer, on the latest state.
- **R42**: THE ENGINE SHALL report, for the project and for every issue that cannot progress without a person, a check run or a new budget, its stop reasons from the enum `stop_reason`, with the gate, question, issue, scope or subject each one names (final check, decided by the maintainer on 2026-11-25), and WHILE the `integrity` given to `next_steps` is broken or regressed (SPEC-0003 R4, R49) SHALL propose no action and report only `ledger_broken` (decided by the maintainer on 2026-11-25, counter-review item 3).
- **R43**: THE ENGINE SHALL take the engine transitions of artifact statuses of SPEC-0001 only on the triggers listed in section Artifact transitions taken by the engine, and record each as `artifact_status_changed`, in the order that section gives.
- **R44**: THE ENGINE SHALL take the approvers only from the explicit list recorded by `wayloop init` and by later `limits_set` events, SHALL compare a handle with that list by exact spelling, the adapters giving every handle in lower case (SPEC-0005), SHALL refuse a list that holds one handle twice with the code `approver_duplicate`, and SHALL never treat as an approver a handle that is not listed or the account of the runtime recorded by `run_started` (decided by the maintainer on 2026-11-25, counter-review item 14 and section 3 of the counter-review).
- **R45**: WHEN `run_start_requested` carries an effective configuration that differs from the recorded one, THE ENGINE SHALL record at once the changes that the table Direction of a configuration change lists as tightening, SHALL open one gate of kind `configuration` that holds only the keys and values that loosen, and SHALL withdraw every configuration gate still pending from an earlier run; WHEN a configuration gate is approved, THE ENGINE SHALL apply its keys on top of the configuration recorded at that time (decided by the maintainer on 2026-11-25, counter-review items 10, 11 and 12).
- **R46**: THE ENGINE SHALL record `run_started` with the account of the runtime and, WHILE that account is a recorded approver, propose no action and report the stop reason `engine_account_is_approver`; WHILE no `run_started` is recorded, THE ENGINE SHALL propose no action and report the stop reason `run_not_started` (decided by the maintainer on 2026-11-25, counter-review item 15).
- **R47**: THE ENGINE SHALL derive the phase of the project from the statuses of its artifacts with the function of section Project phase, and record `phase_changed` whenever an event changes the derived phase.
- **R48**: WHILE an open brief has no council report in `draft` or `approved`, or an approved council report has no pending plan proposal, THE ENGINE SHALL propose the action `run_council` or `generate_plan` for that subject within the budget of its step kind counted for that brief (section Limits and budgets), and dispatch it through `project_step_requested` (decided by the maintainer on 2026-11-25, counter-review items 20 and 22).
- **R49**: WHEN the review step of an issue starts, THE ENGINE SHALL dispatch it once per lens that R8 or R11 requires for the tier of the issue, one lens at a time in the order `presence`, `behaviour`, `proof`, refusing a request for another lens with `lens_not_expected`, each to a verifier, identified by its executor and holder, that ran neither `build` nor `counter_review` of the issue and differs from the verifiers of the earlier lenses of the same review step, and only while the changed paths of the branch are recorded at the head that `verify` verified (I3, I10), whatever the order in which they were observed, over the coverage units of the requirements cited by its task; IF the task has no coverage unit, THEN THE ENGINE SHALL hold the issue with a question of category `needs_validation` instead of dispatching, and WHEN an approver answers that question, SHALL move the issue to `pull_request`, the `question_answered` being the recorded acknowledgement that the task has no coverage unit (decided by the maintainer on 2026-11-25, second counter-review S9); WHEN the issue leaves `review` while that question is pending, THE ENGINE SHALL withdraw it with the reason `review_left` and remove its hold (final check, decided by the maintainer on 2026-11-25); THE ENGINE SHALL keep that acknowledgement in the state (`no_unit_acknowledged`) and, WHEN the last hold of the issue is removed while it is kept, SHALL record `step_completed` of `review` and move the issue to `pull_request`, clearing `pull_request_pass_completed` (decided by the maintainer on 2026-11-25), whatever the target of that removal, without asking the question again (decided by the maintainer on 2026-11-25, second counter-review check N-S2); WHEN the last lens has voted, THE ENGINE SHALL tally each unit under R9 and R10 and move the issue to `pull_request` when every unit is `covered`, to `build` when a unit is `uncovered`, and otherwise hold it with a question of category `needs_validation` (SPEC-0009 R10) (verifier identity, lens order and zero units: decided by the maintainer on 2026-11-25, counter-review items 52, 53 and 54).
- **R50**: THE ENGINE SHALL list in `ready` of `next_steps`, after the ready issues, every coverage unit in state `stale` (SPEC-0003 R16), by unit identifier in code point order.
- **R51**: WHEN `pull_request_merged` is observed for any pull request merged into the default branch, whether it closes a planned issue (I18, I24) or not (P13), or WHEN the git adapter observes a push to the default branch (`default_branch_pushed`, P13), THE ENGINE SHALL record `coverage_stale` for every `covered` unit that SPEC-0003 R16 makes stale: a unit with a cited file in the `changed_paths` the observation carries, the paths of the merge or the push relative to its base, unless every verdict of the unit was produced on the merged or pushed head.
- **R52**: IF a policy, given to `wayloop init` or carried by `run_start_requested`, lists in `skips` a step other than `counter_review`, or in `merge_waived_tiers` a tier other than `small` (ADR 0029), THEN THE ENGINE SHALL refuse it with the code `policy_skip_not_allowed` or `policy_waiver_not_allowed` and record nothing.
- **R53**: WHEN `wayloop init` writes the configuration, THE ENGINE SHALL decide it with the pure function `genesis` of section The functions of the core (decided by the maintainer on 2026-11-25, second counter-review S2), write the default of section Limits and budgets for every limit the person does not give, list each default in `meta.defaults_applied`, record the values in the first `limits_set`, refuse a configuration in which a token budget of section Limits and budgets is not given with the code `budget_missing`, and refuse a limit or a budget below its minimum of section Limits and budgets with the code `limit_range` (decided by the maintainer on 2026-11-25, counter-review item 13).
- **R54**: THE ENGINE SHALL hold the transitions of the tables Issue transitions, Session transitions, Gate transitions and Project transitions as one constant table of data in `wayloop-core`, one row per transition with its identifier, source states, input, guards, target and recorded events, from which `decide` takes the row, evaluates its guards and applies its effects; a test SHALL keep that table equal to the tables of this specification (decided by the maintainer on 2026-11-25, counter-review section 4, code pattern 1).
- **R55**: THE ENGINE SHALL evaluate every guard as one named predicate of the core with exactly one code of section Errors, and every code of section Errors with the severity `refusal` or `recorded` SHALL be produced by a predicate or a rule named in its row (decided by the maintainer on 2026-11-25, counter-review section 4, code pattern 4). A condition that chooses between rows of the same source state and input, and never refuses, is a selector: a named predicate without a code, written "Selector:" in the column Guard; the selectors of one source state and input are exclusive and complete (decided by the maintainer on 2026-11-25, second counter-review B6).
- **R56**: THE ENGINE SHALL represent the state so that the data attached to an issue depends on its state (no checkout at `merge`, `done` or `superseded`; a dispatch only in states from `claim` to `pull_request`; at most one dispatch per issue), and SHALL check the invariants of section Invariants after every `apply` in its tests (decided by the maintainer on 2026-11-25, counter-review section 4, code pattern 3).
- **R57**: THE ENGINE SHALL apply every recorded event as a fact: `apply` SHALL never panic and never refuse, on any sequence of events, and only the loader of SPEC-0003 guarantees that each event is applied once, by checking that `seq` follows on (decided by the maintainer on 2026-11-25, counter-review item 8).
- **R58**: THE PROJECT SHALL write each scenario of `examples/` in the trace format of section Examples, validated by its schema, and run it with a runner that binds every symbol (dispatches, revisions, gates, heads) to the value the engine produced and compares every expected field (decided by the maintainer on 2026-11-25, counter-review section 3 on AC43 and section 4, code pattern 9).
- **R59**: THE ENGINE SHALL take every time as a `Timestamp` in whole seconds of UTC, and THE RUNTIME SHALL append the events of `due_events` under the lock before it decides any input that changes state, command, result or observation, recording them at the later of `now` and the `recorded_at` of the last event (decided by the maintainer on 2026-11-25, counter-review item 9).

## Acceptance criteria
- **AC1** (R1, R18, R32, R54, R55): Given the transition table of the core (R54), when every pair of source state and input kind of the tables Issue transitions, Session transitions and Gate transitions is enumerated, then each row has at least one positive case whose next state, holds and recorded events, in the order of the column Recorded, match the row; each guard of each row has at least one negative case that gives its code; for each pair, the selectors of its rows are exclusive and complete: every case selects exactly one row (second counter-review B6); and every enumerated pair absent from the tables is refused with `transition_not_allowed` or `step_mismatch` for a command, recorded as `result_rejected` with `result_kind_mismatch` for a result, and handled as R32 says for an observation. The rows of the table Project transitions join the enumeration with R45 to R48. Verification: property test (exhaustive enumeration of the table) and TDD (section 3 of the counter-review).
- **AC2** (R1, R5): Given states reached by random sequences of inputs decided and applied from the initial state, and a random input, when `decide`, `apply`, `due_events` and `next_steps` run twice, then both outputs are byte-identical in canonical JSON; and the core crate passes `cargo clippy -- -D clippy::disallowed_methods -D clippy::disallowed_types` with a `clippy.toml` that disallows reading the system clock, the environment, the file system and random sources, and the hash maps and sets whose iteration order is random. Verification: property test (section 3 of the counter-review).
- **AC3** (R5, R19): Given a random set of issues, lanes and free workers, when the next steps are computed for the same state with the issues stored in different orders, then the output is identical and ordered as section Ready work and lanes says. Verification: property test.
- **AC4** (R8): Given an issue of tier `contract`, when the review step starts, then three verifiers are dispatched with distinct lenses and none of them is the implementer or the counter-reviewer; given the verifier of one lens that times out, then one more dispatch of that lens is started and a second failure of any lens holds the issue with `executor_failure`, so that at most four dispatches of `review` are started. Verification: TDD.
- **AC5** (R9): Given every combination of three votes, when tallied, then the outcome matches the rule of R9. Verification: property test.
- **AC6** (R10): Given a `covered` vote with no citation, when tallied, then it counts as `uncovered` and its `coverage_verdict` records `downgrade` `uncited`. Verification: TDD.
- **AC7** (R11): Given an issue of tier `substantial`, when the review step starts, then exactly one verifier is dispatched with the `proof` lens; given an issue of tier `small`, then none is, even under a policy that lists no skip. Verification: TDD.
- **AC8** (R45, R44): Given a recorded configuration with approvers `maintainer`, 20 turns and a sprint budget of 100,000 provider tokens, when a run starts with the approver `other` added, 30 turns and a sprint budget of 50,000 provider tokens, then `limits_set` records the budget of 50,000 at once, a gate of kind `configuration` holds only the new approver and the 30 turns, the limit stays 20 while it is pending, an approval by `other` is ignored with `actor_not_authorised`, and an approval by `maintainer` records `limits_set` with both changes on top of the recorded configuration; given a second run before the approval, then the first gate is withdrawn with the reason `configuration_replaced` (decided by the maintainer on 2026-11-25, counter-review item 11). Verification: TDD.
- **AC9** (R46): Given recorded approvers `maintainer` and `wayloop-bot`, when a run starts with the account `wayloop-bot`, then `run_started` is recorded, no action is proposed and the stop reason is `engine_account_is_approver`; given no `run_started` recorded, then no action is proposed and the stop reason is `run_not_started`. Verification: TDD.
- **AC10** (R47): Given the fixture states of the table Project phase, when the phase is derived, then each gives its phase; given a project with an approved milestone and no brief, then the phase is `delivery`; given a `complete` project and a new brief approved, then the phase is `council`. Verification: TDD.
- **AC11** (R48, R29): Given a project in phase `council` with an approved brief and no council report, when the next steps are computed, then `run_council` is proposed; given the budget `budgets.council` exhausted for that brief, then it is not proposed and the stop reason is `budget_exhausted` with the scope; given an approved council report of that brief, then `run_council` is not proposed again. Verification: TDD.
- **AC12** (R12): Given a draft council report at its gate, when an approver rejects it, then the report is `superseded`, no `run_council` is proposed and the stop reason is `rejected_awaiting_input`; given `unblock_requested` by an approver on the brief, then `subject_resumed` is recorded and `run_council` is proposed again. Verification: TDD.
- **AC13** (R13, R14, R44): Given any sequence of random inputs in which no recorded approver acts, when decided and applied, then no gate is resolved as `approved`, no `limits_set` or `policy_set` widens the configuration, no issue reaches `done` except through a merge observation with `gate_bypassed` or a recorded `gate_waived`, and no artifact passes a person transition of SPEC-0001. Verification: property test.
- **AC14** (R14): Given an issue at `merge` and an approval from an agent account, when processed, then `input_ignored` with `actor_not_authorised` is recorded and the gate stays pending. Verification: TDD.
- **AC15** (R15): Given a draft council report with one question without an answer, when an approver approves it, then `input_ignored` with `question_unanswered` is recorded and the gate stays pending. Verification: TDD.
- **AC16** (R16, R40): Given an issue at `build` with an open question, when its task is superseded, then the issue is `superseded`, its checkout is released with the reason `superseded`, its holds are cleared and the question gate is withdrawn. Verification: TDD.
- **AC17** (R17, R39): Given a policy that waives the merge gate for `small`, an issue whose area and changed paths are `small`, and green required checks at the recorded head, when the checks are observed, then `gate_waived` names the policy revision and the approver of that revision, no `gate_resolved` is recorded, and the action `merge` is proposed; given a changed path of tier `substantial`, then a merge gate is opened instead. Verification: TDD.
- **AC18** (R19): Given one ready issue, when each readiness condition is made false in turn, then the issue is not ready and the report names exactly that condition's code. Verification: TDD.
- **AC19** (R19, R35): Given an active sprint with two lanes of three ready issues each and three free workers, when the next steps are computed, then two claims are proposed, the first ready issue of each lane, and `ready` is empty; given one lane whose first issue waits at `merge` without a checkout, then no claim is proposed for that lane. Verification: TDD.
- **AC20** (R19): Given random sprints, workers and inputs, when a model runtime follows the proposed actions in their order, turning each into its command and answering each dispatch with a result whose outcome is drawn at random (pass, fail, question, executor failure), while observations of checks and merges arrive in a random order, then no lane ever has two issues in states from `claim` to `merge` and the number of checkouts never exceeds the workers. Verification: property test (runtime model: section 3 of the counter-review).
- **AC21** (R19): Given no active sprint and a policy with the areas `core` and `docs`, when the next steps are computed, then each area is a lane and an issue whose area is not an area of the policy is reported with `not_in_lane`; given two areas where the path of one is a prefix of the path of the other, then the policy is refused with `area_paths_overlap`. Verification: TDD.
- **AC22** (R20, R43): Given the first claim of a task whose story, two specifications `SPEC-0003` and `SPEC-0001`, epic and milestone are `approved`, when accepted, then the issue is at `claim` with a checkout naming its branch and worktree, and `artifact_status_changed` is recorded for the task, the story, `SPEC-0001`, `SPEC-0003`, the epic and the milestone, in that order, each to `implementing` (decided by the maintainer on 2026-11-25, counter-review item 7). Verification: TDD.
- **AC23** (R21): Given an issue at `build` with dispatch D2 after a retry of dispatch D1, when a result citing D1 arrives with an executor call record, then it is refused with `dispatch_stale` and only `executor_call_recorded` is recorded; given a result citing D2, then it is accepted. Verification: TDD.
- **AC24** (R22): Given a lease of 120 minutes and a session whose latest event extending its lease was recorded at 10:00, when `due_events` runs at 11:59, then it returns nothing; when it runs at 12:00, then it returns `checkout_released` with `lease_expired` and `session_ended` with `abandoned`, in that order and with no `summary_recorded` (until #52, decided by the maintainer on 2026-11-25, counter-review item 80), the issue keeps its state and its task moves to `approved`; given `lease_renewed` at 11:30, then nothing is due at 12:00; given an issue with a recorded `step_completed` of `pull_request`, then its task stays `implementing`, also after its pull request was closed and cleared (second counter-review B1); given an issue whose pull request is recorded while its `pull_request` dispatch is open, then its task moves to `approved`; given a session that holds no checkout and whose latest event extending its lease was recorded at 10:00, when `due_events` runs at 12:00, then it returns only `session_ended` with `abandoned` (second counter-review B12). Verification: TDD.
- **AC25** (R23): Given `limits.executor_retries` set to 2, when the executor of a step times out three times, then two `step_retried`, one `step_failed` and one `issue_blocked` with `executor_failure` are recorded; given a result that does not validate against the schema of its step, then `result_rejected` with the code `result_schema` and the reason the schema interpreter gives is recorded and counts as one retry (second counter-review B7). Verification: TDD.
- **AC26** (R24): Given a checklist result that omits requirement R2 cited by the task, a counter-review result with an objection without a ruling and a review result of the lens `proof` without a vote for one coverage unit of the task, when each is submitted, then each is recorded as `result_rejected` with `checklist_incomplete`, `objection_unruled` or `vote_missing`. Verification: TDD.
- **AC27** (R25, R11): Given a policy in which the tier `small` skips `counter_review`, when an issue of area tier `small` completes `reanalysis`, and `verify` with the changed paths at the verified head recorded, then it moves to `checklist` and `pull_request`, and each `step_completed` lists the skipped step; given a changed path of tier `substantial` observed before `verify` completes, then `review` is not skipped. Verification: TDD.
- **AC28** (R26): Given an issue at `verify` at attempt 1 of 3, when the verification fails on check `unit`, then `attempt_failed` with the source `verification` names `unit` and carries a failure note, the attempt number is 2 and the issue is at `build`. Verification: TDD.
- **AC29** (R27): Given `limits.attempts` 3 and `limits.same_failure` 5, when attempts 1, 2 and 3 fail on different checks, then the failures of attempts 1 and 2 move the issue to `build` and the failure of attempt 3 opens the question `repeated_failure`; no attempt 4 is started. Verification: TDD.
- **AC30** (R27, R31): Given `limits.same_failure` 2, when check `unit` fails on two consecutive attempts, then a question of category `repeated_failure` with the options `retry` and `stop` is opened and the issue is held; when an approver answers `retry`, then the attempt number is 1, the failure counts are reset and the issue resumes at `build`; when an approver answers `stop` instead, then the hold becomes `stopped_by_person`. Verification: TDD.
- **AC31** (R28): Given a limit of N turns, when N executor steps have started and another is requested, then the issue is held with `turn_limit` and no `step_started` is recorded; given `limits_set` with N + 5 turns after an approved configuration gate, then the hold is removed and the step can be requested. Verification: TDD.
- **AC32** (R29): Given a sprint budget of N provider tokens, when an executor call brings the recorded provider tokens of the sprint to N, then `budget_exhausted` is recorded, no step of the sprint other than a dispatch of `review` covered by the verification reserve is proposed, and a result of a step already started is still accepted; given `limits_set` with a budget above N, then `budget_restored` is recorded and steps are proposed again; given N − 1 provider tokens recorded and a `limits_set` that lowers the budget to N − 1, then `budget_exhausted` is recorded right after the `limits_set` (second counter-review S8). Verification: TDD.
- **AC33** (R30, R31): Given an issue at `counter_review`, when its result raises a question of category `security_sensitive_action`, then the issue keeps `counter_review`, is held with `question` and no action is proposed for it; given an answer from a person who is not an approver, then `input_ignored` is recorded; given a free-text answer from an approver, then `question_answered` and `issue_unblocked` are recorded and the issue resumes at `counter_review`. Verification: TDD.
- **AC34** (R32, R34): Given an issue at `pull_request` before its checks complete, when `pull_request_merged` is observed, then the issue is `done` and `gate_bypassed` and `reconciliation_required` are recorded; given an issue at `build`, when `issue_closed` not caused by its pull request is observed, then it is held with `reconciliation_required` and the code `issue_closed_without_merge`; given a `done` issue, when `issue_reopened` is observed, then `reconciliation_required` is recorded with the code `issue_reopened`, no hold is added and the issue stays `done`. Verification: TDD.
- **AC35** (R33): Given any observation delivered k times with the same external identifier, k from 1 to 5, in any interleaving with other inputs, when decided and applied, then the state and the ledger equal those after one delivery. Verification: property test.
- **AC36** (R34, R35): Given an issue at `merge` with an approved merge gate, when its pull request is merged, then the issue is `done`, its task is `done` and no `gate_bypassed` is recorded; given an issue whose `pull_request` dispatch is closed by its result, then the checkout is released with the reason `awaiting_checks` and its task stays `implementing`, also after a later claim and lease expiry. Verification: TDD.
- **AC37** (R36): Given an issue at `merge`, when its pull request is closed without merge, then it is held with `pull_request_closed`, its pull request is cleared and the merge gate withdrawn; given `pull_request_reopened` of the same number while a required check is still pending, then the hold is removed, the pull request is restored as at the close and the issue is at `pull_request` (with every required check `success` at `verified_head`, AC84: the merge gate opens again; final check 2); given `unblock_requested` by an approver instead, then it resumes at `build`. Verification: TDD.
- **AC38** (R37, R38): Given an issue at `merge` with an approved gate at head H1, when the head H2 is observed, then the gate is withdrawn and the issue is at `pull_request`; given then a check result and a review at H1, then each is recorded as `input_ignored` with `head_stale`; given required checks at H2 with one `cancelled` and later `success`, then the merge gate opens; given one `timed_out`, then `attempt_failed` is recorded. Verification: TDD.
- **AC39** (R39): Given an issue at `merge` with an approved gate, when the next steps are computed before the required checks at the recorded head are all `success`, then no `merge` action is proposed; after they are, then one is. Verification: TDD.
- **AC40** (R41): Given a revision R1, an event on issue 13 and a command on issue 12 with the expected revision R1, when decided, then it is accepted; given an event on issue 12 after R1, then the command is refused with `revision_stale`, `expected` holds the current revision and the ledger is unchanged; given a project command after a `gate_opened` on another issue, then it is refused. Verification: TDD.
- **AC41** (R42, R5): Given a project with a pending council report gate, an issue held with `turn_limit` and a sprint budget exhausted, when `next_steps` is computed with `integrity` valid, then the stops are `approval_required`, `turn_limit` and `budget_exhausted`, each naming its gate, issue or scope, and every value matches `^[a-z][a-z0-9_]*$`; given the same state with `integrity` broken, then the only stop is `ledger_broken` and no action is proposed. Verification: TDD (the output of `wayloop status` is tested by SPEC-0004; section 3 of the counter-review).
- **AC42** (R43): Given the last task of a story merged while another task of the story is superseded, when processed, then the story moves to `done`; given an epic whose only task is superseded, then the epic does not move. Verification: TDD.
- **AC43** (R1, R5, R18, R20, R26, R34, R58): Given the scenarios of `examples/`, each valid against the trace schema of section Examples, when the runner applies each input in order to its initial state, binding every symbol to the value the engine produced at its first appearance and requiring equal values at every later appearance, then every expected state, hold, refusal and recorded event matches, and a scenario with an unbound symbol, an unknown field or a missing required field fails the schema. Verification: end-to-end (section 3 of the counter-review).
- **AC44** (R6, R19): Given an issue with one dependency not done, a hold and a checkout held by another session, when a claim is requested, then it is refused with `dependency_not_done`, `issue_on_hold` and `checkout_held` in one pass, in the order of the guard column, each with its pointer, and the ledger is unchanged; given an issue at `merge`, when a claim is requested, then it is refused with `awaiting_external` (I2, second counter-review N3); given a `step_requested` from a session for an issue that is not checked out, then it is refused with `checkout_not_held`, not with `transition_not_allowed` (second counter-review B2). Verification: TDD.
- **AC45** (R4): Given a `step_result` without a dispatch and a `pull_request_merged` without an external identifier, when submitted, then each is refused with `input_class` and the reason `missing` at the missing field; given a `pull_request_merged` that carries `expected_revision`, then it is refused with `input_class` and the reason `unknown_field`; given a `claim` command without an expected revision through the entry point, then it is refused with `command_input` and the reason `missing`, and `decide` is not called. Verification: TDD.
- **AC46** (R7): Given a ledger whose latest `limits_set` sets 20 turns and a configuration file changed to 30 turns after it, when the ledger is replayed and the next steps are computed, then the limit applied is 20. Verification: TDD.
- **AC47** (R1, R2, R3): Given a random sequence of inputs decided one after the other by a writer, when the recorded events are applied again from the initial state, then the state is byte-identical to the state the writer held after each event, and no call of `decide` happens during the replay; and given any state and input, when `decide` runs, then every emitted event carries the `input_hash`, the `dispatch` or the `external_id` of the input, according to its class. Verification: property test.
- **AC48** (R49, R8, R11): Given an issue of tier `contract` whose task cites one requirement with two acceptance criteria, when the review step starts, then three dispatches follow one another with the lenses `presence`, `behaviour` and `proof`; given a verifier that ran `build` of the issue, then its dispatch is refused with `verifier_not_independent`; given the votes `covered` from every lens, then the issue moves to `pull_request`; given one `uncovered` vote, then it moves to `build`; given one `needs_validation` vote and no `uncovered` one, then it is held with a question of category `needs_validation`. Verification: TDD.
- **AC49** (R50): Given two units `0002/R8/AC4` and `0002/R9/AC5` in state `stale` and one ready issue, when the next steps are computed, then `ready` lists the issue first and then the two units in that order. Verification: TDD.
- **AC50** (R51): Given a unit `0001/R1/AC1` `covered` by verdicts produced on commit C0 of an earlier issue, citing `crates/a.rs`, when `pull_request_merged` of another issue is observed with `changed_paths` holding `crates/a.rs`, at `merge` with an approved gate (I18) and, in a second run, at `build` without approval (I24), then in both runs `coverage_stale` names the unit, the merge commit as `commit` and `crates/a.rs` (SPEC-0003 R16); given `changed_paths` without `crates/a.rs`, then the unit stays `covered`; given the merge of the pull request whose merged head H1 is the commit of every verdict of the unit, with `changed_paths` holding `crates/a.rs`, then no `coverage_stale` is recorded; given the merge of a pull request that closes no planned issue, or one whose issue is `done`, with `changed_paths` holding `crates/a.rs`, then P13 records `pull_request_merged` and `coverage_stale` for the unit, and `reconciliation_required` for the `done` issue; given `default_branch_pushed` of commit C9 with `changed_paths` holding `crates/a.rs`, then `coverage_stale` names the unit and C9 as `commit`; given either observation with no cited file in `changed_paths`, then only `input_ignored` with `no_effect` is recorded. Verification: TDD.
- **AC51** (R49): Given the paths of the branch observed at head H2 during `build`, and then `verify` passed with a `verification_result` that names H2, when the next steps are computed, then `run_step` of `review` is proposed and a `step_requested` for `review` is accepted, although no `branch_changes_observed` was recorded after `build`; given `verify` that names H3 while the paths are recorded at H2, then no action is proposed for the issue, its report of unmet conditions (R19) names `awaiting_external`, and a `step_requested` for `review` is refused with `head_not_observed` and H3 in `expected`; given then `branch_changes_observed` at H3, then the step is accepted. Verification: TDD.
- **AC52** (R52): Given a policy whose `skips` holds `{tier: substantial, steps: [review]}`, when `wayloop init` or a run start carries it, then it is refused with `policy_skip_not_allowed`, the reason `enum` and `counter_review` in `expected`; given `merge_waived_tiers` holding `substantial`, then it is refused with `policy_waiver_not_allowed`, the reason `enum` and `small` in `expected`; in both cases nothing is recorded. Verification: TDD.
- **AC53** (R53): Given `wayloop init` with approvers and every token budget but no limit, when it runs, then the first `limits_set` records `turns_per_issue` 20, `attempts` 3, `same_failure` 2, `executor_retries` 2 and `lease_minutes` 120, and `meta.defaults_applied` lists the five keys of `limits`; given a given limit, then it is recorded and not listed; given no `budgets.sprint`, then the configuration is refused with `budget_missing`, the reason `missing` and the pointer of the key, and no ledger is created; given a valid configuration, then `genesis` returns `ledger_created`, `limits_set` and `policy_set` with `seq` 0, 1 and 2, chained, each with the `input_hash` of the configuration (R3, second counter-review S2). Verification: TDD.
- **AC54** (R54): Given the tables Issue transitions, Session transitions, Gate transitions and Project transitions of this file, when a test reads them and the constant table of the core, then every row of one has a row of the other with the same identifier, source states, input, target and recorded events in the same order; a row added, removed or changed on one side only fails the test with the identifier of the row. Verification: contract test.
- **AC55** (R55): Given the guard predicates of the core and the table Errors, when a test enumerates them, then every guard predicate names exactly one code of the table, every selector names none, and every code of the table with the severity `refusal` or `recorded` is named by at least one predicate or rule of its row. Verification: TDD.
- **AC56** (R56): Given states reached by random sequences of decided inputs, when every event is applied, then the invariants of section Invariants hold after each event. Verification: property test.
- **AC57** (R57): Given any sequence of events that `decide` produced, and the same sequence with one event removed or repeated, when applied, then `apply` returns a state and never panics; given a ledger whose `seq` does not follow on, then the loader stops with `ledger_sequence_gap` (SPEC-0003 R60) before `apply` is called. Verification: property test.
- **AC58** (R1): Given a state whose last event has `seq` 41 and hash H41, when `decide` accepts a claim that emits three events at the time `at`, then the events carry `seq` 42, 43 and 44, the first has `prev_hash` H41 and each next one the `hash` of the one before, every `recorded_at` is `at`, every `hash` verifies under SPEC-0003 R58, and after `apply` the state holds the last `(seq, hash)` of the three. Verification: TDD.
- **AC59** (R6): Given an issue at `build` and a `step_requested` for `verify` with three subject guards failing, when decided, then the only error is `step_mismatch`; given a `step_requested` for `build` from a session that does not hold the checkout, with a dispatch open and the turn limit reached, then the errors are `checkout_not_held` and `dispatch_open` in the order of the guard column; given a `step_result` whose checklist misses a requirement, then nothing is refused and `result_rejected` with `checklist_incomplete` is recorded. Verification: TDD.
- **AC60** (R18): Given an issue at `build` with a dispatch of `build`, when a `verification_result` citing that dispatch arrives with an executor call record, then `executor_call_recorded` and then `result_rejected` with `result_kind_mismatch` are recorded and the retries increase by one; given a `step_result` with the outcome `changes_requested` for a dispatch of `checklist`, then the same code is recorded. Verification: TDD.
- **AC61** (R59): Given a session whose lease has ended, when a result or an observation that changes state arrives, then the due events are recorded before the events of that input; given `now` earlier than the `recorded_at` of the last event, then the due events carry that `recorded_at`; given an input time with a fraction of a second, then the input is refused by its schema. Verification: TDD.
- **AC62** (R53): Given `wayloop init` with `limits.attempts` 0, `limits.lease_minutes` 0 or `budgets.sprint` 0, when it runs, then it is refused with `limit_range`, the reason `range` and the pointer of the key, and nothing is recorded; given `limits.executor_retries` 0, then it is accepted. Verification: TDD.
- **AC63** (R44): Given `wayloop init` with the approvers `maintainer` and `maintainer`, when it runs, then it is refused with `approver_duplicate` and the reason `duplicate`; given the recorded approver `maintainer` and an approval whose actor is `Maintainer`, when decided, then `input_ignored` with `actor_not_authorised` is recorded, because the core compares exactly what the adapter gives. Verification: TDD.
- **AC64** (R20, R53): Given a claim of issue 12 whose task has the title "Record a time entry" and no `branch_type`, in a project named `wayloop`, when accepted, then `checkout_acquired` carries the branch `feat/12-record-a-time-entry` and the worktree `wayloop-12` (section Names of a checkout); given a task with the `branch_type` `fix`, then the branch starts with `fix/12-` (this clause is tested by task T13, which builds the member); given a `claim` result that reports another worktree name, then `result_rejected` with `checkout_mismatch` is recorded with the derived names in `expected`; given `wayloop init` without `project.name`, or with `Way Loop`, then it is refused with `project_name_invalid` and the reason `missing`, or `syntax` with `fix` `way-loop`, and nothing is recorded; given the name `---`, from which the producer gives only `work`, then it is refused with the reason `syntax` and no `fix`. Verification: TDD.
- **AC65** (R37): Given an issue at `build` after a failed required check, when `pull_request_synchronized` at H2 is observed, then the head is recorded, the issue stays at `build` and no hold is added; given a re-run of check `rust` at the recorded head while the issue is at `build`, whether it passes or fails, then only the conclusion of `rust` in `checks` changes, and no `attempt_failed` is recorded and no count of R27 changes (I42, second counter-review B3); given an issue at `pull_request` with no dispatch and a run of the required checks `docs` `success` and `rust` `cancelled`, then the run is recorded, `checks` holds `docs` `success` and the issue stays at `pull_request` (I51); given checks at a head H4 not yet observed while the issue is at `build`, then they are kept in `pending_checks`, and when the issue is at `pull_request` without dispatch and `pull_request_synchronized` at H4, which is `verified_head`, is observed, then `checks` holds them and the row they select applies (I22, I47, second counter-review B4); given an issue at `merge` with an approved gate and a failing required check at the recorded head, then the gate is withdrawn with `checks_failed`, `attempt_failed` is recorded and the issue is at `build`; given a `pull_request_checks_completed` at head H3 not yet observed, then it is recorded and the issue is unchanged, and when `pull_request_synchronized` at H3 follows, then its conclusions apply at that input. Verification: TDD.
- **AC66** (R37): Given an issue at `review` with `verified_head` H2, when `branch_changes_observed` at H3 is observed, then the issue is at `verify`, its open dispatch of `review` is closed, `verified_head` is cleared and the attempt number is unchanged. Verification: TDD.
- **AC67** (R25): Given an issue of tier `small` whose verification passes at H1 while no changed paths at H1 are recorded, when the `verification_result` is decided, then the issue is at `review` and not at `pull_request`; when `branch_changes_observed` at H1 then gives the tier `small`, then the issue is at `pull_request` and `step_completed` for `review` with the outcome `completed` and `skipped` `[review]` is recorded; given paths at H1 that give the tier `substantial`, then the issue stays at `review` and `run_step` of `review` is proposed. Verification: TDD.
- **AC68** (R35, R22): Given an issue at `pull_request` with `verified_head` H1, when `pull_request_opened` at H2 is observed, then the pull request is recorded and the issue is at `verify` with no attempt counted; given `pull_request_opened` at H1, then the issue stays at `pull_request` with its pull request recorded and its dispatch open until the `step_result` of `pull_request` closes it; given that `step_result` while the recorded head is H1 and `verified_head` is H3, or while `head_pending` is true, then the dispatch is closed, the checkout released and no row of the checks is applied, and the issue waits at `pull_request` for I22 or I39 (second counter-review B5); given a task in `approved` whose issue is `open` when the merge of its closing pull request is observed, then the task moves to `done` (SPEC-0001, as amended). Verification: TDD.
- **AC69** (R30, R31): Given an issue held with `reconciliation_required` and `stopped_by_person`, when an approver sends `unblock_requested` naming `stopped_by_person`, then only that hold is removed and no `issue_unblocked` is recorded; when an approver sends `unblock_requested` naming no reason, then the last hold is removed, `issue_unblocked` is recorded and its `resumed_at` is the state the issue is in at that moment (final check 2); given an answer `retry` on a question `repeated_failure` while another hold remains, then the attempt number and the failure counts change only when that question hold is removed. Verification: TDD.
- **AC70** (R30, R40): Given an issue held with `stopped_by_person` while its `build` dispatch is open, when the result of that dispatch arrives, then it is decided as usual and the issue moves to `verify`, still held; given an issue whose task is superseded while a dispatch is open, then the dispatch is closed and a later result of it is refused with `dispatch_stale`, its executor call record kept; given `issue_closed` marked as caused by the closing pull request before `pull_request_merged`, then `input_ignored` with `no_effect` is recorded, and the merge then makes the issue `done` with no hold. Verification: TDD.
- **AC71** (R54; SPEC-0003 R12, R48): Given a session that holds no checkout, when its holder sends `session_end_requested` without a hand-over, then `session_ended` with the outcome `completed` is recorded; given a session whose lease ended without a hand-over, when its last holder submits a hand-over before another claim of the issue, then `handover_recorded` with `late` `true` is recorded; given another session that claimed the issue first, then the late hand-over is refused with `checkout_not_held`; given a session that holds no checkout, then `session_end_requested` with a hand-over and `lease_renewal_requested` are refused with `checkout_not_held`, and given a session that holds a checkout, then `session_end_requested` without a hand-over is refused with `handover_input` and the reason `missing` (S2 to S4, second counter-review S4). Verification: TDD.
- **AC72** (R23): Given a step with one retry used, when the step completes, then the retries of the issue are 0; given an issue held with `executor_failure`, when an approver removes the hold, then the retries are 0. Verification: TDD.
- **AC73** (R26, R27): Given an issue at attempt 1 of 3, when the tally of its review gives a unit `uncovered`, then `attempt_failed` with the source `review` is recorded and the issue is at `build` at attempt 2; given an issue at `merge` whose gate an approver rejects at attempt 3 of 3, then `attempt_failed` with the source `merge_rejected` is recorded and the question `repeated_failure` is opened. Verification: TDD.
- **AC74** (R27): Given `limits.same_failure` 2, when check `unit` fails in `verify`, then passes as a required check, then fails in `verify`, then the count of `unit` is 1 and no question is opened; given a failure of `unit`, then a run of the checks that does not report `unit`, then a failure of `unit`, then the count is 2 and the question is opened. Verification: TDD.
- **AC75** (R38): Given a policy with no required check, when `pull_request_opened` at `verified_head` is observed, then the issue moves to `merge` and the merge gate opens; given a policy with the required check `rust` and an issue at `pull_request` with its pull request recorded and no conclusion of `rust`, then the stop reason `awaiting_checks` names the issue. Verification: TDD.
- **AC76** (R14, R54): Given an issue at `merge` with a pending gate, when a `pull_request_review_submitted` with the state `approved` by an approver is observed, then `input_ignored` with `no_effect` is recorded and the gate stays pending. Verification: TDD.
- **AC77** (R19): Given an active sprint with one lane of three ready issues and no free worker, when the next steps are computed, then `ready` lists only the first issue of the lane; given an issue listed in two lanes of the baseline plan, then `reconciliation_required` with the code `issue_in_two_lanes` is recorded, the issue is not ready and its readiness report gives `not_in_lane` (second counter-review N2); given a lane occupied by issue 7 at `merge`, then the readiness report of the next issue of the lane gives `lane_busy` with 7 in `found`. Verification: TDD.
- **AC78** (R49): Given an issue of tier `substantial`, when a `step_requested` for `review` names the lens `presence`, then it is refused with `lens_not_expected` and `proof` in `expected`; given a verifier whose executor and holder equal those of the `build` dispatch, then it is refused with `verifier_not_independent`; given a task that cites no requirement with an acceptance criterion, when the review step is requested, then the issue is held with a question of category `needs_validation` and no `step_started` is recorded; given then an answer from an approver, then `question_answered`, `step_completed` of `review` and `issue_unblocked` are recorded and the issue is at `pull_request` (second counter-review S9); given a hold `stopped_by_person` that remains when the question is answered, then the issue stays at `review` with no `step_completed`, and when an approver removes that hold, `step_completed` of `review` and `issue_unblocked` are recorded, the issue is at `pull_request` and no question is opened again (second counter-review check N-S2); given the question of I46 pending and a `pull_request_synchronized` at a head other than `verified_head` (I43), then the issue is at `verify`, `gate_withdrawn` with the reason `review_left` is recorded for the question, its hold is removed, `no_unit_acknowledged` is not set and `issue_unblocked` records `resumed_at` `verify` (decided by the maintainer on 2026-11-25, final check 2; criterion: stability and maintainability). Verification: TDD.
- **AC79** (R28): Given an issue at `review` whose turns equal `limits.turns_per_issue` and whose review dispatch is covered by the verification reserve, when the step is requested, then `step_started` is recorded and no `turn_limit` hold is added. Verification: TDD.
- **AC80** (R29): Given a sprint budget of N provider tokens, when two calls each bring the sprint above N, then exactly one `budget_exhausted` is recorded, at the first; given a stale result whose call brings the sprint to N, then `executor_call_recorded` and `budget_exhausted` are recorded and nothing else; given two sprints S01 and S02, then the provider tokens of S01 do not count in S02. Verification: TDD.
- **AC81** (R41): Given a session holding issue 12 and an observation on issue 12 after the revision R1, when the holder sends `lease_renewal_requested`, then `lease_renewed` is recorded, since the command carries no revision; given a second `lease_renewal_requested` of the same session with the same content, then a second `lease_renewed` is recorded and the lease is extended again, since R52 of SPEC-0003 does not apply to it (second counter-review B8); given `session_end_requested` with the revision R1 and only events of the issues of other sessions after R1, then it is accepted; given a `checkout_released` of the session after R1, then it is refused with `revision_stale`; given a revision that is not the hash of an event of the current segment, then it is refused with `revision_stale`. Verification: TDD.
- **AC82** (R19, R22): Given an issue whose `pull_request` step completed (I44) and whose pull request was then closed (I25) and unblocked to `build`, when it passes `verify` and `review` again and reaches `pull_request`, and the lease of its session ends before the `pull_request` step is dispatched, then the issue is ready and its task stays `implementing`; given an issue at `pull_request` whose step completed in the current pass and whose pull request is not yet observed, then it is not ready and its readiness report gives `awaiting_external`; given that pull request then observed (I15), then it is still not ready (decided by the maintainer on 2026-11-25). Verification: TDD.
- **AC83** (R42, R29): Given a reconciliation that belongs to no milestone and an active sprint, when its `reconcile` step runs, then its provider tokens are charged to `budgets.sprint` of that sprint; given no active sprint, when the next steps are computed, then no `reconcile` action is proposed for it and `stops` holds `no_active_sprint` naming its subject; given a `reconcile` dispatch started while sprint S1 was active and a `plan_observed` that ends S1 before its result, when its `executor_call_recorded` is recorded, then no scope is charged, `unassigned_provider_tokens` grows by its provider tokens, no `budget_exhausted` is recorded and `state.yaml` shows the new total (final check, decided by the maintainer on 2026-11-25); given a `project_step_requested` for that `reconcile` step with no active sprint, then it is refused with `no_active_sprint` and the reason `state`, and nothing is recorded (decided by the maintainer on 2026-11-25). Verification: TDD.
- **AC84** (I26, I16, I39, R18): Given an issue moved to `build` by a failed required check at H1 (I20), which then pushes H2 (I41) and completes `build` (I9), so that it is at `verify` with `verified_head` H1 and `pull_request_pass_completed` set, when its pull request is closed (I25) and reopened (I26), then the issue is at `verify`, not at `pull_request`, and `issue_unblocked` is recorded; given an issue at `pull_request`, held with `pull_request_closed` after its `pull_request` step completed, when the pull request is reopened, then it is at `pull_request` with `pull_request_pass_completed` still set and the pull request restored with its head, `heads`, `checks` and `head_pending` as at the close; given an issue at `merge` whose required checks were all `success` at `verified_head` when its pull request was closed, when the pull request is reopened, then I16 is applied from the restored checks and the issue is at `merge` with `gate_opened` (kind `merge`) recorded, or `gate_waived` for the tier `small` with a waiver (decided by the maintainer on 2026-11-25, final check 2; criterion: stability and maintainability); given an issue at `review` with `no_unit_acknowledged` set whose last hold is `pull_request_closed`, when the pull request is reopened, then it is at `pull_request`, `step_completed` of `review` is recorded and `pull_request_pass_completed` is cleared; given an issue at `pull_request` with no dispatch whose recorded head H2 differs from `verified_head` H1, when a `pull_request_checks_completed` at H2 makes every required check `success`, then the issue is at `verify` with `verified_head` cleared, no attempt counted, and neither `gate_opened` (kind `merge`) nor `gate_waived` is recorded, also for the tier `small` with a waiver (final check, decided by the maintainer on 2026-11-25); given an issue whose pull request is recorded at H3 (I41) while `verify` passes at H2 and `review` passes, when it reaches `pull_request` (I13), then I39 applies at once and the issue is at `verify` with `verified_head` cleared and no attempt counted; given the same head at the close of the `pull_request` dispatch (I44), then the same (decided by the maintainer on 2026-11-25, final check 2; criterion: stability and maintainability). Verification: TDD.
- **AC85** (R30, R31, R36): Given an issue at `review` held with `pull_request_closed`, when a `branch_changes_observed` at a head other than `verified_head` moves it to `verify` (I43) and the pull request is then reopened, then the issue stays at `verify`, `issue_unblocked` records `resumed_at` `verify`, and it is ready; given an issue at `build` with its `build` dispatch open when the hold `pull_request_closed` is entered, when that dispatch completes (I9) and the pull request is then reopened, then the issue stays at `verify` and is not sent back to `build`; given an issue at `pull_request` held with `pull_request_closed` and `stopped_by_person`, when an approver sends `unblock_requested` naming `pull_request_closed`, then the issue moves to `build` at once, `closed_pull_request` is dropped and no `issue_unblocked` is recorded; given an `issue_blocked` of any reason, then it carries no target (decided by the maintainer on 2026-11-25, final check 2; criterion: stability and maintainability). Verification: TDD.

## Model

### Machines and state

The engine runs four machines over one state, which is a projection of the ledger (SPEC-0003):

| Machine | Instances | State | Section |
| --- | --- | --- | --- |
| Project phase | One per project | `phase`, derived from artifact statuses | Project phase |
| Gate | One per approval, question or configuration change | `gate_state` and its resolution | Gates and approval states |
| Session | One per session | open or ended, its holder, executor and checkouts | Sessions |
| Issue cycle | One per planned task issue | `issue_state`, checkout, holds, dispatch, counters | Issue cycle |

The state also holds the recorded configuration (approvers, limits, budgets, policy), the plan
as last observed, the sessions and the latest event that extends the lease of each, the project
dispatches, the recorded `external_id` values, the map from the `input_hash` of each command to
the `seq` of the first and last event it caused, renewals of a lease left out (SPEC-0003 R52,
second counter-review check N-S4), the map from the hash of each
event of the current segment to its `seq` (section Revisions), the last event `(seq, hash)`
(R1), the provider tokens recorded per budget scope, the provider tokens recorded with no budget scope
(`unassigned_provider_tokens`, section Limits and budgets; always present, 0 when no token is
unassigned, so that the shape of the state and of `state.yaml` is fixed from the first state
(decided by the maintainer on 2026-11-25, final check 2; criterion: stability and maintainability)) and the account of the runtime. The set of
`external_id` values grows without limit with the ledger (decided by the maintainer on
2026-11-25, counter-review item 63).

The issue-cycle state is not the status of the task artifact (SPEC-0001 R5). A task in status
`implementing` may be at any non-terminal issue-cycle state, checked out or not, held or not.
`blocked` is not a value of any enum: an issue is blocked while its set of holds is not empty.

The state of one issue:

| Field | Meaning |
| --- | --- |
| `issue_state` | One of the values of the table Issue states |
| `checkout` | Absent, or `{holder, session, executor, branch, worktree}` from `checkout_acquired` (SPEC-0003) |
| `holds` | Set of `{reason, gate}`, keyed by `(reason, gate)`; `gate` names the question of a hold with the reason `question`. No target is stored: the target of a removal is computed when the hold is removed, from the state at that moment (table Holds) (decided by the maintainer on 2026-11-25, final check 2; criterion: stability and maintainability) |
| `closed_pull_request` | Absent, or the value of `pull_request` that I25 cleared, kept unchanged while the hold `pull_request_closed` remains, so that I26 restores it; dropped when that hold is removed, or when another pull request is recorded (I15, I39), after which a reopen of the old number matches no row and is handled under R32 (decided by the maintainer on 2026-11-25, final check 2; criterion: stability and maintainability) |
| `dispatch` | Absent, or the hash of the open `step_started` event |
| `turns` | Executor steps started for the issue, a count |
| `attempt` | Number of the current attempt, from 1 |
| `failures` | For each check name, the number of consecutive attempts on which it failed |
| `retries` | Failures of the open step, a count: executor failures and rejected results; reset at every `step_completed` and when a hold `executor_failure` is removed (R23) |
| `panel_repeated` | Whether the one repetition of the panel of the current review step is used (R8) |
| `verifiers` | The verifiers `(executor, holder)` of the lenses already dispatched in the current review step, and those of `build` and `counter_review` (R49) |
| `pull_request` | Absent, or `{number, head_sha, heads, checks, head_pending}`; `head_sha` is the recorded head, the one meaning of "recorded head" in this specification (second counter-review S1); `heads` lists every head observed for it, in order; `checks` maps each check name to its latest conclusion at `head_sha`, after each run is applied: a `cancelled` conclusion leaves the entry of its check unchanged (R38); `head_pending` is true from a failed required check (I20) until the new head is observed (I21, I22, I39), and while it is true I44 applies no row of the checks (second counter-review B5) |
| `pending_checks` | For each head of the pull request not yet observed, the conclusions of the check runs recorded at it (I47). When a row records that head as the head of the pull request (I15, I21, I22, I39, I41), its conclusions replace `checks` and the entries of earlier heads are dropped (R37; decided by the maintainer on 2026-11-25, second counter-review B4) |
| `changed_paths` | The paths of the latest `branch_changes_observed` and the head they were observed at |
| `verified_head` | Absent, or the head named by the latest passing `verification_result` (I10); `review` needs `changed_paths` at this head (I3) |
| `pull_request_step_completed` | Whether a `step_completed` of the step `pull_request` is recorded for the issue; set by I44 and never cleared, also when the pull request is closed and cleared (I25). R22 and I31 read it (decided by the maintainer on 2026-11-25, second counter-review check N-S2) |
| `pull_request_pass_completed` | Whether the `pull_request` step of the current pass has completed; set by I44, cleared when the issue enters `pull_request` from `verify` or `review` (I10, I13, I48, and the move of R49 at the removal of the last hold), which starts a new pass, and kept when it returns from `merge` (I22) or by a reopen (I26) while the issue is at `pull_request` or `merge`, which continue the same pass; a reopen in an earlier state leaves the issue in that state, and its next entry to `pull_request` clears the field (final check, decided by the maintainer on 2026-11-25); the state is the one at the reopen, not at the close (final check 2). It is not cleared at I15 or at the dispatch of the step (I3): the pull request may be observed after I44, and before the dispatch a second pass must already be ready. The table Ready work and lanes reads it, so that no issue waits for an external event that never comes (decided by the maintainer on 2026-11-25) |
| `no_unit_acknowledged` | Whether an approver has answered the question of I46 that the task has no coverage unit; set by that `question_answered`, which can only happen at `review`, since any move out of `review` withdraws that question and removes its hold (I43, I48; (final check, decided by the maintainer on 2026-11-25)); cleared when the issue leaves `review`. While it is set, the removal of the last hold moves the issue to `pull_request` (R49, table Holds; decided by the maintainer on 2026-11-25, second counter-review check N-S2) |

### Invariants

Checked by `State::check_invariants` after every `apply` in the tests (R56, AC56):

1. An issue at `merge`, `done` or `superseded` has no checkout and no dispatch.
2. An issue has a dispatch only in a state from `claim` to `pull_request`, and at most one.
3. An issue has no two holds with the same `(reason, gate)`.
4. An issue at `done` or `superseded` has no hold and no pending gate.
5. No lane has two issues in states from `claim` to `merge`.
6. A checkout names an open session, and two issues never share a checkout of two sessions.
7. `attempt` is at least 1, and no attempt starts once the failed attempt number is at least `limits.attempts`; no retry starts once `retries` is at least `limits.executor_retries`. A `limits_set` that lowers a limit below the current count is legal (R45), so the counts are not bounded by the current limits (decided by the maintainer on 2026-11-25, second counter-review S7).
8. The last event `(seq, hash)` of the state is the event last applied.
9. An issue has `closed_pull_request` only while it holds `pull_request_closed` (final check 2).

### Time

Time enters the core only as data, as one type `Timestamp`: whole seconds of UTC, written
`YYYY-MM-DDTHH:MM:SSZ` (decided by the maintainer on 2026-11-25, counter-review item 9).

- `decide` takes `at`, the time the writer gives for the events of this input after the rule of
  SPEC-0003 R27; it is written as `recorded_at` of every event it returns, and no guard compares
  it with any other time.
- `due_events(state, now)` is the only function that compares a recorded time with the current
  time. The runtime calls it, and appends what it returns under the lock, at the later of `now`
  and the `recorded_at` of the last event, before it decides any input that changes state:
  command, result or observation (R59); a read never calls it.
- The lease of a session, its end and its renewal are defined once, in SPEC-0003 section Clock
  times (R46); its length is `limits.lease_minutes` of section Limits and budgets.

### Input classes

Every input belongs to one class (R4); the class of each input is given by the table Inputs of
section Events. The class decides the cause recorded, how staleness is
detected and what happens when the input does not fit the state.

| Class | Sent by | Carries | Cause recorded (R3) | When it does not fit the state |
| --- | --- | --- | --- | --- |
| Command | Runtime or agent, through a command (SPEC-0004) | `expected_revision`, except the commands of the table Subjects of commands that carry none | `input_hash` | Refused (R6, R18, R41); nothing recorded |
| Result | Runtime, for an open dispatch: executor output, check results, a council report or a plan | `dispatch` | `dispatch` | Stale dispatch: refused, only its cost recorded (R21). Invalid, of a kind or outcome its step does not accept, or failing a guard: `result_rejected` (R18, R23), because model output that does not validate becomes an error event (ADR 0003) |
| Observation | Adapters: tracker, git, plan; persons act through the tracker (ADR 0012) | `external_id`, `actor` when a person acts | `external_id` | Never refused for its state (R32); duplicates ignored (R33); unauthorised actors recorded as `input_ignored` (R14) |

The ledger records what happened, not what was asked: an accepted input is recorded through the
events it causes, and a refused command or a stale result is not. An observation always causes
at least one event, so that its `external_id` is recorded; one that changes nothing is recorded
as `input_ignored` with the code `no_effect`.

Persons act only through the tracker. The tracker adapter turns a comment, a review or a label
change of an authenticated handle into an observation whose `actor` is that handle, in lower
case (SPEC-0005, SPEC-0007); the core compares the handle with the recorded approvers by exact
spelling (R44). The commands a person runs locally, such as `wayloop init` and `wayloop ledger
recover`, follow SPEC-0003 section Person-only commands: they are not exposed through the MCP
server, the SDKs or tool definitions.

### The functions of the core

```text
decide(state, input, at)                   -> Ok { events: [event] } | Refused { error_report }
apply(state, event)                        -> state
due_events(state, now)                     -> [event]
next_steps(state, integrity, free_workers) -> { actions, ready, stops }
genesis(configuration, ledger_id, at)      -> Ok { events: [event] } | Refused { error_report }
```

- `genesis` is the decision of `wayloop init` (R53), the one input that has no state to decide
  on (decided by the maintainer on 2026-11-25, second counter-review S2). It validates the
  configuration given to `wayloop init` (R44, R52, R53, section Names of a checkout, lanes, and
  the budgets of SPEC-0003 R37) and returns the three finished events `ledger_created`, the first
  `limits_set` and the first `policy_set`, with `seq` 0, 1 and 2, the first with 64 zeros as
  `prev_hash` (SPEC-0003 R2, R57), every `recorded_at` equal to `at`; or a refusal with every
  error found, in one pass. `wayloop init` is a command, so each event carries as its cause (R3)
  the `input_hash` of the configuration given, its canonical JSON. Like the other functions it
  reads no clock, file or environment: the person-only command of SPEC-0003 reads the file and
  the clock and gives the values.

- `decide` validates the input, evaluates the guards and returns the events in the order they
  are recorded, which is the order of the column Recorded of its row (decided by the maintainer
  on 2026-11-25, counter-review item 7). Each event is finished: `decide` assigns its `seq`, its
  `prev_hash`, its `recorded_at` and its `hash` from the last event `(seq, hash)` of the state
  (R1), so that a gate or a dispatch is named by the hash of its event inside the same output.
  A refusal is the `ErrorReport` of `wayloop-schemas` (ADR 0009 C3, ADR 0030 G1); for a command
  it reaches the caller as the refused outcome of its envelope (`Outcome::refused`, SPEC-0004
  R18), and no type of the core repeats its shape. It is total: every pair of state and input
  gives events or a refusal, never a panic. Each event it returns is applied before the next one
  is computed, so that `apply(apply(s, e1), e2)` is the state after the input.
- `apply` is the only way the state changes (R1). It is total over the kinds of the catalogue of
  SPEC-0003 and records each event as a fact: it never refuses and never panics, whatever the
  sequence, including an event that names an issue it does not know or a release of a checkout
  that is absent (R57). It is not idempotent: it increments counters, and the loader alone
  guarantees that each event is applied once, by checking that `seq` follows on (decided by the
  maintainer on 2026-11-25, counter-review item 8). A replay is a fold of `apply` (R2); the
  events carry everything the fold needs, including the tier inputs, the plan and the policy.
- `due_events` returns the events due at `now`, in the order of section Ready work and lanes.
- `next_steps` changes nothing and takes no time (R5). `integrity` is the value the loader gives
  after verifying the chain: `valid`, `broken` or `regressed` (SPEC-0003 R4, R49); a state
  rebuilt from the valid prefix of a broken ledger cannot know it is broken, so the loader tells
  it (decided by the maintainer on 2026-11-25, counter-review items 2 and 3). `actions` lists
  what the runtime may start now, each of kind `run_step`, `merge`, `run_council`,
  `generate_plan`, `reconcile` or `claim`; `ready` lists ready issues that get no action for
  lack of workers; `stops` lists the stop reasons (R42). The runtime turns an action into a
  command; it never chooses among actions (ADR 0006: the core decides, the runtime executes).
- The same arguments give the same output, byte for byte in canonical JSON (constitution II).

### Configuration in the state

Approvers, limits, budgets and policy come from the state (R7), so that a replay never depends
on the configuration of the machine that replays.

- `wayloop init`, run by a person at an interactive terminal, records the first `limits_set`,
  with the explicit list of approvers it is given, and the first `policy_set`, each with that
  person as `approved_by` (R44). Agents, bots and the account of the runtime are never
  approvers; `wayloop init` refuses an empty list with `approvers_empty` and a list that holds
  one handle twice with `approver_duplicate`.
- `run_start_requested` carries the effective configuration (ADR 0010) and the account of the
  runtime. `decide` compares it with the recorded one (R45), key by key, under the table
  Direction of a configuration change. The tightening changes are recorded at once in
  `limits_set` or `policy_set`. The loosening changes open one gate of kind `configuration`
  whose `gate_opened` carries the keys that loosen and their proposed values, never the whole
  configuration and never a file (decided by the maintainer on 2026-11-25, counter-review items
  10 and 11). Only an already recorded approver resolves it, and its approval records
  `limits_set` or `policy_set` with those keys applied on top of the configuration recorded at
  the time of the approval, with that approver as `approved_by`; a later tightening is therefore
  never undone. A new `run_start_requested` withdraws every configuration gate still pending,
  with the reason `configuration_replaced`. A configuration that removes every approver is
  refused with `approvers_empty`.
- `run_started` records the account of the runtime (R46). While that account is an approver, the
  engine proposes nothing: the runtime would otherwise approve its own work. Before the first
  `run_started`, no account is known, and the engine proposes nothing either (R46, decided by the
  maintainer on 2026-11-25, counter-review item 15).
- The plan enters the state through `plan_observed`, which the plan adapter emits from the
  tracker and the repository: items with their type, status, links, area and dependencies, and
  the active sprint with its lanes. It carries the artifact kinds and links that phase and
  readiness read (decided by the maintainer on 2026-11-25, counter-review item 19): briefs;
  council reports with their brief; gate issues with their subject; milestones with their
  council report; epics with their milestone; stories with their epic; tasks with their epic,
  milestone, story, cited specifications and requirements, dependencies and area;
  specifications with their status and, for an approved one, its coverage units; and the sprint
  with its lanes. The first `plan_observed` after `ledger_created` is the baseline and is taken
  as it is; it records `coverage_planned` for every approved specification it holds (SPEC-0003
  R14; decided by the maintainer on 2026-11-25, counter-review item 54).
- Later observations follow three rules (decided by the maintainer on 2026-11-25, counter-review
  items 16, 17 and 18). A plan is applied only when its last part has arrived, so that readiness
  never reads half a plan; the parts are kept apart in the state until then, and parts of two
  plans never mix, because each part names its plan. A new item is taken in `draft`, or in the
  status the ledger already recorded for its identifier, as when a roadmap gate approved it
  before its tracker item existed (SPEC-0001 R25). A status that differs from the state is not
  taken: the ledger is the authority on statuses (SPEC-0001 R17); `decide` records
  `reconciliation_required` with the code `status_conflict` for an item only when the observed
  status equals neither the current nor the previous status that the ledger recorded for it,
  since the tracker adapter writes a status after the engine records it. Statuses change through
  gates, person requests and the transitions of R43.
- The paths changed on a branch enter through `branch_changes_observed`, which the git adapter
  emits for the head of the branch. The engine computes the tier from them (R25); the agent
  never reports its own tier.
- The default branch is the branch that the tracker reports as the default of the repository
  (SPEC-0005), the branch of SPEC-0001 R23. The tracker and git adapters report
  `pull_request_merged` and `default_branch_pushed` only for merges and pushes into it.

Direction of a configuration change (decided by the maintainer on 2026-11-25, counter-review
item 12). A change listed in the second column is recorded at once; every other change, a change
of a key that no row lists included, counts as loosening and needs the configuration gate.

| Key | Tightening, recorded at once | Loosening, through the gate |
| --- | --- | --- |
| `approvers` | Removing a handle, while one remains | Adding a handle |
| `limits.turns_per_issue`, `limits.attempts`, `limits.same_failure`, `limits.executor_retries`, `limits.lease_minutes` | Lowering, within the minimum of section Limits and budgets | Raising |
| `budgets.council`, `budgets.planning`, `budgets.sprint`, `budgets.milestone` | Lowering, within the minimum | Raising |
| `ledger_budgets` (SPEC-0003) | Lowering, within the bounds of SPEC-0003 R37 | Raising |
| `policy.areas` | Raising the tier of an existing area | Adding, removing or renaming an area; changing its paths; lowering its tier |
| `policy.path_tiers` | Raising the tier of an existing pattern | Adding, removing or changing a pattern; lowering its tier |
| `policy.skips` | Removing a skip | Adding a skip |
| `policy.merge_waived_tiers` | Removing a tier | Adding a tier |
| `policy.required_checks` | Adding a check | Removing or renaming a check |
| `project_name` | None | Any change: it names the worktrees of the issues claimed after it, while the issues claimed before keep their names (section Names of a checkout) (decided by the maintainer on 2026-11-25, second counter-review N5) |

### Names of a checkout

The engine derives the branch and the worktree of an issue at its first claim and records them
in `checkout_acquired`; later claims of the issue reuse them (R20; decided by the maintainer on
2026-11-25, counter-review item 28). The branch is the output of the producer of SPEC-0001 R11
for the type, the issue and the title of its task. The type is the member `branch_type` of the
task (SPEC-0001, Task, as amended), and `feat` while the task has none, until that member is built
(decided by the maintainer on 2026-11-25; the member named `branch_type`, not `type`, because
`type` already names the artifact type of an item of `plan_observed`: decided by the maintainer
on 2026-11-25, second counter-review B10). The worktree is named `<project>-<issue>`, where
`<project>` is the name of the project (decided by the maintainer on 2026-11-25). The name is the
key `project.name` of `wayloop.toml`, given to `wayloop init`, which refuses it with
`project_name_invalid` when it is absent (reason `missing`) or not a slug of SPEC-0001 R11, one
to five words of `[a-z0-9]+` joined by single hyphens (reason `syntax`, with the slug that the
producer of SPEC-0001 R11 gives for it in `fix`, except when the producer gives its fallback
`work` because nothing of the name is left, where no `fix` is given: decided by the maintainer on
2026-11-25, second counter-review N4); the first `policy_set` records it as `project_name`,
so that a replay never depends on the configuration file (R7) (decided by the maintainer on
2026-11-25, Q21). The result of the step `claim` reports
the names the runtime prepared, and a name that differs is `result_rejected` with
`checkout_mismatch`, so that the core never trusts a name it did not derive.

### Project phase

The phase is a function of artifact statuses, evaluated in this order; the first row that
holds gives the phase (R47). Deriving it, instead of storing it, keeps it consistent with the
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
| P1 | Any | `brief_submitted` (observation) | Brief valid (SPEC-0001), in `draft` (`subject_invalid`) | Person | Same | `gate_opened` (kind `brief`) |
| P2 | Any | `approval` approving the brief gate | G2 | Person | Derived: `council` unless `delivery` holds | `approval_recorded`, `gate_resolved`, `artifact_status_changed` (brief `approved`, gate issue `done`), `phase_changed` |
| P3 | Any | `project_step_requested` (command), step `council` or `plan` | R48 applies to the subject (`project_step_not_applicable`); budget of the step kind for the brief not exhausted (`budget_exhausted`); no open project dispatch for the subject (`dispatch_open`) | Engine | Same | `step_started` (project dispatch) |
| P4 | Any | `council_report` (result) | Dispatch current; report valid, its brief open (`subject_invalid`) | Agent | Derived | `step_completed`, `gate_opened` (kind `council_report`) |
| P5 | Any | `approval` approving the council report gate | G2; every question answered (SPEC-0001) | Person | Derived: `planning` | `approval_recorded`, `gate_resolved`, `artifact_status_changed` (report `approved`, gate issue `done`; second counter-review check N-S3), `phase_changed` |
| P6 | Any | `plan_proposal` (result) | Dispatch current; every item valid (SPEC-0001), at least one milestone, from the approved report, its brief open (`subject_invalid`) | Agent | Derived | `step_completed`, `gate_opened` (kind `roadmap`) |
| P7 | Any | `approval` approving a `roadmap` or `roadmap_change` gate that approves a milestone, carrying the content of the proposal | G2; the SHA-256 of the content carried equals the `proposal` of the gate (`proposal_hash_mismatch`) | Person | Derived: `delivery` | `approval_recorded`, `gate_resolved`, `artifact_status_changed` for each item, `phase_changed` |
| P8 | Any | `approval` rejecting a `council_report` or `roadmap` gate | G3 | Person | Derived | `approval_recorded`, `gate_resolved` (`rejected`), `artifact_status_changed` (report or plan items to `superseded`; for a `council_report` gate, its gate issue to `superseded`, second counter-review check N-S3); stop `rejected_awaiting_input` (R12) |
| P9 | Any | `unblock_requested` on a brief with a rejected report or plan | Actor is an approver (`actor_not_authorised`) | Person | Same | `subject_resumed` naming the brief; R48 applies again |
| P10 | `delivery` | `status_change_requested`: milestone to `done` (gate met) | SPEC-0001 transition allowed (`transition_refused`); actor is an approver | Person | Derived: `complete` when no milestone remains `approved` or `implementing` | `artifact_status_changed`, `phase_changed` |
| P11 | Any | `status_change_requested`: a brief or council report to `superseded` | SPEC-0001 transition allowed (`transition_refused`); actor is an approver | Person | Derived | `artifact_status_changed`, `gate_withdrawn` for its gates, `phase_changed` |
| P12 | Any | `project_step_requested`, step `reconcile`, then `roadmap_change_proposal` (result) | For a reconciliation that belongs to no milestone, a sprint is active (`no_active_sprint`; the request is refused and nothing is recorded (decided by the maintainer on 2026-11-25)); dispatch current; change valid against the current plan (`plan_changed`) | Agent | Same | `step_started`; then `step_completed`, `gate_opened` (kind `roadmap_change`) |
| P13 | Any | `pull_request_merged` (observation) of a pull request merged into the default branch that is not the closing pull request of a non-terminal planned issue (a pull request without issue, of a gate issue or of a dependency update, or one whose issue is `done` or `superseded`); or `default_branch_pushed` (observation) | None | Any (fact) | Same | `pull_request_merged` for a merge; `coverage_stale` for each unit of R51; `reconciliation_required` (`merged_after_end`) when the issue is `done` or `superseded` (I36); `input_ignored` (`no_effect`) when nothing else is recorded |

P4 and P6 run from any phase, because P3 runs in any phase and `delivery` takes priority in the
derivation: a second brief approved during `delivery` gets its report and its plan (decided by
the maintainer on 2026-11-25, counter-review item 21). P9 records `subject_resumed`, a kind of
its own, because `issue_unblocked` removes a hold of an issue and a brief has no hold (decided by
the maintainer on 2026-11-25, counter-review item 23). An approval or an answer on the tracker
that fails a guard of a project transition is recorded as `input_ignored` with the code of the
guard (G6), because an observation is never refused; an observed status change that SPEC-0001
does not allow is recorded the same way with `transition_refused` (decided by the maintainer on
2026-11-25, counter-review item 64).

The questions of a council report are answered by an `answer` that names the report gate and
the identifier `Q<k>` of the question; it may wait until the phase machine is built (decided by
the maintainer on 2026-11-25, counter-review item 24).

Reconciliation (SPEC-0007 R3) runs in any phase. Its proposal opens a gate of kind
`roadmap_change`, and the change is applied by the `gate_resolved` that approves it, through the
`artifact_status_changed` events it records (SPEC-0007 R4). The survey (SPEC-0010, SPEC-0011)
may run in any phase and changes no phase by itself.

P13 keeps the coverage true for every change of the default branch, not only for planned issues
(R51, SPEC-0003 R16, Q12 of SPEC-0003). A push to the default branch is covered by the same rule
when the git adapter observes it as `default_branch_pushed`; a push it does not observe makes no
unit stale, which the guide of the adapter states as a limit.

### Gates and approval states

A gate is a pending decision of a person (constitution IV, ADR 0012). The loop cannot pass it:
no transition resolves a gate without an input from a recorded approver (R14), and `next_steps`
proposes no action that depends on a pending gate (R13). The identity of a gate is the hash of
its `gate_opened` event.

Approval states:

| Gate kind | Opened when | Subject guards checked on approval | Effect of `approved` | Effect of `rejected` |
| --- | --- | --- | --- | --- |
| `brief` | A brief is submitted (P1) | Brief valid (`subject_invalid`) | Brief `approved` (P2) | Brief stays `draft`; its gate issue moves to `superseded`; a new submission opens a new gate |
| `council_report` | A report is returned (P4) | Every question answered (`question_unanswered`) | Report `approved` (P5) | Report `superseded`; stop `rejected_awaiting_input` (P8) |
| `roadmap` | A plan proposal is returned (P6) | Every item valid (`subject_invalid`); the content carried matches the proposal (`proposal_hash_mismatch`) | Items `approved` (P7) | Items `superseded`; stop `rejected_awaiting_input` (P8) |
| `roadmap_change` | A reconciliation proposal is returned (P12) | Change applies to the current plan (`plan_changed`); the content carried matches the proposal (`proposal_hash_mismatch`) | Change applied (SPEC-0007 R4) | Nothing changes |
| `specification` | `specification_submitted` for a specification in `draft` or `surveyed` | Specification valid (`subject_invalid`) | Specification `approved`; `coverage_planned` with the coverage units of its active requirements (SPEC-0003 R14) | Stays as it was |
| `merge` | An issue reaches `merge` and the policy does not waive it (I16, I40) | Every required check `success` at the recorded head (`checks_not_green`) | The merge may happen; `done` follows the merge (I18) | Issue to `build`, attempt ended (I19) |
| `configuration` | A run starts with a wider configuration (R45) | The proposal still differs from the recorded configuration (`configuration_unchanged`) | `limits_set` or `policy_set` with the proposed keys on top of the recorded configuration | Recorded configuration kept |
| `question` | An agent or the engine raises a question (table Questions) | For `repeated_failure`: answer `retry` or `stop` (`option_unknown`) | Not applicable: resolution `answered` | Not applicable |

A gate of kind `specification` has no gate issue, since a gate issue names only a brief or a
council report as its subject. It is approved by the observed merge, by a recorded approver, of
the pull request that sets the specification to `Approved`: the tracker adapter reports that
merge as an `approval` of the gate, with the person who merged as actor, and G2 records
`approval_recorded` and `gate_resolved`. A merge by an account that is not a recorded approver
is `input_ignored` with `actor_not_authorised`, and the gate stays pending (decided by the
maintainer on 2026-11-25, Q14). It is outside the slice of `plan.md`.

Gate transitions:

| # | From | Input | Guard | Actor | To | Recorded |
| --- | --- | --- | --- | --- | --- | --- |
| G1 | (none) | Subject reaches an approval state | None | Engine | `pending` | `gate_opened` |
| G2 | `pending` | `approval` with decision `approve` | Actor is a recorded approver (`actor_not_authorised`, R14); subject guards (R15) | Person | `resolved`, resolution `approved` | `approval_recorded`, `gate_resolved`, then, when the gate has a gate issue, `artifact_status_changed` (gate issue to `done`) |
| G3 | `pending` | `approval` with decision `reject` | Actor is a recorded approver (`actor_not_authorised`) | Person | `resolved`, resolution `rejected` | `approval_recorded`, `gate_resolved`, then, when the gate has a gate issue, `artifact_status_changed` (gate issue to `superseded`) |
| G4 | `pending`, kind `question` | `answer` | Actor is a recorded approver (`actor_not_authorised`); for `repeated_failure`, the answer is an option (`option_unknown`) | Person | `resolved`, resolution `answered` | `question_answered`, `gate_resolved` |
| G5 | `pending` | Subject superseded; new head, failed required check or close of the pull request of a merge gate; issue `done` through a bypass; a new run for a configuration gate | None | Engine, on the trigger | `withdrawn` | `gate_withdrawn` with the reason; when the gate has a gate issue in `draft`, `artifact_status_changed` (gate issue to `superseded`); when the subject is superseded and has a replacement, `gate_opened` for the replacement subject |
| G6 | `pending` | `approval` or `answer` failing G2 to G4 | None | Any | `pending` | `input_ignored` with the code of the failure |
| G7 | (none) | Issue of tier `small` reaches `merge`; policy waives the merge gate | Tier computed by the engine (R25) | Engine, under the policy | No gate: waived | `gate_waived` with the policy revision and its approver |

`gate_state` is `pending`, `resolved` or `withdrawn`; `resolution` is `approved`, `rejected` or
`answered`. A gate issue of SPEC-0001 is `done` when its gate is `approved`, and `superseded` when
its gate is withdrawn or rejected (decided by the maintainer on 2026-11-25, counter-review item
25; SPEC-0001 Gate issue, as amended); G2, G3 and G5 record that transition in their column
Recorded, so that AC54 compares complete rows (second counter-review S6). When the subject of a gate is superseded, G5 opens a new
gate for the replacement subject, as SPEC-0001 requires: the gate issue of a gate in `draft`
becomes `superseded` with `superseded_by` naming the new gate, and a gate issue in `done` stays
`done` (decided by the maintainer on 2026-11-25, counter-review item 25). How a person expresses
an approval or an answer on the tracker is decided by SPEC-0007; this machine consumes the
normalised observation.

Only an `approval` resolves the merge gate. A `pull_request_review_submitted` with the state
`changes_requested` at the recorded head, from an approver, rejects it (I19); any other review
has no effect and is recorded as `input_ignored` with `no_effect` (decided by the maintainer on
2026-11-25, counter-review item 47).

The merge waiver (G7) follows ADR 0029: for the tier `small`, merge is not an approval state
when the recorded policy waives it. The person accountable is the approver of the policy
revision that holds the waiver, and `gate_waived` names both. A waiver is never recorded as
`gate_resolved` by a person, and a policy change that widens `small` or adds a waiver passes the
`configuration` gate (R45).

### Sessions

A session is opened by `session_started` and closed by `session_ended` (SPEC-0003); it holds zero
or more checkouts. Session transitions:

| # | From | Input (class) | Guard | Actor | To | Recorded |
| --- | --- | --- | --- | --- | --- | --- |
| S1 | (none) | `session_start_requested` (command) | None | Runtime or agent | Open | `session_started` with the identifier of SPEC-0003 R47 |
| S2 | Open | `session_end_requested` (command) with a hand-over | Selector: the input carries a hand-over. Sender is the holder of the session (`checkout_not_held`); the session holds a checkout (`checkout_not_held`); hand-over valid (SPEC-0001 R22, `handover_input`) and within its budget (`handover_budget`) | Agent | Ended | `handover_recorded`, `checkout_released` (`handed_over`) for each checkout of the session by issue number, `session_ended` (`handed_over`) |
| S3 | Open | `session_end_requested` (command) without a hand-over | Selector: the input carries no hand-over. Sender is the holder of the session (`checkout_not_held`); the session holds no checkout, since a session that holds one ends only with a hand-over (`handover_input`, reason `missing`, at the pointer of the hand-over) | Agent | Ended | `session_ended` (`completed`) |
| S4 | Open | `lease_renewal_requested` (command, no revision) | Sender is the holder of the session (`checkout_not_held`); the session holds a checkout (`checkout_not_held`) | Runtime of the holder | Open, lease extended | `lease_renewed` |
| S5 | Ended `abandoned` | `handover_submitted` (command) by its last holder, for an issue it held | The lease of the session ended; no other session has claimed the issue since (`checkout_not_held`); hand-over valid and within its budget | Agent | Ended | `handover_recorded` with `late` `true` |
| S6 | Open | Due: the lease of the session ended (`due_events`), whether or not it holds a checkout | `now` not earlier than the end of the lease | Engine | Ended `abandoned` | For each checkout of the session, by issue number, the events of I31; then `session_ended` (`abandoned`) |

S3 lets a session that never held a checkout, or released every one before, end as `completed`
(decided by the maintainer on 2026-11-25, counter-review item 41). S5 is the late hand-over of
SPEC-0003 R12 as a row with its guards (decided by the maintainer on 2026-11-25, counter-review
item 42). S4 carries no revision: it applies to the session, not to an issue, so that an
observation on an issue during a step never makes a heartbeat stale (decided by the maintainer on
2026-11-25, counter-review item 60). Every heartbeat is a new renewal and extends the lease,
although its content, and so its `input_hash`, repeats: the retry rule of SPEC-0003 R52 does not
apply to `lease_renewal_requested` (decided by the maintainer on 2026-11-25, second counter-review
B8). The From column of S2 to S4 names only that the session is open; whether it holds a checkout
is a guard with a code (R6), so that every refusal has one code (second counter-review S4). S6
ends a session at the end of its lease whether or not it holds a checkout: a session that holds
none, such as one after I44, ends as `abandoned` `limits.lease_minutes` minutes after its latest
event that extends a lease (SPEC-0003 R46), so that open sessions do not pile up (decided by the
maintainer on 2026-11-25, second counter-review B12).

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
| `review` | Verifiers by risk tier, one dispatch per lens, none of them the implementer or the counter-reviewer (R8, R11, R49) | Agent | Yes |
| `pull_request` | The pull request is opened or updated, then its required checks run | Engine | No |
| `merge` | Approval state: the merge gate is pending, approved or waived; the merge is awaited | Person, or the engine under a waiver (ADR 0029) | No |
| `done` | Closing pull request merged; terminal | None | No |
| `superseded` | Task superseded; terminal | None | No |

The values from `claim` to `merge` are the identifiers of `IssueStep` in `crates/wayloop-schemas`,
re-exported by `wayloop-core`, the `step` values of the hand-over (SPEC-0001). `issue_state` is
built from `IssueStep` plus `open`, `done` and `superseded` by one helper of the module `contract`
that writes the schema from `IssueStep::ALL`, so that the steps are listed once (decided by the
maintainer on 2026-11-25, counter-review item 66). The descriptions of the steps in
`IssueStep` follow the column Meaning of this table; the description of `merge` there still says
that only a person merges, against ADR 0029, and is corrected by task T1 of `tasks.md`.

Issue transitions. `X` is any state from `claim` to `pull_request`; `N` is any non-terminal
state. The column From names issue states only, with the conditions on the pull request and the
dispatch that pick the row; a condition on the checkout is a guard (R6, second counter-review
B2). A row without a checkout guard applies whether or not the issue is checked out. A condition
written "Selector:" chooses between rows of the same source state and input and has no code
(R55, second counter-review B6). Row I0 is evaluated before every other row (R33). Every result row has, before its own guards, the guards
"dispatch current" (`dispatch_stale`, R21) and "a kind and an outcome that the step of the
dispatch accepts" (`result_kind_mismatch`, R18); a result row whose own guard fails records
`result_rejected` with the code of the guard under I28 or I29. The column Recorded gives the
events in the order they are appended (R43 for the artifact transitions).

| # | From | Input (class) | Guard | Actor | To | Recorded |
| --- | --- | --- | --- | --- | --- | --- |
| I0 | Any | Any observation whose `external_id` is recorded | None | None | Unchanged | Nothing |
| I1 | `open` | `claim_requested` (command) | The session is open (`session_not_open`); ready (R19) | Engine | `claim`, checked out | `checkout_acquired` with the names of section Names of a checkout, then the artifact transitions of R43 |
| I2 | `N` except `open` | `claim_requested` (command) | The session is open (`session_not_open`); ready (R19), which refuses an issue checked out by another session with `checkout_held` and an issue at `merge` with `awaiting_external` (second counter-review B2 and N3) | Engine | Same state, checked out | `checkout_acquired`, task to `implementing` if `approved` |
| I3 | `X` | `step_requested` (command) | Selector: not I4 and not I46. Sender holds the checkout of the issue (`checkout_not_held`); the step is the current state (`step_mismatch`); no open dispatch (`dispatch_open`); no hold (`issue_on_hold`); for `review`: the lens is the next lens R49 requires (`lens_not_expected`), the verifier ran neither `build` nor `counter_review` of the issue and is not the verifier of an earlier lens of the step (`verifier_not_independent`, R49), and the head of `changed_paths` equals `verified_head`, whatever the order in which the two were observed (`head_not_observed`), so that SPEC-0003 R17 has a head to compare with; no budget scope of the issue exhausted, except for a dispatch of `review` covered by the verification reserve (`budget_exhausted`, R29) | Engine | Same state, dispatch open | `step_started` |
| I4 | `X` | `step_requested` (command) | Selector: an executor step, turns at least the limit (R28), not a dispatch of `review` covered by the reserve, and not I46. The guards of I3 | Engine | Same state, hold `turn_limit` | `issue_blocked` |
| I5 | `claim` | `step_result` (result) of `claim` | Result reports the branch and worktree names of the checkout (`checkout_mismatch`) | Engine | `reanalysis` | `step_completed` |
| I6 | `reanalysis` | `step_result`, outcome `completed` | Result valid against the schema of its step (`result_schema`, R23; second counter-review B7) | Agent | `counter_review`, or `checklist` when the tier skips it (R25) | `executor_call_recorded`, `step_completed` |
| I7 | `counter_review` | `step_result`, outcome `completed` | As I6; every objection ruled (`objection_unruled`, R24) | Agent | `checklist` | `executor_call_recorded`, `step_completed` |
| I8 | `checklist` | `step_result`, outcome `completed` | As I6; checklist names every cited requirement (`checklist_incomplete`, R24) | Agent | `build` | `executor_call_recorded`, `step_completed` |
| I9 | `build` | `step_result`, outcome `completed` | As I6 | Agent | `verify` | `executor_call_recorded`, `step_completed` |
| I10 | `verify` | `verification_result` (result), every check passed, naming the head it verified | None; the target depends on the tier at that head (R25), within this one row | Engine | `pull_request` when the tier skips `review` (R25) and the changed paths at that head are recorded; otherwise `review`; in both, `verified_head` set to that head, `pull_request_pass_completed` cleared when it moves to `pull_request` (decided by the maintainer on 2026-11-25) | `step_completed` with `head_sha`; the counts of the passing checks reset in `failures` (R27); then I39 at once when a pull request is recorded at a head other than `verified_head` and `head_pending` is false (final check 2) |
| I11 | `verify` | `verification_result`, a check failed | Selector: limits of R27 not reached | Engine | `build`, attempt plus one | `attempt_failed` (`verification`) |
| I12 | `verify`, `review`, `pull_request`, `merge` | A failure that ends an attempt under R26 (I11, I14, I19, I20) | Selector: limits of R27 reached; the guards of the failing row | Engine | `build`, hold `question` | The events of the failing row up to `attempt_failed`, then `gate_opened` (question `repeated_failure`), `issue_blocked` |
| I13 | `review` | `step_result` of a lens dispatch, outcome `completed` | Selector: a lens of R49 has not voted yet, or the tally of R9 gives `covered` for every unit. As I6; one vote of the lens per coverage unit (`vote_missing`, R24); the evidence is valid (`verdict_commit_stale`, `verdict_tree_dirty`, SPEC-0003 R17) | Agent | `review` while a lens of R49 has not voted; otherwise `pull_request` when the tally of R9 gives `covered` for every unit, `pull_request_pass_completed` cleared when it moves to `pull_request` (decided by the maintainer on 2026-11-25) | `executor_call_recorded`, `coverage_verdict` for each vote by unit identifier, `step_completed`; at `pull_request` then I39 at once when a pull request is recorded at a head other than `verified_head` and `head_pending` is false (final check 2) |
| I14 | `review` | As I13, the last lens | Selector: the tally gives a unit `uncovered`; limits of R27 not reached. The guards of I13 | Agent | `build`, attempt plus one | `executor_call_recorded`, `coverage_verdict` for each vote, `step_completed` (`changes_requested`), `attempt_failed` (`review`) |
| I15 | `pull_request`, no pull request recorded | `pull_request_opened` (observation) at the head `verified_head` | Branch of the issue and body that closes the issue (SPEC-0001 R10), otherwise hold `reconciliation_required` (`pull_request_unlinked`) | Engine | Same state, pull request recorded; the checks kept in `pending_checks` at that head move to `checks` | `pull_request_opened`; then, when no dispatch is open, the row I12, I16, I17, I20 or I40 that the recorded checks select (I12 added by the maintainer on 2026-11-25, second counter-review check N-S5) |
| I16 | `pull_request`, pull request recorded, no dispatch | `pull_request_checks_completed` (observation) at the recorded head | Selector: after the run is applied to `checks`, every required check is `success` there (second counter-review B3); the recorded head equals `verified_head`, otherwise I39 (final check, decided by the maintainer on 2026-11-25); the tier is not waived (G7) | Engine | `merge` | `pull_request_checks_completed`, `gate_opened` (kind `merge`) |
| I17 | As I16 | As I16 | Selector: as I16, and the tier is `small` and the policy waives the merge gate | Engine | `merge` | `pull_request_checks_completed`, `gate_waived` |
| I18 | `merge` | `pull_request_merged` (observation) | Selector: merge gate `approved` or waived | Person or engine | `done`, holds cleared | `pull_request_merged`, `issue_done`, task to `done` and the transitions of R43, `coverage_stale` for each unit of R51 |
| I19 | `merge` | `approval` rejecting the merge gate, or `pull_request_review_submitted` with the state `changes_requested` at the recorded head | Selector: limits of R27 not reached. Actor or reviewer is a recorded approver (`actor_not_authorised`) | Person | `build`, pull request kept, attempt plus one | `pull_request_review_submitted` when it is the input, `approval_recorded`, `gate_resolved` (`rejected`), `attempt_failed` (`merge_rejected`) |
| I20 | `pull_request` with no dispatch, or `merge`, pull request recorded | `pull_request_checks_completed` at the recorded head | Selector: after the run is applied to `checks`, a required check there is `failure` or `timed_out` (second counter-review B3); limits of R27 not reached | Engine | `build`, attempt plus one, same branch, `head_pending` | `pull_request_checks_completed`, `gate_withdrawn` (`checks_failed`) at `merge`, `attempt_failed` (`pull_request_checks`) |
| I21 | `pull_request`, dispatch open, pull request recorded | `pull_request_synchronized` (observation) at the branch of the issue, at the head `verified_head` | New head | Engine | Same state, head recorded, `checks` replaced by the checks kept in `pending_checks` at that head, `head_pending` false | `pull_request_synchronized`; the recorded checks are applied by I44 when the dispatch closes |
| I22 | `pull_request` without dispatch, or `merge` | `pull_request_synchronized` at the head `verified_head`; a new head other than `verified_head` follows I39, back to `verify` (decided by the maintainer on 2026-11-25, Q19) | New head | Any (fact) | `pull_request`, head recorded, `head_pending` false, `checks` replaced by the conclusions kept in `pending_checks` at that head (second counter-review B4) | `pull_request_synchronized`, `gate_withdrawn` (`head_changed`) for the merge gate; then the row I12, I16, I17, I20 or I40 that the new `checks` select, when they select one (I12 added by the maintainer on 2026-11-25, second counter-review check N-S5) |
| I23 | `N` with a pull request | `pull_request_checks_completed` or `pull_request_review_submitted` at a head in `heads` other than the recorded head | None | Any (fact) | Unchanged | `input_ignored` (`head_stale`) |
| I24 | `N` | `pull_request_merged` of the closing pull request | Selector: not I18, the merge gate neither `approved` nor waived | Any (fact) | `done`, holds cleared | `pull_request_merged`, `issue_done`, `gate_bypassed`, `gate_withdrawn` for pending gates, `checkout_released` (`merged`) if checked out, `reconciliation_required` (`merged_without_approval`), task to `done` from `implementing` or `approved`, `coverage_stale` for each unit of R51 |
| I25 | `N` with a pull request | `pull_request_closed` (observation) | None | Any (fact) | Same state, pull request cleared and kept in `closed_pull_request` (decided by the maintainer on 2026-11-25, final check 2; criterion: stability and maintainability), hold `pull_request_closed` | `pull_request_closed`, `gate_withdrawn` (`pull_request_closed`), `checkout_released` (`pull_request_closed`) if checked out, `issue_blocked` |
| I26 | `N`, held with `pull_request_closed` | `pull_request_reopened` (observation) of the number of `closed_pull_request` | None | Any (fact) | `closed_pull_request` restored as `pull_request` with its head, `heads`, `checks` and `head_pending` as recorded at the close, and dropped; hold removed; `pull_request` when the issue is at `pull_request` or `merge` at that moment, otherwise the state it is in (R31, R36) (final check, decided by the maintainer on 2026-11-25); the state at the reopen (decided by the maintainer on 2026-11-25, final check 2; criterion: stability and maintainability) | `pull_request_reopened`, then `step_completed` of `review` and the move of R49 while `no_unit_acknowledged` is set and no hold remains, `issue_unblocked` when no hold remains; then, at `pull_request` with no dispatch, the row I12, I16, I17, I20, I39, I40 or I51 that the restored pull request and its checks select, as I15, I22 and I44 do (decided by the maintainer on 2026-11-25, final check 2; criterion: stability and maintainability) |
| I27 | An executor step with a dispatch | `step_result`, outcome `question_raised` | Question valid: context, options with cost, recommendation (`question_invalid`, SPEC-0007 R1) | Agent | Same state, hold `question` | `executor_call_recorded`, `step_completed` (`question_raised`), `gate_opened` (kind `question`), `issue_blocked` |
| I28 | Any state with a dispatch | `step_result` invalid or failing a guard, or `executor_failure` (result) | Selector: retries below `limits.executor_retries`; for a dispatch of `review` of tier `contract`, instead, the repetition of the panel not yet used (R8, Q12) | Engine | Same state, dispatch closed, retries plus one | `executor_call_recorded` when carried, `result_rejected` or `step_retried` |
| I29 | As I28 | As I28 | Selector: retries at least `limits.executor_retries`; for a dispatch of `review` of tier `contract`, instead, the repetition of the panel already used (R8, Q12) | Engine | Same state, dispatch closed, hold `executor_failure` | `executor_call_recorded` when carried, `result_rejected` when the output is invalid, `step_failed`, `issue_blocked` |
| I30 | `N` | `session_end_requested` with a hand-over (S2) | As S2, for each issue that the session holds | Agent | Same state, not checked out | The events of S2 |
| I31 | `N` | Due: the lease of the session ended (`due_events`), for each issue that the session holds (S6) | `now` not earlier than the end of the lease | Engine | Same state, not checked out, dispatch closed | `checkout_released` (`lease_expired`), task to `approved` while `pull_request_step_completed` is false (second counter-review B1, check N-S2); after the releases of the session, `session_ended` (`abandoned`) of S6 |
| I32 | `N`, held | Input of the table Holds for a reason of the issue, other than the observations that their own rows decide (I26 for a reopen, I43 and I48 for a withdrawal with `review_left`), so that one input selects one row (decided by the maintainer on 2026-11-25, final check 2; criterion: stability and maintainability) | As the table says | Person or engine | That hold removed; the target of the table Holds, computed from the state at that moment; `pull_request` when it was the last and `no_unit_acknowledged` is set (R31, R49) (decided by the maintainer on 2026-11-25, final check 2; criterion: stability and maintainability) | Its effects on counters (table Holds), then, when no hold remains, `step_completed` of `review` while `no_unit_acknowledged` is set, and `issue_unblocked` |
| I33 | `N` | `stop_requested` (observation) | Actor is an approver (`actor_not_authorised`) | Person | Same state, hold `stopped_by_person` | `issue_blocked` |
| I34 | `N` | `status_change_requested`: task to `superseded` (observation), or task expanded (SPEC-0013 R3) | SPEC-0001 transition allowed (`transition_refused`); actor is an approver (`actor_not_authorised`), or the expansion was approved through a `roadmap_change` gate | Person | `superseded`, dispatch closed, holds cleared | `issue_superseded`, `checkout_released` (`superseded`) if checked out, `gate_withdrawn` for each pending gate |
| I35 | `N` | `issue_closed` (observation) not marked as caused by the closing pull request | None | Any (fact) | Same state, hold `reconciliation_required` | `reconciliation_required` (`issue_closed_without_merge`), `issue_blocked` |
| I36 | `done`, `superseded` | Any observation of the issue not consistent with it, `issue_reopened` included; a `pull_request_merged` follows P13, which records the same event | None | Any (fact) | Unchanged | `reconciliation_required` (`issue_reopened` or `merged_after_end` or `observation_inconsistent`) |
| I37 | `N` | `tracker_block_invalid` (observation) naming the issue | None | Engine | Same state, hold `question` | `gate_opened` (question `invalid_tracker_block`), `issue_blocked` |
| I38 | `review` | As I13, the last lens | Selector: the tally gives no unit `uncovered` and a unit `needs_validation`. The guards of I13 | Agent | Same state, hold `question` | `executor_call_recorded`, `coverage_verdict` for each vote, `step_completed`, `gate_opened` (question `needs_validation`), `issue_blocked` |
| I39 | `pull_request`, or `merge` (Q19) | `pull_request_opened` or `pull_request_synchronized` at the branch of the issue, at a head other than `verified_head`; or, at `pull_request` with a pull request recorded and no dispatch, `pull_request_checks_completed` at a recorded head other than `verified_head` after which every required check is `success` in `checks`, so that no row opens or waives the merge gate on a head `verify` never checked (final check, decided by the maintainer on 2026-11-25); with a policy that lists no required check, every check run at such a head selects this row, which is intended (final check 2); or, at once, the issue reaching `pull_request` with no dispatch (I10, I13, I48, the move of R49, I26) or the close of its dispatch (I44) while a pull request is recorded at a head other than `verified_head` and `head_pending` is false, so that no issue waits for an observation of a head already observed (decided by the maintainer on 2026-11-25, final check 2; criterion: stability and maintainability) | None | Any (fact) | `verify`, pull request recorded at that head, dispatch closed, no attempt counted, `verified_head` cleared | The observed event (`pull_request_opened`, `pull_request_synchronized` or `pull_request_checks_completed`), or, when applied at once, nothing beyond the events of the row that led to it; at `merge`, `gate_withdrawn` (`head_changed`) |
| I40 | `pull_request`, no dispatch, the recorded policy lists no required check | `pull_request_opened` or `pull_request_synchronized` at the head `verified_head`, or the `step_result` of I44 once the pull request is recorded at that head | Selector: the tier waived or not (G7) picks the event | Engine | `merge` | The events of the input, then `gate_opened` (kind `merge`) or `gate_waived` |
| I41 | `build`, `verify`, `review` with a pull request | `pull_request_synchronized` at the branch of the issue | New head; at `review`, the head equals `verified_head` (otherwise I43) | Any (fact) | Same state, head recorded, `checks` replaced by the conclusions kept in `pending_checks` at that head (B4) | `pull_request_synchronized` |
| I42 | `N` other than `pull_request`, with a pull request | `pull_request_checks_completed` at the recorded head, with any conclusions | Selector: not at `merge`, or at `merge` with no required check `failure` or `timed_out` in `checks` after the run is applied (a failure at `merge` follows I20) | Any (fact) | Same state, `checks` updated; no `attempt_failed` and no count of R27 changes (decided by the maintainer on 2026-11-25, second counter-review B3) | `pull_request_checks_completed` |
| I43 | `review` | `branch_changes_observed` or `pull_request_synchronized` at a head other than `verified_head` | None | Any (fact) | `verify`, dispatch closed, `verified_head` cleared, no attempt counted | The observed event; when the question of I46 is pending, `gate_withdrawn` (`review_left`) for it, its hold removed, and `issue_unblocked` when no hold remains (final check, decided by the maintainer on 2026-11-25) |
| I44 | `pull_request`, dispatch open | `step_result` of `pull_request`, outcome `completed` | None | Engine | Same state, dispatch closed, not checked out, `pull_request_step_completed` set (check N-S2) and `pull_request_pass_completed` set (decided by the maintainer on 2026-11-25) | `step_completed`, `checkout_released` (`awaiting_checks`) if checked out; then, when the pull request is recorded, its recorded head equals `verified_head` and `head_pending` is false, the row I12, I16, I17, I20 or I40 that the checks recorded at its head select; when the pull request is recorded at another head and `head_pending` is false, I39 at once (decided by the maintainer on 2026-11-25, final check 2; criterion: stability and maintainability); otherwise no row of the checks, and I15, I22 or I39 decides when the head is observed (decided by the maintainer on 2026-11-25, Q15 and Q20; the condition on the head decided by the maintainer on 2026-11-25, second counter-review B5) |
| I45 | `N` | `issue_closed` (observation) marked as caused by the closing pull request | None | Any (fact) | Unchanged | `input_ignored` (`no_effect`) |
| I46 | `review` | `step_requested` for `review` | Selector: the task has no coverage unit; I46 has priority over I4, because a dispatch with no coverage unit can never pass review, so the question that only a person can answer comes before a limit that a `limits_set` lifts (decided by the maintainer on 2026-11-25, second counter-review check N-S1). The guards of I3 | Engine | Same state, hold `question`; the answer leads to `pull_request` under R49 (S9) | `gate_opened` (question `needs_validation`, naming the missing coverage units), `issue_blocked` |
| I47 | `N` with a pull request recorded | `pull_request_checks_completed` at a head of the pull request not in `heads` | None | Any (fact) | Unchanged; conclusions kept in `pending_checks` (second counter-review B4) | `pull_request_checks_completed` |
| I48 | `review`, no dispatch | `branch_changes_observed` at `verified_head` | Selector: the tier computed with these paths is `small` (R11, R25); otherwise I50 | Engine | `pull_request`, `pull_request_pass_completed` cleared (decided by the maintainer on 2026-11-25) | `branch_changes_observed`, `step_completed` (step `review`, outcome `completed`, `skipped` `[review]`) (decided by the maintainer on 2026-11-25, Q17); when the question of I46 is pending, `gate_withdrawn` (`review_left`) for it, its hold removed, and `issue_unblocked` when no hold remains (final check, decided by the maintainer on 2026-11-25); then I39 at once when a pull request is recorded at a head other than `verified_head` and `head_pending` is false (final check 2) |
| I49 | `pull_request`, dispatch open, pull request recorded | `pull_request_checks_completed` at the recorded head | None | Any (fact) | Same state, `checks` updated; applied by I44 when the dispatch closes | `pull_request_checks_completed` (decided by the maintainer on 2026-11-25, Q20) |
| I50 | `N` | `branch_changes_observed` at the branch of the issue, other than the cases of I43 and I48 | None | Any (fact) | Same state, `changed_paths` recorded with their head | `branch_changes_observed` (the recording of section Configuration in the state, written as a row so that AC1 enumerates it) |
| I51 | `pull_request`, pull request recorded, no dispatch | `pull_request_checks_completed` at the recorded head | Selector: neither I16, I17, I20 nor I39 (final check 2): after the run is applied to `checks`, no required check is `failure` or `timed_out` and not every one is `success`, as when a check is `cancelled` or reported by another suite | Any (fact) | Same state, `checks` updated; the issue waits for the other runs (`awaiting_checks`, R38) | `pull_request_checks_completed` (decided by the maintainer on 2026-11-25, second counter-review B3) |

Notes on the table:

- Every result that carries an executor call record records `executor_call_recorded` first,
  whatever its outcome, refused stale results included (R21): the cost of a step is never lost
  (constitution VI).
- Engine steps (`claim`, `verify`, `pull_request`) are dispatched like executor steps (I3) so
  that every result names its dispatch; they count no turn. The dispatch of `pull_request` is
  closed by its own result (I44), not by an observation, so that a step with nothing to push
  still ends (decided by the maintainer on 2026-11-25, counter-review item 35).
- A pull request opened or updated at a head other than `verified_head` sends the issue back to
  `verify` (I39), so that no unverified commit reaches review or merge (decided by the maintainer
  on 2026-11-25, counter-review item 34). After a failed required check the pull request stays
  open while the agent rebuilds; a push during `build`, `verify` or `review` records the new head
  and changes nothing else (I41), and a re-run of the checks outside `pull_request` and `merge`,
  passing or failing, updates `checks` only, with no attempt counted and no count of R27 changed
  (I42) (decided by the maintainer on 2026-11-25, counter-review items 29 and 30; any conclusion,
  second counter-review B3).
- I16, I17, I20 and I40 apply at `pull_request` only after the dispatch of the step is closed
  (I44). A check run at the recorded head that arrives while that dispatch is still open is
  recorded and kept in `checks` (I49), and I44 applies the row it selects when it closes the
  dispatch, so that no fact is lost, no invariant bends, and the order of the result and the
  observation does not matter (decided by the maintainer on 2026-11-25, Q20). I16, I17, I20, I39
  (its trigger of the final check) and I51 read `checks` after the run is applied, not the run alone, so that two suites that report
  the required checks of one head, or a `cancelled` check followed by a `success`, open the merge
  gate (second counter-review B3). I44 applies a row only while the recorded head is
  `verified_head` and `head_pending` is false, so that `verify` has checked the head that reaches
  `merge`, and a failure of an earlier head is never counted twice (second counter-review B5).
- A check run at a head not yet observed is recorded at once, so that its `external_id` is
  recorded, and applied when that head is observed (I47, I15, I21, I22, I39, I41, in any
  non-terminal state, field `pending_checks`); a check run or a review at a
  head the pull request had before is stale (I23) (decided by the maintainer on 2026-11-25,
  counter-review item 31).
- At `review`, a move of the branch away from `verified_head` returns the issue to `verify`
  without counting an attempt (I43), so that review never waits for paths at a head that is no
  longer the branch (decided by the maintainer on 2026-11-25, counter-review item 32).
- The skip of `review` for the tier `small` needs the changed paths at the verified head (I10);
  without them the issue goes to `review`, where I3 waits for them (counter-review item 33).
  When they arrive and the tier is `small`, I48 moves the issue to `pull_request` and records the
  skip, so that no retry is spent on the order of the adapters and R11 holds (decided by the
  maintainer on 2026-11-25, Q17).
- A new head pushed outside a step, at `pull_request` without dispatch or at `merge`, follows I22
  only when it is `verified_head`; any other head returns the issue to `verify` by I39, so that
  no head that `verify` never checked reaches `merge` (decided by the maintainer on 2026-11-25,
  Q19).
- After a release (I30, I31, I44) the issue keeps its state, so that a new claim (I2) resumes
  where the work stopped and the brief starts from the resume pack (SPEC-0003 R44). Once a
  `step_completed` of `pull_request` is recorded for the issue (`pull_request_step_completed`)
  the task stays `implementing`
  (R22, R35), also after the pull request is closed and cleared (I25) and the issue is claimed
  again (second counter-review B1).
- An issue at `merge`, or at `pull_request` with `pull_request_pass_completed` set and before a
  check fails, needs a person or the tracker, not an agent: it is not ready (R19). An issue at
  `merge` is not checked out and still occupies its lane.
- I11 and I20 differ on the branch. A failed verification before the pull request ends the
  attempt; the runtime discards the branch and worktree (SPEC-0006 R8). A failed required check
  after the pull request is opened keeps the branch, so that the pull request stays open; the
  next `pull_request` step pushes a new head (I21).
- A hold blocks only new dispatches (R30): a result of a dispatch already open is decided by its
  row, and the issue stays held (decided by the maintainer on 2026-11-25, counter-review item
  38).
- `done` and `superseded` clear every hold (I18, I24, I34), and an `issue_closed` caused by the
  closing pull request has no effect (I45), so that a close observed before the merge never
  leaves a hold on a finished issue (decided by the maintainer on 2026-11-25, counter-review item
  39).

### Holds

A hold is the `blocked` state of the conventions page: the issue keeps its state and cannot
start a new dispatch until every hold is removed (R30, R31). Holds survive a release: a held
issue is not ready, whoever holds its checkout. Holds form a set keyed by `(reason, gate)`; the
same key is never held twice. The effects of a removal on counters, and its target, apply when
that hold is removed, not when the input that asks for it arrives (decided by the maintainer on
2026-11-25, counter-review item 37). The target is computed at the removal from the state the
issue is in at that moment, since rows may move a held issue (R30: an open dispatch is decided as
usual; I43 and I50 apply to a held issue); no target is stored when the hold is entered, and a
removal without a target leaves the issue where it is (R31; decided by the maintainer on 2026-11-25, final check 2; criterion: stability and maintainability).

| Reason | Entered by | Removed by | A person may remove it with `unblock_requested` | Target at removal (from the state at that moment), and effects |
| --- | --- | --- | --- | --- |
| `question` | I12, I27, I37, I38, I46 | `answer` on its question gate by an approver (G4); for `invalid_tracker_block`, also a later valid `plan_observed` of the item, which withdraws the gate; for the question of I46, also any move of the issue out of `review` (I43, I48), which withdraws the gate with `review_left` (final check, decided by the maintainer on 2026-11-25) | No | None; for `repeated_failure`: answer `retry` sets `attempt` to 1, resets `failures` and moves the issue to `build`; answer `stop` replaces the hold with `stopped_by_person`; for `needs_validation` of I38: when the issue is at `review`, `review` again from its first lens, otherwise none; for a withdrawal with `review_left`: none, the row that withdrew it (I43, I48) sets the state (decided by the maintainer on 2026-11-25, final check 2; criterion: stability and maintainability); for `needs_validation` of I46, raised because the task has no coverage unit: `pull_request`, and the removal records `step_completed` (step `review`, outcome `completed`) after `question_answered`, which is the acknowledgement of the approver (decided by the maintainer on 2026-11-25, second counter-review S9); the answer sets `no_unit_acknowledged`, and when another hold remains, the `step_completed` and the move to `pull_request` are recorded when the last hold is removed, whatever the target of that removal, and the question is not asked again (decided by the maintainer on 2026-11-25, second counter-review check N-S2) |
| `turn_limit` | I4 | `limits_set` with `limits.turns_per_issue` above `turns` (R28) | No | None |
| `executor_failure` | I29 | `unblock_requested` by an approver | Yes | None; retries and the repetition of the panel reset (R23) |
| `pull_request_closed` | I25 | `pull_request_reopened` (I26); or `unblock_requested` by an approver | Yes | After a reopen: `closed_pull_request` restored as the recorded pull request; `pull_request` when the issue is at `pull_request` or `merge`, otherwise none, so that a head `verify` never checked does not reach `pull_request` (final check, decided by the maintainer on 2026-11-25); then the row of I26. After an unblock: `build`, and `closed_pull_request` dropped (decided by the maintainer on 2026-11-25, final check 2; criterion: stability and maintainability) |
| `reconciliation_required` | I15, I35, and R32 for an inconsistent observation | `unblock_requested` by an approver | Yes | None |
| `stopped_by_person` | I33, or answer `stop` | `unblock_requested` by an approver | Yes | None |

An `unblock_requested` names one reason, and removes the hold of that reason; one that names no
reason removes every hold of the issue that a person may remove. When holds remain,
`issue_unblocked` is not recorded, but the target of each removal applies at once (R31).

Questions (`question_category`):

| Category | Raised by | Answer | Effect of the answer |
| --- | --- | --- | --- |
| `decision`, `irreversible_action`, `security_sensitive_action`, `outside_worktree`, `broken_plan`, `methodology_choice` | Agent, with context, options with cost and a recommendation (SPEC-0007 R1) | Free text; it may name an option | None on the state; the answer reaches the next brief |
| `repeated_failure` | Engine (R27) | One of the fixed options `retry` and `stop` | As the table Holds says |
| `needs_validation` | Engine (R49, I38, I46), naming the missing fact, the evidence gathered and who can answer (SPEC-0009 R10) | Free text | As the table Holds says; for I38, the answer reaches the brief of the next verifier; for I46, the answer lets the issue go to `pull_request` (S9) |
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
| The issue is a planned task issue in state `open`, from `claim` to `verify`, `review` while the head of `changed_paths` equals `verified_head`, or `pull_request` while `pull_request_pass_completed` is false or, after a failed check, before its new head is observed (`head_pending`) (decided by the maintainer on 2026-11-25) | `awaiting_external` for `merge`, for `pull_request` awaiting checks and for `review` before the changed paths at `verified_head` are observed (I3); `transition_not_allowed` for `done` and `superseded`, since no claim leaves a terminal state |
| The task is `approved` or `implementing` | `task_not_approved` |
| Its epic and milestone are `approved` or `implementing` | `plan_not_approved` |
| Every cited specification is `approved`, `implementing` or `done` | `specification_not_approved` |
| Every task in `depends_on` is `done` | `dependency_not_done` |
| No hold | `issue_on_hold` |
| No checkout | `checkout_held` |
| No other issue of its lane is in a state from `claim` to `merge` | `lane_busy`, with the issue that occupies the lane in `found` (decided by the maintainer on 2026-11-25, counter-review item 51) |
| The issue is in exactly one lane: while a sprint is active, in a lane of the sprint; without one, its area is an area of the policy | `not_in_lane` (decided by the maintainer on 2026-11-25, counter-review item 50) |
| No budget scope of the issue is exhausted | `budget_exhausted` |

Lanes. While a sprint is active, the lanes are those of the sprint in the latest
`plan_observed`, whose paths are disjoint (SPEC-0006 R4, ADR 0013), and an issue outside every
lane is not ready. Without an active sprint, each area of the recorded policy is a lane with the
paths of the area, and the issues of a lane are the issues whose task names that area. In M1
area and lane paths are prefixes, compared by code point, and two paths overlap when one is a
prefix of the other; `policy_set` is refused with `area_paths_overlap` when two areas have
overlapping paths, and an issue listed in two lanes of a sprint is recorded as
`reconciliation_required` with the code `issue_in_two_lanes` and is not ready (decided by the
maintainer on 2026-11-25, counter-review item 49). Glob patterns of `path_tiers` follow
SPEC-0021; in M1 they are prefixes too. A lane runs one issue at a time: it is the unit of
parallelism, and two issues of one lane may change the same files. An issue occupies its lane
from its first claim until it is `done` or `superseded`, including while it waits at `merge` or
under a hold (R35); an issue that was released keeps its lane and is the one claimed next. A
hold of a person may occupy a lane for long; the readiness report of the next issue names the
occupying issue (`lane_busy`, item 51).

Order. `due_events` returns, for every session whose lease ended, by session identifier, its
`checkout_released` events by issue number, each followed by the task transition of I31, then
its `session_ended`. The `summary_recorded` events of SPEC-0003 R36 and R43 follow, after the
`session_ended`, once #52 is built: until then `due_events` returns none, and AC24 is amended
when #52 lands (decided by the maintainer on 2026-11-25, counter-review item 80). `next_steps`
lists actions in this order, and stops when `free_workers` actions that need a worker are
listed:

1. `run_step` for checked-out issues with no dispatch and no hold, by issue number; their
   workers are already counted as busy by the runtime, so they need no free worker. An issue at
   `review` whose `changed_paths` are not at `verified_head` gets no action and is reported with
   `awaiting_external` until the paths of that head are observed (I3).
2. `merge` for issues where R39 holds, by issue number; it needs no worker.
3. `run_council`, `generate_plan` and `reconcile` where they apply, by subject identifier; each
   needs a free worker (decided by the maintainer on 2026-11-25, counter-review item 48).
4. `claim` for the first ready issue of each lane without an issue from `claim` to `merge`:
   lanes in the order of the sprint, or of the areas of the policy without a sprint, issues in
   the order of the lane, then by issue number.

`ready` lists, in the same order, only the issues that would get a `claim` if more workers were
free: the first ready issue of each lane that got no `claim` for lack of workers, never a second
issue of a lane (decided by the maintainer on 2026-11-25, counter-review item 48). It is followed
by the coverage units in state `stale`, by unit identifier in code point order (R50). This
machine does not dispatch their verification: the convergence run of SPEC-0009 R11 does.

### Limits and budgets

Two words have one meaning each in this specification and in SPEC-0003 (decided by the
maintainer on 2026-11-25, counter-review items 56 and 57; the name `model_turns` and the naming
rule of the two units below decided by the maintainer on 2026-11-25, with the amendment):

- A **turn** is one executor step started for an issue (I3). The model turns inside one executor
  call are a different count, `model_turns` of `executor_call_recorded` (SPEC-0003).
- **Provider tokens** are the tokens the provider of a model reports, `tokens_in` plus
  `tokens_out` of `executor_call_recorded`; the budgets of this section count them, with the type
  `ProviderTokens`. **Size tokens** are the size of a stored document, the bytes of its canonical
  JSON divided by four, rounded up (SPEC-0001 Q6); `ledger_budgets`, hand-overs and summaries
  count them, with the type `SizeTokens`. In the contracts a member that counts provider tokens is
  named `tokens_in`, `tokens_out` or `provider_tokens`, and a member named `tokens` counts size
  tokens. Neither is converted into the other.

| Limit | Key (ADR 0010) | Default | Minimum | Counts | On reaching it |
| --- | --- | --- | --- | --- | --- |
| Turns per issue | `limits.turns_per_issue` | 20 turns | 1 | Executor steps started (I3) | Hold `turn_limit` when the turns are at least the limit (I4, R28) |
| Attempts per issue | `limits.attempts` | 3 attempts | 1 | The attempt number; a failure of R26 ends an attempt | Question `repeated_failure` when the failed attempt number is at least the limit (I12, R27) |
| Same failure | `limits.same_failure` | 2 attempts | 1 | Consecutive attempts on which one check, by name, fails | Question `repeated_failure` when the count is at least the limit (I12, R27) |
| Executor retries per step | `limits.executor_retries` | 2 retries | 0 | Executor failures and rejected results of the open step; for `review` of tier `contract`, the one repetition of the panel replaces this limit (R8, Q12) | `step_failed`, hold `executor_failure` (I29, R23) |
| Lease | `limits.lease_minutes` | 120 minutes | 1 | The end of the lease, as SPEC-0003 section Clock times defines it | Release (I31, R22) |
| Budget of council steps | `budgets.council` | None | 1 provider token | Provider tokens of the executor calls of `council` steps, per brief | `budget_exhausted` (R29) |
| Budget of plan steps | `budgets.planning` | None | 1 provider token | Provider tokens of the executor calls of `plan` steps, per brief | `budget_exhausted` (R29) |
| Budget of a sprint | `budgets.sprint` | None | 1 provider token | Provider tokens of the executor calls of issues of that sprint, and of `reconcile` steps that belong to no milestone while that sprint is active, per sprint identifier (decided by the maintainer on 2026-11-25) | `budget_exhausted` (R29) |
| Budget of a milestone | `budgets.milestone` | None | 1 provider token | Provider tokens of the executor calls of issues of that milestone and of `reconcile` steps that belong to it, per milestone identifier | `budget_exhausted` (R29) |

Every limit and budget is an integer of at least its minimum, and a smaller value is refused with
`limit_range` (R53); every comparison with a limit uses "at least" (decided by the maintainer on
2026-11-25, counter-review item 13). The budget of a project step follows its step kind, not the
phase it runs in: a `council` step charges `budgets.council` and a `plan` step charges
`budgets.planning`, each counted per brief, and a `reconcile` step charges the budget of the
milestone it belongs to; no budget is added, and `budgets.intake` is removed, since no intake
step is defined (decided by the maintainer on 2026-11-25, counter-review item 22). A
reconciliation that belongs to no milestone charges `budgets.sprint` of the active sprint; with
no active sprint, the `reconcile` step is not proposed, and `next_steps` reports the stop reason
`no_active_sprint` naming its subject (decided by the maintainer on 2026-11-25, Q18 and Q22); a
`project_step_requested` for that step is refused with the code `no_active_sprint` and nothing is
recorded (P12, section Errors; decided by the maintainer on 2026-11-25). The provider tokens of
its executor calls are charged to the sprint active when each `executor_call_recorded` is
recorded, as for every call (R29); when no sprint is active at that moment, as when a
`plan_observed` ended the sprint while the dispatch was open, the call is recorded with no budget
scope, its tokens are added to `unassigned_provider_tokens` of the state, no `budget_exhausted`
follows from them, and they are visible in the projection `state.yaml` (SPEC-0003 section
Storage), where that member is written in full (final check, decided by the maintainer on 2026-11-25).

Defaults are listed in `meta.defaults_applied` (ADR 0009 C2) when `wayloop init` writes them, and
recorded by the first `limits_set`; from then on the recorded value applies (R7, R53). Budgets have
no default: every step kind has a token budget (constitution VI), so `wayloop init` writes the
values the person gives and refuses a missing one with `budget_missing` (R53). The scopes of an
issue step are its milestone and, when the issue is in a sprint, that sprint; each sprint
identifier and each milestone identifier has its own budget, and a call is charged to the scopes
of its issue as they are when the call is recorded (decided by the maintainer on 2026-11-25,
counter-review item 59). `budget_exhausted` is recorded once per scope, at the event whose
provider tokens reach the budget, a refused stale result included (R21, R29; decided by the
maintainer on 2026-11-25, counter-review item 58).

Units. Two separate limits bound the work, and this section is the one place that states their
units (Q13). The token budgets of the table above count provider tokens, per budget scope:
before a step starts, the engine checks the budget of every scope of the step, and an exhausted
scope stops the new steps of that scope only, except the dispatches of `review` exempted below,
with the stop reason `budget_exhausted` naming it (R29); other scopes go on. The verification
reserve of SPEC-0006 R9 to R11 counts agent invocations: one complete review pass of an issue
with its retries, derived from the recorded limits and never written as a number of its own:

```text
reserve(small)       = 0
reserve(substantial) = lenses(substantial) x (1 + limits.executor_retries)
reserve(contract)    = lenses(contract) + 1
```

`lenses(tier)` is the number of dispatches of `review` of the tier in the table of section Review
by risk tier: one for `substantial`, each with its retries (R11, I28); three for `contract`, plus
the one repetition of the panel, which replaces the retries (R8, Q12). With the default
`limits.executor_retries` of 2, the reserve is 3 invocations for `substantial` and 4 for
`contract`. Neither limit is converted into the other.

A dispatch of `review` covered by the verification reserve is exempt from an exhausted token
budget (R29, decided by the maintainer on 2026-11-22) and from the turn limit (R28, decided by
the maintainer on 2026-11-25, counter-review item 55): work already built is always verified,
and no new work starts in the exhausted scope. Its provider tokens are still recorded and
counted in the scope, and its turn is counted.

Until the verification reserve enters the state with SPEC-0006 (M2), every dispatch of `review`
of an issue whose `build` has completed counts as covered: work already built is verified even
when its token scope is exhausted or its turns are at the limit, within
`limits.executor_retries` and the repetition of Q12.

Usage and rate limits of a provider are not failures: the runtime
waits or asks (SPEC-0008 R16) and the dispatch stays open.

### Risk tier

The tier of an issue is the highest of the tier of its area and the tiers of the paths of
`changed_paths` at the head that the decision names, as the recorded policy maps them
(SPEC-0021): `verified_head` for the skip of `review` (I10, I48) and the verifiers of `review`
(R49), the recorded head, `pull_request.head_sha`, for the merge waiver (I17, I40, G7). Only a
decision that names no head, the skip of `counter_review`, uses the tier of the area alone (R25;
decided by the maintainer on 2026-11-25, second counter-review S1). The skip of `review` and the
merge waiver both need the changed paths recorded at their head: without them, `review` runs
and the merge gate is opened (decided by the maintainer on 2026-11-25, second counter-review
check N-B1). The enum
`risk_tier` has the values `small`, `substantial` and `contract`, in rising order of risk. The engine computes the tier
at every decision that uses it (R25); it is never reported by an agent. The tier decides which
steps are skipped (R25), the verifiers of `review` (R8, R11, R49) and whether the merge gate is
waived (R17).

These three tiers are the only names of risk in the specifications (Q10). The levels high,
medium and low that #309 first used are `contract`, `substantial` and `small`; R8, R11 and
SPEC-0006 R11 are written with the tier names, and this section is the one place that defines
them.

### Review by risk tier

| Tier | `counter_review` | `review` | Dispatches of `review` |
| --- | --- | --- | --- |
| `small` | As the policy says (R25) | Skipped: the deterministic checks of `verify` decide alone (R11) | None |
| `substantial` | As the policy says (R25) | One verifier (R11) | `proof` |
| `contract` | As the policy says (R25) | A panel of three verifiers (R8) | `presence`, then `behaviour`, then `proof` |

The lenses, the coverage units, the votes (verdicts) and their evidence are defined once, in
SPEC-0003 section Coverage and verification. Each dispatch of `review` names its lens and its
verifier in `step_started`. A verifier is identified by its executor and its holder `(executor,
holder)`; it is not one that ran `build` or `counter_review` of the issue, and the three lenses
of a panel have three distinct verifiers (R49). This identity protects the integrity of the
process against a mistake of the runtime, not against an agent that declares a false holder: it
is not authentication (decided by the maintainer on 2026-11-25, counter-review item 52). Its
result carries one vote per coverage unit of the requirements cited by the task, each with its
citations (R24). The engine records each vote as `coverage_verdict` (SPEC-0003 R14, R15); a vote
without a citation for each claim counts as `uncovered`, and its `coverage_verdict` records
`downgrade` `uncited` (R10). After the last lens, the engine tallies each unit under R9; no model
computes or reports the outcome. A verdict produced on a commit other than the head of the
branch as last observed (`branch_changes_observed`), or on a dirty working tree, is a result that
fails a guard (SPEC-0003 R17): `result_rejected`, one retry (R23). A dispatch of `review` starts
only while the changed paths of the branch are recorded at the head that `verify` verified (I3,
I10, `head_not_observed`), whatever the order of the two observations, so that this comparison
always has a head.

A task whose cited requirements give no coverage unit (no requirement cited, a requirement
without acceptance criteria, or a specification without `coverage_planned`) would pass review
without a vote; the engine holds it with a question of category `needs_validation` instead
(I46; decided by the maintainer on 2026-11-25, counter-review item 54).

The repetition of the panel that R8 allows is one more dispatch of one lens of the same review
step when its verifier fails or its result is rejected (Q12), once per review step; it replaces
`limits.executor_retries` for the dispatches of `review` of the tier `contract`, so that the
panel costs at most four invocations. The review of `substantial` costs at most one dispatch
plus `limits.executor_retries` retries. The reserve of SPEC-0006 R11 covers both, with the
formula of section Limits and budgets, Units. A second failure of any lens
holds the issue with `executor_failure` (I29).

`review` follows the tier for every tier (Q11): the policy may skip `counter_review` for any
tier, but never `review` of `substantial` or `contract`, and never keeps `review` for `small`; a
policy that lists another step in `skips` is refused (R52). A tier that rises after
`counter_review` does not repeat it; it raises the review required at `review`. A merge waiver
needs a `branch_changes_observed` at the recorded head: without one, the merge gate is opened
(R25, section Risk tier; the tier of the area alone never waives it).

### Revisions

A command carries `expected_revision`, the hash of the latest ledger event the sender read
(ADR 0009 C6, SPEC-0004 R5), unless the table Subjects of commands says it carries none. A
revision is stale when it is not the hash of an event of the current segment of the ledger
(SPEC-0003 section Storage), or when an event recorded after it touched the subject of the
command. A revision older than the current segment is therefore stale, whatever happened since;
the state keeps the map from the hash of each event of the current segment to its `seq`, so that
no scan of the ledger is needed and the map is bounded by `snapshot_every` events (decided by the
maintainer on 2026-11-25, counter-review item 62, option (b)). Until snapshots are built, the
current segment is the whole ledger.

Subjects of commands (decided by the maintainer on 2026-11-25, counter-review items 60 and 61;
the content of the table decided by the maintainer on 2026-11-25, with the amendment):

| Command | Subject | An event recorded after the revision touches the subject when |
| --- | --- | --- |
| `claim_requested`, `step_requested` | The issue | Its header names that issue |
| `run_start_requested`, `project_step_requested`, `specification_submitted` | The project | It changes the phase, a gate or the plan: `phase_changed`, `gate_opened`, `gate_resolved`, `gate_withdrawn`, `gate_waived`, `plan_observed`, `artifact_status_changed`, `limits_set`, `policy_set`, `subject_resumed` |
| `session_end_requested`, late hand-over (S5) | The session | It is a `checkout_acquired`, a `checkout_released` or a `session_ended` of that session |
| `session_start_requested` | None | Never: its `expected_revision` is not compared |
| `lease_renewal_requested` | None | It carries no `expected_revision` and applies to the session (S4); it is never a retry of SPEC-0003 R52: every heartbeat extends the lease (decided by the maintainer on 2026-11-25, second counter-review B8) |

A command other than `lease_renewal_requested` whose `input_hash` is already recorded in its
session, or anywhere in the ledger for a command without a session, is a retry: it is answered from the recorded events, found through the
map from its `input_hash` to the `seq` of its first and last event, before its revision is
evaluated (SPEC-0003 R52). A stale command is refused with the current revision in `expected` and
nothing is recorded (R41); the sender reads the state again and decides again. Staleness and
every guard are evaluated by the writer under its lock on the latest state, so that two commands
that both read the same revision cannot both pass a guard that only one of them may pass. This is
the rule of ADR 0009 C6 as amended by #233. This section adds three things to C6: the subjects of
the table above, the bound of the current segment, and the order of the checks, in which a retry
of SPEC-0003 R52 is answered before its revision is evaluated.

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
| `run_start_requested` | Command | Runtime: account, mode, effective configuration | R45, R46 |
| `session_start_requested` | Command | Runtime or agent: holder, executor, mode | S1 |
| `claim_requested` | Command | Runtime or agent | I1, I2 |
| `step_requested` | Command | Runtime or agent, the holder; for `review`, the lens and the verifier `(executor, holder)` | I3, I4, I46 |
| `project_step_requested` | Command | Runtime: step `council`, `plan` or `reconcile`, subject | P3, P12 |
| `lease_renewal_requested` | Command, without `expected_revision` | Runtime of the holder, as SPEC-0003 section Clock times says | S4 |
| `session_end_requested` | Command | Agent, the holder, with its hand-over input (SPEC-0001) when it holds a checkout | S2, S3, I30 |
| `handover_submitted` | Command | Agent, the last holder of an issue after its lease ended, with its hand-over input | S5 |
| `specification_submitted` | Command | Agent: the normalised specification (SPEC-0001 R12) | Gate `specification` |
| `step_result` | Result | Runtime: validated executor output, outcome `completed`, `changes_requested` or `question_raised`, with the executor call record; for `claim`, the branch and worktree it prepared; for `pull_request`, the number and head it pushed; for `review`, the votes with their evidence (SPEC-0003, Coverage and verification) | I5 to I9, I13, I14, I27 to I29, I38, I44 |
| `verification_result` | Result | Runtime: the head it verified (`head_sha`), check names and outcomes, the failure note | I10 to I12 |
| `executor_failure` | Result | Runtime: `error` or `timeout`, with the executor call record when one exists | I28, I29 |
| `council_report`, `plan_proposal`, `roadmap_change_proposal` | Result | Runtime, for a project dispatch | P4, P6, P12 |
| `plan_observed` | Observation | Plan adapter: the items of the kinds of section Configuration in the state and the active sprint, in parts when larger than the line limit | Readiness, lanes, phase |
| `branch_changes_observed` | Observation | Git adapter: branch, head, changed paths; at least for every head named by a passing `verification_result`, so that `review` never waits for an observation that is not coming (I3); in parts when larger than the line limit (SPEC-0003 R59) | R25, I43, I48, I50 |
| `brief_submitted` | Observation | Tracker: a brief submitted on its gate issue | P1 |
| `approval` | Observation | Tracker: person, gate, decision `approve` or `reject`; for a `roadmap` or `roadmap_change` gate, the content of the proposal | G2, G3, P2, P5, P7, P8, I19 |
| `answer` | Observation | Tracker: person, question gate, text, option when one is named; for a question of a council report, its identifier `Q<k>` | G4, I32 |
| `unblock_requested`, `stop_requested` | Observation | Tracker: person, issue or brief; for `unblock_requested`, the hold reason when one is named | I32, I33, P9 |
| `status_change_requested` | Observation | Tracker: person, a person transition of SPEC-0001 | P10, P11, I34 |
| `pull_request_opened`, `pull_request_synchronized`, `pull_request_checks_completed`, `pull_request_review_submitted`, `pull_request_merged`, `pull_request_closed`, `pull_request_reopened` | Observation | Tracker (SPEC-0003 R9); `pull_request_merged` also carries `changed_paths`, the paths the pull request changes at its merged head relative to its base, relative, NFC, sorted by code point, without duplicates, as `branch_changes_observed` reports them; they are not part of the payload, and the engine records their effect as `coverage_stale` (R51) | I15 to I26, I39 to I43, I47, I49, I51, P13, R51 |
| `issue_closed`, `issue_reopened` | Observation | Tracker; `issue_closed` carries `by_pull_request`, true when the tracker reports that the closing pull request closed the issue (counter-review item 39) | I35, I36, I45 |
| `default_branch_pushed` | Observation | Git adapter, for a push to the default branch whose commit is not the merge commit of a pull request as the tracker reports it; a merge of a pull request is reported only as `pull_request_merged`. It carries the commit, its base and `changed_paths` as for `pull_request_merged` | P13, R51 |
| `tracker_block_invalid` | Observation | Tracker adapter (SPEC-0005 R3) | I37 |

Payloads of the kinds owned by this specification, version 1. The header fields (`issue`,
`session`, `actor` and the cause of R3) are those of SPEC-0003 and are not repeated. A gate is
cited by the hash of its `gate_opened` event, of the type `GateRef`; a dispatch by the hash of
its `step_started` event, of the type `DispatchRef`; a revision has the type `Revision` (decided
by the maintainer on 2026-11-25, counter-review section 4, code pattern 5).

Every payload below is a Rust type of `wayloop-schemas`, and its schema is generated from it
(ADR 0030); this table gives the members and their meaning, and no JSON schema is written by
hand. Each kind is one contract of the registry `contract::CONTRACTS`, in the area and under the
name that SPEC-0003 section Contracts of the ledger defines for every event kind, with the
reading direction, the version rule and the value forms (hashes, handles, issue numbers, paths,
times) stated there once. The same section gives the area and the name of the inputs that cross
the entry point: each command of the table Inputs is a contract of the area `commands` under the
name of its command in SPEC-0004.

| Kind | Fields |
| --- | --- |
| `run_started` | `account` (tracker handle of the runtime), `engine_version`, `mode` (`in_session` or `unattended`), `configuration_sha256` (of the effective configuration, canonical JSON) |
| `limits_set` | `approvers` (handles, sorted by code point, unique, at least one), `limits` (`{turns_per_issue, attempts, same_failure, executor_retries, lease_minutes}`, the units of section Limits and budgets), `budgets` (`{council, planning, sprint, milestone}`, provider tokens), `ledger_budgets` (`{handover, session, issue, milestone}`, size tokens, SPEC-0003), `approved_by` (handle), `gate` (optional, the configuration gate) |
| `policy_set` | `policy_revision` (SHA-256 of the canonical policy), `project_name` (the slug of `project.name`, section Names of a checkout, Q21), `areas` (list of `{name, paths, tier}`), `path_tiers` (list of `{pattern, tier}`), `skips` (list of `{tier, steps}`; the only step a policy may skip is `counter_review`, R25, Q11), `merge_waived_tiers` (list of tiers), `required_checks` (names, sorted), `approved_by`, `gate` (optional) |
| `plan_observed` | `previous_plan_sha256` (of the plan observed before, absent for the baseline), `plan_sha256` (of the whole observed plan), `part`, `parts`, `items` (list of `{id, type, status, issue, brief, council_report, subject, epic, milestone, story, specifications, requirements, depends_on, area, units}`, the members that apply to the type, optional fields left out), `sprint` (optional `{id, lanes: [{name, paths, issues}]}`); a plan larger than the line limit is split into parts |
| `branch_changes_observed` | `branch`, `head_sha`, `part`, `parts` (from 1; `parts` 1 when the paths fit one line), `paths` (relative, NFC, sorted by code point, without duplicates; the paths of this part); the paths of a head are applied when its last part arrives (SPEC-0003 R59, decided by the maintainer on 2026-11-25, SPEC-0003 Q15) |
| `phase_changed` | `from`, `to` |
| `gate_opened` | `kind`, `subject` (artifact identifier, issue or `configuration`), `question` (for kind `question`: `{category, text, options, recommendation}`, `options` empty for free-text answers), `proposal` (optional: for a report, a plan or a change, `{file, sha256}`), `configuration` (for kind `configuration`: the loosening keys with their proposed values, in the shape of `limits_set` and `policy_set`, members not proposed left out) |
| `gate_resolved` | `gate`, `resolution`, `by` (handle) |
| `gate_withdrawn` | `gate`, `reason` (`subject_superseded`, `head_changed`, `checks_failed`, `pull_request_closed`, `bypassed`, `block_valid`, `configuration_replaced`, `review_left`: the issue left `review` while the question of I46 was pending (final check, decided by the maintainer on 2026-11-25)) |
| `gate_waived` | `kind` (`merge`), `subject` (issue), `tier`, `policy_revision`, `policy_approved_by` |
| `gate_bypassed` | `gate` (optional), `merge_commit` |
| `approval_recorded` | `gate`, `decision` (`approve` or `reject`), `by`, `proposal_sha256` (for a `roadmap` or `roadmap_change` gate, of the content carried) |
| `question_answered` | `gate`, `text`, `option` (optional), `question_id` (optional, `Q<k>` of a council report), `by` |
| `input_ignored` | `input` (input name), `code`, `pointer` (optional) |
| `step_started` | `step`, `subject` (for a project step, the brief or report), `executor` (optional), `turn` (boolean), `attempt`, `lens` (for `review`), `verifier` (for `review`, `{executor, holder}` of the agent that runs the lens) |
| `step_completed` | `step`, `outcome` (`completed`, `changes_requested`, `question_raised`), `skipped` (steps skipped after it), `output` (optional `{file, sha256}`), `head_sha` (for `verify`, the head it verified, I10; for `pull_request`, the head pushed) |
| `step_retried` | `step`, `error` (`error` or `timeout`), `retries` |
| `step_failed` | `step`, `cause` (`result_rejected`, `error` or `timeout`), `retries` |
| `attempt_failed` | `attempt`, `source` (`verification`, `pull_request_checks`, `review` or `merge_rejected`), `failing_checks` (sorted; for `review`, the uncovered units), `note` (`{tried, failed, first_error, next}`) |
| `issue_blocked` | `reason`, `gate` (optional); no target, since the target is computed at the removal (decided by the maintainer on 2026-11-25, final check 2; criterion: stability and maintainability) |
| `issue_unblocked` | `removed` (reason), `resumed_at` (issue state) |
| `subject_resumed` | `subject` (the brief of P9), `by` |
| `issue_done` | `pull_request`, `merge_commit` |
| `issue_superseded` | `superseded_by` (task identifiers) |
| `artifact_status_changed` | `artifact`, `from`, `to`, `by` (optional, the person) |
| `budget_exhausted`, `budget_restored` | `scope` (`{kind, id}`, kind `council`, `planning`, `sprint` or `milestone`; `id` the brief, sprint or milestone), `budget` (provider tokens), `provider_tokens` |
| `reconciliation_required` | `subject`, `code` (a value of the enum `reconciliation_code` of the table Reconciliation codes; it names the inconsistency found, it is not a refusal and not a constant of `Code`), `detail` (optional) |

`checkout_acquired`, `lease_renewed`, `checkout_released`, `session_started`, `session_ended`,
`handover_recorded`, `executor_call_recorded`, `result_rejected`, `summary_recorded`, the pull
request events and the kinds `coverage_planned`, `coverage_verdict` and `coverage_stale` are
owned by SPEC-0003; this machine emits them with the fields that specification defines.

Reconciliation codes (decided by the maintainer on 2026-11-25, counter-review item 64):

| `reconciliation_code` | Recorded by | Inconsistency |
| --- | --- | --- |
| `issue_closed_without_merge` | I35 | The issue was closed, not by its closing pull request |
| `issue_reopened` | I36 | A `done` or `superseded` issue was reopened |
| `merged_without_approval` | I24 | The closing pull request merged without an approved or waived merge gate |
| `merged_after_end` | P13, I36 | A pull request of a `done` or `superseded` issue merged |
| `status_conflict` | Later `plan_observed` | The tracker shows a status that is neither the current nor the previous status of the ledger |
| `pull_request_unlinked` | I15 | The pull request of the branch of the issue does not close the issue (SPEC-0001 R10) |
| `issue_in_two_lanes` | `plan_observed` | An issue is listed in two lanes of the sprint |
| `observation_inconsistent` | R32, I36 | Any other observation without a fact transition |

### Artifact transitions taken by the engine

The engine transitions of SPEC-0001 and their triggers (R43). Person transitions arrive as
`approval` or `status_change_requested` and pass G2 or the approver guard. At a claim, the
transitions are recorded in this order: the task, its story, its cited specifications by
identifier in code point order, its epic, its milestone (decided by the maintainer on
2026-11-25, counter-review item 7). At a merge: the task, then its story and its epic.

| Artifact | Transition | Trigger |
| --- | --- | --- |
| Task | `approved` to `implementing` | I1, I2 |
| Task | `implementing` to `approved` | I31 (lease expired), only while `pull_request_step_completed` is false for the issue (decided by the maintainer on 2026-11-25, second counter-review B1 and check N-S2) |
| Task | `implementing` to `done` | I18, I24 |
| Task | `approved` to `done` | I24, when the closing pull request merges while the task is `approved` (SPEC-0001 as amended; decided by the maintainer on 2026-11-25, counter-review item 27) |
| Milestone, epic | `approved` to `implementing` | First claim of a task of it |
| Story | `approved` to `implementing` | First claim of a task that names it |
| Specification | `approved` to `implementing` | First claim of a task that cites it |
| Story, epic | `implementing` to `done` | A task of it reaches `done` or `superseded` and every task of it is `done` or `superseded`, at least one `done` |
| Gate issue | `draft` to `done`; `draft` to `superseded` | G2 on its gate; G3 or G5 on its gate |
| Change delta | `approved` to `implementing`; `implementing` to `done` | Owned by SPEC-0009 (R3), not taken by this machine |

### Stop reasons

`stop_reason` (R42) holds each value of `question_category`, for a pending question, and each
value of `hold_reason` other than `question`, for a held issue (table Holds), by reference to
those enums, without repeating their values; and its own values `approval_required` (a pending
gate other than a question), `budget_exhausted` (a token budget of a scope, R29),
`verification_reserve_exhausted` (the verification reserve in invocations, SPEC-0006 R10),
`rejected_awaiting_input`, `engine_account_is_approver`, `run_not_started` (R46),
`awaiting_checks` (R38), `no_active_sprint` (a reconciliation that belongs to no milestone while
no sprint is active, section Limits and budgets; no existing value fits, since no budget is
exhausted and no person is awaited: the plan lacks a sprint; decided by the maintainer on
2026-11-25; the value and its description are in the slice, task T8, and the behaviour of AC83
is outside it, decided by the maintainer on 2026-11-25) and `ledger_broken`. Each stop names its gate, question, issue, scope or subject (final check, decided by the maintainer on 2026-11-25).
`wayloop next` and `wayloop status` return them (SPEC-0004).

### Errors

Refusals use the error report of ADR 0009 C3 as amended, built from the types of
`wayloop-schemas`: every code is a constant of `Code`, every reason a value of `Reason`, and no
code or reason is spelt anywhere else (ADR 0030 G1). Pointers follow RFC 6901 from the root of
the input sent, or of the document given by `document` when the input holds a list `documents`
(SPEC-0004 R2); the field that names the subject (`/issue`, `/dispatch`, `/expected_revision`)
carries the error. The optional member `found` of the error holds the values that the refused
condition found, in the input or in the state; its one definition is in SPEC-0003 section
Contracts of the ledger, and the shape it takes for each code is given in the row of that code
below (decided by the maintainer on 2026-11-25, counter-review item 65; one definition and a shape
per code, second counter-review S5). A code whose row gives no shape of `found` never carries
it. A new meaning gets a new code (ADR 0009 C8). A code used here and
defined by SPEC-0001 or SPEC-0003 is listed with a reference, not defined again.

Severity: `refusal`, the command or input is refused and nothing is recorded; `recorded`, the
code is carried by an event (`input_ignored`, `result_rejected`) because a result or an
observation is not refused; `readiness`, the code is reported for an issue that is not ready
(R19) and refuses a claim.

| Requirement or row | Code | Reason | Severity | `expected` |
| --- | --- | --- | --- | --- |
| R4 | `input_class` | `missing`, `unknown_field` | refusal | The field the class requires (`dispatch` or `external_id`); none for `unknown_field`. A command without `expected_revision` gets `command_input` of SPEC-0004 |
| R45, R53 | `approvers_empty` | `missing` | refusal | None |
| R44, R53 | `approver_duplicate` | `duplicate` | refusal | None; the repeated handle in `pointers` |
| R53 | `limit_range` | `range` | refusal | None; the minimum in `message`; `found`: the value given, an integer |
| R18 | `transition_not_allowed` | `state` | refusal, readiness | The states from which the transition is allowed |
| R18 | `step_mismatch` | `state` | refusal | The current state |
| R18, R23 | `result_kind_mismatch` | `enum` | recorded (`result_rejected`) | The result kinds and outcomes the step of the dispatch accepts |
| R23, I6 to I9, I13, I27; SPEC-0003 R21 | `result_schema` | The reasons the schema interpreter gives: `missing`, `unknown_field`, `type`, `enum`, `length`, `range`, `syntax`, `requires` | recorded (`result_rejected`) | As the schema interpreter gives them (decided by the maintainer on 2026-11-25, second counter-review B7) |
| I1, I2 | `session_not_open` | `state` | refusal | None |
| I3, P3 | `dispatch_open` | `state` | refusal | None; `found`: the open dispatch, a `DispatchRef` |
| P3 | `project_step_not_applicable` | `state` | refusal | None |
| P12, section Limits and budgets | `no_active_sprint` | `state` | refusal | None; a `reconcile` step of a reconciliation that belongs to no milestone has no budget scope to charge while no sprint is active, so the request is refused and nothing is recorded; the same value is the stop reason of section Stop reasons (decided by the maintainer on 2026-11-25) |
| R19 | `task_not_approved`, `plan_not_approved`, `specification_not_approved` | `state` | readiness | The accepted statuses |
| R19 | `dependency_not_done` | `state` | readiness | `done` |
| R19, I3 | `issue_on_hold` | `state` | readiness, refusal | None |
| R19 | `lane_busy` | `conflict` | readiness | None; `found`: the issue that occupies the lane, an issue number |
| R19 | `checkout_held` | Defined by SPEC-0003 (R10) | readiness | |
| R19 | `not_in_lane` | `missing` | readiness | None |
| R19 | `awaiting_external` | `state` | readiness | None |
| R19 | `area_paths_overlap` | `conflict` | refusal | None; the overlapping areas in `pointers` |
| R19, R29, I3, P3 | `budget_exhausted` | `range` | readiness, refusal | None |
| R21 | `dispatch_stale` | `stale` | refusal | The open dispatch, when one exists |
| I3, S2 to S5 | `checkout_not_held` | Defined by SPEC-0003 (R48) | refusal | |
| S2, S5 | `handover_input` | Defined by SPEC-0001 (R22) | refusal | |
| S2, S5 | `handover_budget` | Defined by SPEC-0003 (R12) | refusal | |
| I3, R49 | `verifier_not_independent` | `conflict` | refusal | None; the step that the verifier ran in `pointers` |
| I3, R49 | `lens_not_expected` | `state` | refusal | The lens R49 expects next |
| I3, R49 | `head_not_observed` | `state` | refusal | `verified_head`, the head whose changed paths are not yet recorded |
| I5, R20 | `checkout_mismatch` | `conflict` | recorded (`result_rejected`) | The branch and worktree names of the checkout |
| R52 | `policy_skip_not_allowed` | `enum` | refusal | `counter_review` |
| R52 | `policy_waiver_not_allowed` | `enum` | refusal | `small` |
| R53 | `budget_missing` | `missing` | refusal | The budget keys not given |
| R53, Names of a checkout | `project_name_invalid` | `missing`, `syntax` | refusal | None; for `syntax`, the slug that the producer gives for the name in `fix`, left out when it is the fallback `work` (N4) |
| R14, G6 | `actor_not_authorised` | `requires` | recorded (`input_ignored`) | None |
| R15 | `question_unanswered`, `checks_not_green`, `plan_changed` | `requires` | recorded (`input_ignored`) | None |
| R15, P1, P4, P6 | `subject_invalid` | `requires` | recorded (`input_ignored`, or `result_rejected` for P4 and P6) | None; the first error of the subject in `message` |
| R15 | `configuration_unchanged` | `requires` | recorded (`input_ignored`) | None |
| P7 | `proposal_hash_mismatch` | `conflict` | recorded (`input_ignored`) | The SHA-256 of the proposal of the gate |
| P10, P11, I34 | `transition_refused` | Defined by SPEC-0001 (R13) | recorded (`input_ignored`) | |
| R23, R24 | `checklist_incomplete`, `objection_unruled`, `vote_missing` | `missing` | recorded (`result_rejected`) | The missing requirements, objections or coverage units |
| I27 | `question_invalid` | `missing` | recorded (`result_rejected`) | The missing members of the question |
| R23, I13 | `verdict_commit_stale`, `verdict_tree_dirty` | Defined by SPEC-0003 (R17) | recorded (`result_rejected`) | |
| G4 | `option_unknown` | `enum` | recorded (`input_ignored`) | `retry`, `stop` |
| R37, I23 | `head_stale` | `stale` | recorded (`input_ignored`) | The recorded head |
| Input classes, G2, I45 | `no_effect` | `state` | recorded (`input_ignored`) | None |
| R41 | `revision_stale` | `stale` | refusal | The current revision |

### Examples

`examples/` holds scenarios used by AC43, in the trace format below (R58). Each scenario is a
YAML document validated by the trace schema `specs/0002-state-machine/examples/trace.schema.json`,
which the runner of task T11 of `tasks.md` reads; the schema is a test fixture of this
specification, not a contract of the engine.

- `scenario`: identifier, `snake_case`.
- `configuration`: the approvers, limits, budgets and policy recorded by the first `limits_set`
  and `policy_set`, in their payload shapes.
- `initial`: the state before the first input, given as a list of events of the catalogue to
  apply from `ledger_created`, never as a state shape, so that the initial state is reached by
  `apply` and checked by the invariants.
- `steps`: a list; each entry is one of `input` (the input, its class and its fields, every field
  its schema requires, `expected_revision` and the time `at` included), `due_events` (`now`) or
  `next_steps` (`integrity`, `free_workers`), with `expect`: the expected `issue_state`, holds,
  checkout, refusal codes in order, recorded event kinds in order, and any payload member.
- Symbols: a value written `$<name>`, such as `$D1` for a dispatch, `$G1` for a gate, `$rev12`
  for a revision or `$H1` for a head, is bound by the runner to the value the engine produced at
  its first appearance in an `expect`, and compared at every later appearance. A symbol used in an
  input before it is bound fails the scenario. Heads are given as symbols bound by the first
  input that names them.

The four scenarios were written before the amendments of 2026-11-25. Task T11 rewrites them in
this format and to these amendments (expected revisions, the names of the checkout, the release
at the end of the `pull_request` step, the absence of `summary_recorded`); until then they are
not normative.

- `issue-cycle.happy-path.yaml`: claim to `done` for an issue of tier `substantial`.
- `issue-cycle.repeated-failure.yaml`: two failures of one check, the question, the answer `retry`.
- `issue-cycle.release-and-stale.yaml`: a scoped revision accepted, a stale command, a lease
  expiry, a stale result whose cost is kept, a new claim that resumes.
- `issue-cycle.small-tier-waiver.yaml`: an issue of tier `small` merged under the policy waiver.

## Interfaces
- Core: `decide`, `apply`, `due_events` and `next_steps` of section The functions of the core, and `genesis`, the decision of `wayloop init` in the same section (second counter-review S2), in `crates/wayloop-core`; the constant transition table of R54; enums `issue_state` (from `IssueStep`), `gate_kind`, `gate_state`, `resolution`, `hold_reason`, `question_category`, `stop_reason`, `action_kind`, `phase`, `reconciliation_code`, `integrity`, and `risk_tier`, the enum of section Risk tier, which the policy of SPEC-0021 uses. The lens is the enum of SPEC-0003 section Coverage and verification, imported, not defined here. Enums that reach a contract are types of `wayloop-schemas` (ADR 0030 G1). Value types `Timestamp`, `Revision`, `DispatchRef`, `GateRef`, `ProviderTokens` and `SizeTokens` (SPEC-0003 section Contracts of the ledger). All new.
- `wayloop next` returns `actions`, `ready` (issues, then stale coverage units) and `stops`; `wayloop status` returns the phase, the gates, and the state, holds and stop reasons of every issue (SPEC-0004). Both report `meta.revision`. Their data types are output contracts of the area `commands`, generated, in the envelope `Envelope<D>` (ADR 0030 G1, G5); new output members are additive.
- Commands that change state (`claim`, step requests, session end, run start) take `expected_revision` (SPEC-0004 R5) and refuse with `revision_stale`; lease renewal takes none (table Subjects of commands).
- Events: the payloads of section Events for the kinds owned here, as types of `wayloop-schemas` registered in `contract::CONTRACTS` with the area, names and rules of SPEC-0003 section Contracts of the ledger; snapshots and their history are generated (ADR 0030 G5, G6). New, with the new kind `subject_resumed`.
- Configuration: the keys of section Limits and budgets and the list `approvers`, in the project file of ADR 0010, written by `wayloop init`; changes reach the state only under R45. New.
- Error codes of section Errors, as constants of `Code`; reasons from `Reason` (`wayloop-schemas`). New.
- Test fixtures: the trace schema of section Examples. New; not a contract.

## Dependencies
- SPEC-0001 (Approved): statuses, allowed transitions, hand-over `step` values; amended on 2026-11-25 for the task transition `approved` to `done`, the gate issue of a rejected gate, the condition of the task transition `implementing` to `approved` (second counter-review B1) and the task member `branch_type` (B10).
- ADR 0009, amended on 2026-11-25 for the member `found` of the error and for the segment bound and the order of the checks of C6 (second counter-review S5).
- SPEC-0003: the event catalogue, the envelope of events, the payloads of checkout, session, hand-over, cost, rejected results, pull request and coverage events, `due_events` timing, the lease (section Clock times), the rule of a stale unit (R16) that R51 applies, and the member `found` of an error.
- ADR 0029 (Accepted) amends constitution IV for the merge waiver of the tier `small` (R17).
- ADR 0009 C6, as amended by #233, is the scoped staleness rule applied by section Revisions (R41).
- ADR 0030 (Accepted): payloads, inputs and outputs are generated contracts.
- SPEC-0004 R2, R14 and R18 as amended: error report with `document`, entry point, `error` without `data`.
- #228 is done: the `IssueStep` identifiers are the `snake_case` values used here.
- #347: the allowed status transitions of SPEC-0001 as data in the core, which the guards "SPEC-0001 transition allowed" of P10, P11 and I34 and the transitions of R43 use.
- SPEC-0003 section Coverage and verification (#309): lenses, coverage units, votes and their events; SPEC-0006 R11 sizes the reserve of the verifiers of R8 and R11; SPEC-0009 R10 and R11 own the question of a `needs_validation` verdict and the convergence run.
- ADR 0011, ADR 0012 and ADR 0013 are Proposed; executors, gates on the tracker and lanes depend on them.
- Draft specifications this one cites: SPEC-0004 (R5, R11, R14, R18), SPEC-0021 (path patterns, prefixes in M1), SPEC-0006 (reserve), SPEC-0005 and SPEC-0007 (mapping of approvals and handles). The slice of `plan.md` uses none of their unbuilt parts.
- Follow-up changes in other specifications are listed in the report of the pull request that proposes this specification for approval.

## Plan and tasks
`plan.md` and `tasks.md` hold the smallest vertical slice of the issue cycle, decided by the
maintainer on 2026-11-25 (counter-review section 4). Tasks #48 (transition table and guards),
#49 (next step and ready work) and #58 (dry run on local fixtures) implement it, with #347 and
the minimal parts of #161 and #162 that the slice needs.

## Open questions
- [x] **Q1. Which steps does the tier `small` skip?** Decided by the maintainer on 2026-11-17: (b), `small` skips both `counter_review` and `review`, set per tier in the policy, with the tier computed by the engine from the area and the observed diff (R25, section Risk tier). The skip of `review` by the policy was replaced on 2026-11-22 by Q11: `review` follows the tier.
- [x] **Q2. May the loop merge an issue of tier `small` without a person at the gate?** Decided by the maintainer on 2026-11-17: the project policy may waive the merge gate for `small`; ADR 0029 amends constitution IV. The engine records `gate_waived` with the policy revision and the person who approved it, never `gate_resolved`; a policy change that widens `small` or adds a waiver passes the `configuration` gate (R45, R17).
- [x] **Q3. When is an expected revision stale?** Decided by the maintainer on 2026-11-17: (b), scoped to the issue of the command, or to the phase, gates and plan for a project command; guards are evaluated again under the writer lock; ADR 0009 C6 is amended (R41, section Revisions). Completed by the maintainer on 2026-11-25 (counter-review items 60 to 62): the table Subjects of commands, and a revision older than the current segment is stale.
- [x] **Q4. Where does a new claim resume after a release?** Decided by the maintainer on 2026-11-17: (a), at the state reached.
- [x] **Q5. Which issues are ready without an active sprint?** Decided by the maintainer on 2026-11-17: (b), each area of the policy is a lane; a policy with overlapping area paths is refused, and an issue without area is not in a lane (`not_in_lane`).
- [x] **Q6. Default limits.** Decided by the maintainer on 2026-11-17: (a), turns 20, attempts 3, same failure 2, executor retries 2, lease 120 minutes under the single key `limits.lease_minutes`, recorded in `limits_set`, with a heartbeat every quarter of the lease (R22).
- [x] **Q7. What happens when the pull request of an issue is closed without merge?** Decided by the maintainer on 2026-11-17: (a), hold with `pull_request_closed`; the pull request is cleared, and `pull_request_reopened` restores it (R36).
- [x] **Q8. What is a budget scope?** Decided by the maintainer on 2026-11-17: phase budgets for `intake`, `council` and `planning`, a budget per sprint, and a budget per milestone in `delivery` (section Limits and budgets). Amended by the maintainer on 2026-11-25 (counter-review item 22): the budget follows the step kind, counted per brief; a reconciliation step charges its milestone; `budgets.intake` is removed.
- [x] **Q9. Who is an authorised approver?** Decided by the maintainer on 2026-11-17: an explicit list written by `wayloop init`, run by a person. Adding an approver or raising a limit or a budget opens a gate of kind `configuration` that only an already recorded approver resolves; decreases apply at once. The runtime reports its account in `run_started`, and the engine proposes nothing while that account is an approver (R45, R46, R44).
- [x] **Q10. Which tier is each risk level of R8, R11 and SPEC-0006 R11?** Decided by the maintainer on 2026-11-22: (a), one set of names: high is `contract`, medium is `substantial`, low is `small`, defined once in section Risk tier. R8, R11, AC4 and AC7 are written with the tier names, and SPEC-0006 R11 is amended in this pull request to use them.
- [x] **Q11. May the policy keep `review` for `small`, or skip it for another tier?** Decided by the maintainer on 2026-11-22: (a), `review` follows the risk tier for every tier (R8, R11); the policy may skip only `counter_review` (R25, section Review by risk tier). This replaces the part of Q1 that let the policy skip `review`.
- [x] **Q12. What is the one repetition of the panel that R8 allows?** Decided by the maintainer on 2026-11-22: (a), one more dispatch of one lens of the same review step when its verifier fails or its result is rejected, in place of `limits.executor_retries` for the review of `contract`, so that the panel costs at most four invocations, as SPEC-0006 R11 reserves (R8, AC4).
- [x] **Q13. Does the review budget match the reserve of SPEC-0006 R11?** Decided by the maintainer on 2026-11-22: the reserve covers one complete review pass with its retries: three invocations for `substantial` (one verifier and two retries) and four for `contract` (three verifiers and one repetition), none for `small`, with the default limits; the reserve is derived from the recorded `limits.executor_retries` by the formula of section Limits and budgets, Units, and SPEC-0006 R9 to R11, AC5 to AC7 refer to it. Completed by the maintainer on 2026-11-22: a dispatch of `review` covered by the reserve is exempt from an exhausted token budget, so that work already built is always verified (R29, section Limits and budgets, Units, SPEC-0006 R12); an exhausted reserve has its own stop reason, `verification_reserve_exhausted`. The reserve counts invocations and the token budgets are a separate limit, both stated once in section Limits and budgets; SPEC-0006 R12 is aligned with R29: an exhausted token budget stops the steps of its scope, not the whole loop.
- [x] **Q14. How is a specification approved?** (Counter-review item 26; outside the slice.) Decided by the maintainer on 2026-11-25: (b), the observed merge, by a recorded approver, of the pull request that sets the specification to `Approved`, recorded as `approval_recorded` on the gate opened by `specification_submitted`, with no change of the gate issue (section Gates and approval states).
- [x] **Q15. When is the checkout released after the pull request, and with which reason?** (Found while applying counter-review items 27 and 35.) Decided by the maintainer on 2026-11-25: (a), at the result of the `pull_request` step (I44), for an open and an update alike, with the new reason `awaiting_checks`; `awaiting_merge` is removed from the reasons, since no released version holds it (R35, I44, SPEC-0003 Payloads).
- [x] **Q16. How are the branch and the worktree of a checkout named?** (Found while applying counter-review item 28.) Decided by the maintainer on 2026-11-25: (b) for the type, a new optional member of the task artifact (SPEC-0001, Task, as amended), with `feat` until it is built; the member is named `branch_type`, since `type` already names the artifact type of an item of `plan_observed` (decided by the maintainer on 2026-11-25, second counter-review B10); (a) for the worktree, `<project>-<issue>` (section Names of a checkout). The source of the name of the project is Q21.
- [x] **Q17. What happens when the changed paths at `verified_head` reach an issue of tier `small` waiting at `review`?** (Found while applying counter-review item 33.) Decided by the maintainer on 2026-11-25: (a), the row I48.
- [x] **Q18. Which milestone does a reconciliation step charge when its subject has none?** (Found while applying counter-review item 22; outside the slice.) Options: (a) the milestone of the item whose inconsistency started the reconciliation, and a project-level reconciliation, such as a broken plan, is not proposed without one; (b) project-level reconciliations charge `budgets.planning` of the brief of the latest approved milestone; (c) a new budget `budgets.reconcile`, against the decision of item 22. Decided by the maintainer on 2026-11-25 (second counter-review check), corrected the same day with Q22: none of the three; a reconciliation that belongs to no milestone charges `budgets.sprint` of the active sprint, and with no active sprint it is not proposed (stop reason `no_active_sprint`; section Limits and budgets, section Stop reasons).
- [x] **Q19. Does a new head pushed outside a step, at `pull_request` without dispatch or at `merge`, return the issue to `verify`?** (Found while applying counter-review item 34.) Decided by the maintainer on 2026-11-25: (a), I22 applies only at the head `verified_head`, and any other head follows I39, back to `verify`.
- [x] **Q20. What happens to a run of the required checks at the recorded head that arrives while the dispatch of `pull_request` is still open?** (Found while applying counter-review item 35.) Decided by the maintainer on 2026-11-25: (a), the run is recorded and kept in `checks` (I49), and I44 applies the row it selects when it closes the dispatch.
- [x] **Q21. Where does the name of the project in the worktree name come from, and how does it reach the state?** (Found while applying Q16.) Decided by the maintainer on 2026-11-25: (a), a key `project.name` of `wayloop.toml`, given to `wayloop init`, checked as a slug of SPEC-0001 R11 (`project_name_invalid`) and recorded as `project_name` in the first `policy_set` (section Names of a checkout, AC64).
- [x] **Q22. What is the budget of the run in progress that Q18 charges?** (Found while recording Q18; outside the slice.) The table of section Limits and budgets has no run scope: its scopes are the brief (`budgets.council`, `budgets.planning`), the sprint and the milestone, and item 22 added no budget. Open when found: the key of the budget (a new key such as `budgets.run`, or an existing key counted per run), the identifier of a run (for example the `seq` of its `run_started`), its default and minimum, and what is charged when no `run_started` is recorded (R46 proposes no action then, so a reconciliation may never be dispatched without a run). Decided by the maintainer on 2026-11-25: the budget meant is `budgets.sprint` of the active sprint, not a budget of the run; no key, identifier or default is added, and with no active sprint the reconciliation is not proposed and `next_steps` reports `no_active_sprint` (AC83). This corrects the wording of Q18.
