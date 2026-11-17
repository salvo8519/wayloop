---
name: roadmap-update
description: Proposes the roadmap change that a request implies (new task, new epic, moved task, changed scope, dropped work) and applies it through scripts/roadmap.py once the maintainer confirms. Use when a request adds, changes or drops planned work, or when the maintainer asks to update the roadmap.
---

<!-- Generated from .agents/skills/roadmap-update/SKILL.md by scripts/sync_agents.py. Edit the source, not this file. -->

Talk to the maintainer in their preferred language (`CLAUDE.local.md`); everything written to the repository or to GitHub is in English, without icons.

The plan lives in `roadmap/roadmap.yaml`; GitHub holds the state of each issue. Milestones,
epics and issues are never created or edited on GitHub by hand.

## Produces
A roadmap change proposed to the maintainer and, after confirmation, applied: issues created
with `scripts/roadmap.py new`, `roadmap/roadmap.yaml` updated and rendered, GitHub in step with
the file, `roadmap.py check` and `roadmap.py links` passing.

## Steps
1. **State, deterministically.** Run `uv run python scripts/roadmap.py check`,
   `uv run python scripts/roadmap.py sync` and `uv run python scripts/roadmap.py links`. Any
   difference that exists before this change is reported separately, with the command that
   fixes it, and is not mixed into the proposal.
2. **Find what is already planned.** `uv run python scripts/roadmap.py list --open`, and
   `--milestone M<n>` for the milestone in question. Search titles for the request; read the
   matching issues and the specifications they reference (`specs/NNNN-*/spec.md`).
3. **Classify the request**, exactly one of:
   - **Already planned**: an open task covers it. Name it; say whether its scope must change.
   - **New task** in an existing epic, or **new epic** when no epic fits.
   - **Moved**: a task changes milestone.
   - **Changed scope**: the title, the specification or the dependencies of a task change.
   - **Dropped**: planned work will not be done.
   - **No impact**: a question, or work inside the scope of the current issue. Say so in one
     line and stop.
4. **Propose**, in one short block, before changing anything: milestone, epic, title (the
   outcome as a sentence in sentence case, no type prefix, no full stop, unique across the
   roadmap), labels (one `area:*`, plus `parallel`, `spec`, `adr`, `contract`, `maintainer`
   or `needs-decision` where they apply), the specification (existing `SPEC-NNNN`, or
   "specification needed" for engine behaviour), dependencies (`## Depends on`), and the
   reason for the milestone. Follow the issue body of `docs/en/project/conventions.md`
   (Issues): `Part of #<epic>.`, outcome and advantage, scope checklist, acceptance criteria,
   verification technique, references, `## Depends on`, definition of done.
5. **Wait for confirmation.** Apply nothing until the maintainer confirms, unless the
   maintainer has already stated the exact change.
6. **Apply** what was confirmed:
   - new task: write the body to a temporary file, then
     `uv run python scripts/roadmap.py new --milestone M<n> --epic "<epic>" --title "<title>"
     --labels <labels> [--spec NNNN] --body <file>`. This is the only way issues are created;
   - moved or changed scope: edit the task in `roadmap/roadmap.yaml`, run
     `uv run python scripts/roadmap.py render`, then
     `uv run python scripts/roadmap.py sync --apply-milestones`; update the issue title or
     body on GitHub with `gh issue edit` when it changed;
   - dropped: close the issue as not planned with a comment that says why
     (`gh issue close <n> --reason "not planned" --comment "<reason>"`), then
     `uv run python scripts/roadmap.py sync --write`. The task stays in the file with status
     `dropped`, so the history remains visible.
7. **Verify.** `uv run python scripts/roadmap.py check` and
   `uv run python scripts/roadmap.py links` pass. Native blocked-by dependencies match the
   `## Depends on` lines.
8. **Record.** The file change goes into the current pull request when it belongs to its
   issue; otherwise into its own `chore(roadmap): <subject>` pull request.

## Stop and ask
- Before step 6, always: the proposal is the maintainer's decision.
- A change that moves work between milestones already in progress, or drops a task with an
  open pull request.
- A title that would duplicate an existing task or epic, or a request that fits two epics.
- Engine behaviour without a specification: propose the specification task first.

## Report
- Done: classification, issues created or changed (numbers and titles), file changes, checks
  run and their result.
- Not done or not verified: changes not confirmed, GitHub unreachable, pre-existing drift left
  as it was.
- Risks: effect on the order of work, the active sprint and the dependencies of other tasks.
