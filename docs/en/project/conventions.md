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

Engine: `core`, `runtime`, `ledger`, `state`, `tracker`, `survey`, `schemas`. Adapters: `cli`,
`python`, `typescript`, `mcp`, `plugins`. Documentation and project: `docs`, `i18n`, `brand`,
`specs`, `adr`, `agents`, `project`, `roadmap`. Build and delivery: `ci`, `deps`, `release`,
`scripts`. The list is enforced by `scripts/conventions.py`, which the commit-msg hook and the
pull request workflow both run.

## Traceability

- A pull request closes exactly one issue, and that issue is in `roadmap/roadmap.yaml`.
- The pull request marks its task `done` in the roadmap in the same change; CI checks it.
- Commits and titles cite the specification when there is one:
  `feat(ledger): chain events by hash [SPEC-0002 R3]`.

## House style

No icons, emoji or decorative symbols in any file, commit message, issue or pull request.
`scripts/check_style.py` enforces it. Corporate, precise English: short sentences, explicit
terms, no marketing superlatives.
