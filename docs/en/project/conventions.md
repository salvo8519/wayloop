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

`feat`, `fix`, `docs`, `refactor`, `test`, `ci`, `chore`, `perf`, `build`.

## Scopes

`core`, `runtime`, `cli`, `ledger`, `tracker`, `survey`, `python`, `typescript`, `mcp`,
`agents`, `docs`, `project`.

## House style

No icons, emoji or decorative symbols in any file, commit message, issue or pull request.
`scripts/check_style.py` enforces it. Corporate, precise English: short sentences, explicit
terms, no marketing superlatives.
