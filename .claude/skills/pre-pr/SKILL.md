---
name: pre-pr
description: Runs the full local gate, classifies the branch by risk tier, delegates the review lenses to agents that did not write the code, and drafts the pull request description. Use when a branch is about to be pushed for review or a pull request is about to be opened.
---

<!-- Generated from .agents/skills/pre-pr/SKILL.md by scripts/sync_agents.py. Edit the source, not this file. -->

Talk to the maintainer in their preferred language (`CLAUDE.local.md`); everything written to the repository or to GitHub is in English, without icons.

## Produces
A green `uv run just check`, a review of the whole branch by tier with every finding resolved, and a pull request title and body that pass `scripts/conventions.py` and `scripts/roadmap.py check`.

## Steps
1. **Gate.** Run `uv run just check`. Fix what belongs to the branch's issue and re-run until
   it passes. A failure that also occurs on `main` is recorded with its output, not hidden.
2. **Scope.** Run `git fetch origin`, `git diff origin/main...HEAD --stat` and
   `git log --oneline origin/main..HEAD`. Every changed file belongs to the issue in the branch
   name; unrelated changes move to their own branch. Run
   `uv run python scripts/docs_sync.py impact --base origin/main`.
3. **Synchronised artifacts.** Check each, with the command that proves it:
   - roadmap: the issue's task is `done` in `roadmap/roadmap.yaml` when the pull request
     completes it;
   - release note: a change to `crates/*/src/` or `schemas/` has a fragment in `changes/`
     (`<issue>.<type>.md`), otherwise the label `no-release-note` is justified;
   - translation: every changed page under `docs/en/` has its Italian page stamped with
     `uv run python scripts/i18n.py stamp <file>`, or the label `translation-pending`;
   - mirrors: `uv run python scripts/sync_agents.py --check` when `.agents/` changed.
4. **Tier** (AGENTS.md, Review effort proportional to risk):
   - Small (documentation, tooling, CI): the orchestrator reads the whole diff; no review
     agent.
   - Substantial (engine logic, adapters): one reviewer pass on the whole branch.
   - Contract change (schemas of artifacts, events or commands): reviewer plus the
     contract-auditor role, and the maintainer's approval before merge.
5. **Review lenses.** Each lens runs in an agent other than the implementer, read-only, with
   the diff, the issue, the specification and the ADRs named in its brief:
   - correctness and requirements: every requirement cited in the commits is implemented and
     tested; errors are handled; nothing fails silently (reviewer);
   - architecture: decision logic only in `crates/wayloop-core`, adapters translate, the
     ledger written only by the engine, model output validated before it enters the state
     (reviewer);
   - contracts: schemas, events, commands and their documentation changed together
     (contract-auditor, when contracts changed);
   - simplicity: code that need not exist, or that the codebase or a standard library already
     provides (simplifier, when available and the diff adds more than it removes).
   Run at most three in parallel. Hand the findings to the `apply-review` skill; a second pass
   only for a blocking logic defect.
6. **Draft the pull request** from `.github/PULL_REQUEST_TEMPLATE.md`:
   - title: a Conventional Commit header summarising the whole change, citing the
     specification when there is one (`feat(<scope>): <subject> [SPEC-NNNN]`);
   - body: Summary; `Closes #<issue>` (or `Refs #<issue>`); risk tier; changes by area;
     requirements and acceptance criteria covered; testing with the commands run and their
     result; review verdicts and findings rejected with their reason; documentation and
     translation status; risks and follow-ups. Tick the checklist only for what was verified.
7. **Check the description.** Write the body to a temporary file, then run
   `uv run python scripts/conventions.py pr --title "<title>" --branch <branch> --body "$(cat <file>)"`
   and `uv run python scripts/roadmap.py check --pr-body <file>`. Both pass.

## Stop and ask
- The gate fails for a reason outside the issue: report it and propose a draft pull request.
- A contract change, or a change to `crates/`, `schemas/` or an SDK: the merge waits for the maintainer; say so in the body.
- A finding is rejected or a reviewer disagrees with the specification: the maintainer decides.

## Report
- Gate result; tier; reviewers and their verdicts; findings fixed and rejected.
- The title and body drafted, and the output of the two description checks.
- What was not verified (for example CI on other platforms) and the risks.
