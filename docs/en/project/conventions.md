# Conventions

Names and links are structured so that people and agents can follow an issue from the plan to
the code.

| Object | Convention | Example |
| --- | --- | --- |
| Branch | `<type>/<issue>-<slug>` | `feat/12-ledger-hash-chain` |
| Commit and pull request title | Conventional Commits with a required scope: `<type>(<scope>): <subject>` | `feat(ledger): chain events by hash` |
| Pull request body | `Closes #<issue>` and the requirements it implements | `Closes #12` |
| Merge | Squash | |
| Release note | One fragment in `changes/` for every user-facing change | `changes/12.feature.md` |
| Sign-off | Developer Certificate of Origin: `git commit -s` | |

## Types

`feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `build`, `ci`, `chore`, `revert`.

## Scopes

Engine: `core`, `runtime`, `ledger`, `state`, `tracker`, `survey`, `schemas`, `security`.
Adapters: `cli`,
`python`, `typescript`, `mcp`, `plugins`. Documentation and project: `docs`, `i18n`, `brand`,
`specs`, `adr`, `agents`, `project`, `roadmap`. Build and delivery: `ci`, `deps`, `release`,
`scripts`, `benchmark`. The list is enforced by `scripts/conventions.py`, which the commit-msg hook and the
pull request workflow both run.

## Traceability

- A pull request closes exactly one issue, and that issue is in `roadmap/roadmap.yaml`.
- The pull request marks its task `done` in the roadmap in the same change; CI checks it.
- Commits and titles cite the specification when there is one:
  `feat(ledger): chain events by hash [SPEC-0002 R3]`.

## Issues

- **Title**: the outcome, as a short sentence in sentence case, without a type prefix and
  without a full stop: `Ledger replay gives the same state on every platform`, not
  `feat: ledger replay` or `[FEAT] replay`. A title may start with a command name
  (`wayloop init creates ...`). Titles of tasks and epics are unique across the roadmap, so
  that a title identifies one piece of work. `scripts/roadmap.py check` enforces both rules.
- **Type**: chosen with the issue form, which applies the label: `task` for planned work,
  `bug` for a defect, `spec-request` for a proposed capability. Blank issues are disabled.
- **Area**: one `area:*` label. Areas decide which issues can run in parallel; the area list of
  the task form matches the `area:*` labels of the roadmap, and a test checks it.
- **Hierarchy**: every task is a sub-issue of an epic, every epic belongs to a milestone. The
  roadmap file is the source of this structure; maintainers create planned tasks with
  `python scripts/roadmap.py new`, which sets the milestone and labels on GitHub and adds the
  task to the file.
- **Body**: outcome and the advantage it brings, scope as a checklist, acceptance criteria, the
  verification technique, references to specifications and ADRs, dependencies, and the
  definition of done.

## House style

No icons, emoji or decorative symbols in any file, commit message, issue or pull request.
`scripts/check_style.py` enforces it. Corporate, precise English: short sentences, explicit
terms, no marketing superlatives.
