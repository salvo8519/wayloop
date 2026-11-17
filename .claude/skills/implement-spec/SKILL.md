---
name: implement-spec
description: Implements the tasks of an approved specification test-first, one task per commit citing its requirements, and hands the branch to a reviewer of another agent. Use when a specification is Approved and its tasks are ready, or when the maintainer asks to continue one.
---

<!-- Generated from .agents/skills/implement-spec/SKILL.md by scripts/sync_agents.py. Edit the source, not this file. -->

Talk to the maintainer in their preferred language (`CLAUDE.local.md`); everything written to the repository or to GitHub is in English, without icons.

## Produces
Commits on the issue branch, one per task, each citing `[SPEC-NNNN Rk]`; ticked tasks in `specs/NNNN-<slug>/tasks.md`; the specification at status `Implementing` with its acceptance criteria ticked; a review of the whole branch by an agent other than the implementer, with every finding fixed.

## Steps
1. **Preconditions, deterministically.** Read the status line of `specs/NNNN-<slug>/spec.md`:
   it must be `Approved` or `Implementing`, and `plan.md` and `tasks.md` must exist next to it.
   Run `git status` (clean tree) and `git log --oneline origin/main..HEAD` (where the branch
   stands). Run `uv run python scripts/journal.py search <issue>` and read the latest
   hand-over of the issue. If any precondition fails, stop (see Stop and ask).
2. **Baseline.** Run `uv run just check` before changing anything. A failure that exists on
   `main` is recorded, not fixed here, unless the issue is about it.
3. **Pick the next unticked task** of `tasks.md`, in order. Read the requirements and
   acceptance criteria it serves and the files it touches. Tasks marked `[P]` run in parallel
   only in separate worktrees (the `run-sprint` skill); in one session, run them in order.
4. **Does it need to exist.** Before writing code, check in this order: is the behaviour
   required by a numbered requirement; does the codebase already do it (search with `rg`, or
   delegate to the explorer role for a wider question); does the Rust or Python standard
   library, or a dependency already in `Cargo.lock` or `uv.lock`, do it. Record the answer in
   one line in the commit body when it changed the plan.
5. **Test first** for acceptance criteria and bug fixes. Delegate to the test-engineer role
   when it is available, otherwise write the test yourself. Run the narrowest command that
   executes it (`cargo test -p wayloop-core <name>` or `uv run python -m pytest <path> -q`)
   and confirm it fails for the expected reason: a missing behaviour, not a compile error in
   the test or a wrong fixture. A test that passes before the change proves nothing; rewrite
   it.
6. **Minimum code** to make the test pass. Hard rules of AGENTS.md: decision logic only in
   `crates/wayloop-core`, adapters translate and never decide; the ledger is written only by
   the engine; model output enters the state only after schema validation. A change to a
   schema of artifacts, events or commands goes to the contract-auditor role before commit.
   Do not cut validation, error handling or security to make a test pass.
7. **Prove the task.** Re-run the task's test (it passes), then `uv run just check`. Apply the
   `verify` skill: state only what a command you ran proves.
8. **Tick and commit.** Tick the task in `tasks.md` and commit with the `commit` skill:
   `<type>(<scope>): <subject> [SPEC-NNNN Rk]`, scope from the Commit scope row of the
   specification, `Refs: #<issue>`, signed off. One task, one commit; unrelated files stay out.
9. **Loop** from step 3 until every task is ticked. Do not review each task; the branch is
   reviewed once.
10. **Close the specification.** Tick every acceptance criterion in `spec.md` that a test now
    verifies, and set the status to `Implementing` if it was `Approved`. `Done` belongs to the
    maintainer.
11. **Review by another agent.** The implementer never reviews its own work. Hand the branch
    to the `pre-pr` skill, which delegates the review by tier (reviewer, plus the
    contract-auditor for contracts), then to the `apply-review` skill for the findings.

## Stop and ask
- The specification is not `Approved`: stop and propose the spec-writer role; no engine code without an approved specification.
- A task contradicts a requirement, an ADR or another task, or needs a file outside the issue's scope: ask, do not choose.
- A task needs a new dependency, a new contract field or a change to `crates/` outside the plan.
- A test cannot be made to fail for the expected reason, or the baseline gate fails for a reason unrelated to the issue.

## Report
- Tasks done, each with its commit hash and requirements; tasks not done and why.
- Commands run and their result; what was not verified (for example a platform not tested) and the risk.
- The review status and the next step: `pre-pr`, or the question waiting for the maintainer.
