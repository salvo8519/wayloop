# Conventions

Names and links are structured so that people and agents can follow any piece of work from the
plan to the release, and an agent can take over an issue without asking how things are named.
Every rule on this page is checked by a script, by CI or by both.

## The chain

| Level | Identifier | Example | References |
| --- | --- | --- | --- |
| Milestone | `M<n> · <name>` | `M1 · Engine core` | release gate |
| Epic | parent issue `#<n>` | `#22 Ledger` | milestone |
| Task | sub-issue `#<n>` | `#12 Approved specification of the ledger` | parent epic, specification |
| Specification | `SPEC-<NNNN>`, requirement `R<k>` | `SPEC-0003 R4` | issue |
| Branch | `<type>/<issue>-<slug>`, or `sprint/SNN/<issue>-<slug>` in a sprint lane | `feat/12-ledger-hash-chain` | issue |
| Commit | `<type>(<scope>): <subject>` | `feat(ledger): chain events by hash [SPEC-0003 R2]` | specification, issue (trailer) |
| Pull request | same format as a commit | `feat(ledger): chain events by hash` | `Closes #12` |
| Merge commit | pull request title and number | `feat(ledger): chain events by hash (#57)` | pull request |
| Release note | `changes/<issue>.<type>.md` | `changes/12.feature.md` | issue |
| Release | tag `v<semver>` | `v0.1.0` | CHANGELOG and release notes |

The GitHub number (`#12`) is the key of an issue on the tracker. It is unique, permanent, and
GitHub links it automatically in commits, pull requests and comments, so branches, commits,
pull requests and release notes cite it. Pull request numbers share the same sequence. The
artifact identifier of ADR 0008 (`TASK-0107`) is the identity of the artifact inside the engine:
it lives in the structured block of the artifact, and the engine records and checks the pair of
identifier and issue number. Nothing else cites it, so the two schemes cannot drift.

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
- **Hierarchy**: every task is a sub-issue of an epic, every epic belongs to a milestone and
  carries the label `epic`. The roadmap file `roadmap/roadmap.yaml` is the source of this
  structure: each task names its issue and, where one exists, its specification. Create issues
  with `python scripts/roadmap.py new --milestone M2 --epic "<epic>" --title "<title>"`, which
  sets the milestone and labels on GitHub, links the task to its epic and adds it to the file.
- **Body**: it starts with `Part of #<epic>.`, then the outcome and the advantage it brings,
  the scope as a checklist, acceptance criteria, the verification technique, references to
  specifications and ADRs, `## Depends on` with one `- #<n>` line per blocking issue, and the
  definition of done. Every issue under `## Depends on` is also a native blocked-by dependency.
- **Checks**: CI refuses a pull request that closes an issue missing from the roadmap or not
  marked `done` in it, and a specification that no task references. `roadmap.py sync` copies
  issue states from GitHub; `roadmap.py links` checks that every planned issue has the title,
  milestone, labels, parent epic, parent line and dependencies of the plan. The workflow
  `Roadmap sync` runs both every day and on issue events.
- **Plans in public pages**: the README files and every documentation page outside
  `project/`, in English and Italian, describe what exists. What comes next is stated only in
  the roadmap page, generated from the plan; a public page links it instead. A page for a part
  that does not exist says so in one line and links the roadmap page.
  `scripts/docs_sync.py claims` checks it in `just process`, pre-commit and CI: it fails on
  plans in prose ("will", "planned", "coming soon", "on the roadmap", a milestone number such
  as "M2", and the Italian equivalents) and ignores code, link targets and HTML comments.

## Status

The open or closed state of an issue, plus two labels: `in-progress` when the work is claimed
and a branch exists, and `blocked` with a comment that says on what. An agent that takes an
issue adds `in-progress`; an agent that cannot continue adds `blocked` and its reason, or opens
a question with `needs-decision`.

## Branches

`<type>/<issue>-<slug>`, lower case, words separated by hyphens, at most five words in the
slug: `fix/61-replay-order`, `docs/48-mcp-guide`. The type is the Conventional Commit type of
the main change. One branch per issue; branches are short-lived and deleted after merge. An
issue worked in a sprint lane uses `sprint/SNN/<issue>-<slug>` instead: on those branches CI
also checks that the pull request changes only the files of the lane
([sprints](https://github.com/salvo8519/wayloop/blob/main/sprints/README.md)). Exempt:
`dependabot/*`, `release/v*`, and `revert-<n>-*`, the branch GitHub creates for the
Revert button.

## Commits

```text
<type>(<scope>)[!]: <subject> [SPEC-NNNN Rk]

<body: what and why, wrapped at 72 characters>

Refs: #42
Docs-impact: none
Signed-off-by: Name Surname <email>
```

- **Type**: `feat` (new capability), `fix` (wrong behaviour), `perf`, `refactor` (no behaviour
  change), `docs`, `test`, `build` (packaging, dependencies), `ci`, `chore`, `revert`.
- **Scope**: required (except for `revert`), one of: engine `core`, `runtime`, `ledger`,
  `state`, `tracker`, `survey`, `schemas`, `security`; adapters `cli`, `python`, `typescript`,
  `mcp`, `plugins`; documentation and project `docs`, `i18n`, `brand`, `specs`, `adr`,
  `agents`, `project`, `roadmap`; build and delivery `ci`, `deps`, `release`, `scripts`,
  `benchmark`. New scopes are added in `scripts/conventions.py` with this page in the same
  pull request.
- **Subject**: imperative, lower case first word, no full stop, header at most 72 characters.
- **Breaking change**: `!` after the scope and a `BREAKING CHANGE:` paragraph in the body; the
  pull request carries the label `breaking-change`.
- **Trailers**: `Signed-off-by` always (`git commit -s`); `Refs: #<n>` to link the issue;
  `Docs-impact: none` when the change genuinely needs no documentation.
- The commit-msg hook and the pull request workflow run the same code,
  `scripts/conventions.py`.

## Pull requests

- **Title**: a Conventional Commit header that summarises the whole change. Pull requests are
  squash-merged with their title, so the title becomes the commit on `main` and the line in
  `CHANGELOG.md`.
- **Body**: the template sections, and the linked issue with a closing keyword: `Closes #42`
  when the pull request completes the issue, `Refs #42` when it does not. Trivial changes
  without an issue (a typo) carry the label `no-issue`.
- **Branch and issue agree**: the issue number in the branch is the one linked in the body.
- **Size**: one issue, one concern. A change to `wayloop-core` and an unrelated workflow change
  are two pull requests.
- **Roadmap**: the pull request marks its task `done` in `roadmap/roadmap.yaml` in the same
  change.
- **Merge**: the agent that opened the pull request merges it when the gate of its tier holds;
  changes to `crates/`, `schemas/` or an SDK wait for the maintainer's confirmation
  ([AGENTS.md](https://github.com/salvo8519/wayloop/blob/main/AGENTS.md#merging)).
- **Stacked pull requests**: a pull request built on another one targets that branch; before
  it is merged it is rebased onto `main` with its own commits only, and retargeted to `main`.
- **Failures of external analyses**: CodeQL, dependency review and the other external analyses
  are re-run first; merge waits until the failure is understood. When the failure is
  demonstrably on the side of GitHub, of a vendor or of a repository setting not yet enabled,
  and every required check is green, the merge proceeds and the pull request records why.

## Labels

Labels that need judgement are added by the author; `area:*` labels are applied automatically
from the changed paths. Each label below either opens or waives a check, and a test keeps this
list, the workflows and the roadmap file in agreement.

| Label | Effect |
| --- | --- |
| `epic` | marks a parent issue of the roadmap |
| `task`, `bug`, `spec-request` | type of an issue, applied by the issue form |
| `spec`, `adr`, `contract` | the issue writes a specification, an ADR or changes a contract |
| `maintainer`, `needs-decision` | the issue waits for the maintainer; a question is open |
| `parallel` | the task can run in parallel with others of its epic |
| `in-progress`, `blocked` | status, see above |
| `good first issue` | suitable for a first contribution |
| `no-release-note` | waives the release note fragment for a change visible to users |
| `no-docs-needed` | waives the documentation impact check |
| `translation-pending` | waives the Italian translation check; the translation follows |
| `no-issue` | a trivial pull request without an issue |
| `breaking-change` | the pull request breaks a contract or a command |
| `docs-sync` | asks the automated documentation sync for this pull request |
| `maintenance` | the issue of the weekly maintenance run |

## Release notes

A change to `crates/*/src/` or `schemas/` adds one fragment `changes/<issue>.<type>.md`, with
type `breaking`, `feature`, `improvement` or `fix`, unless the pull request is labelled
`no-release-note`; CI runs `towncrier check` against the base branch.

## Documentation in sync

- `scripts/docs_sync.py coverage`: every ADR has its row in the ADR index, its entry in the
  navigation and a valid status line; every specification has a valid status; every page of
  the navigation exists and every page is in the navigation.
- `scripts/docs_sync.py claims`: public pages state no plans in prose (rule "Plans in public
  pages" above).
- `scripts/docs_sync.py impact`: a change to the engine (every crate but the command line),
  the command line, the schemas, the guards of the pure crates (`scripts/check_core_io.py`,
  a `clippy.toml`) or a process script comes with the matching documentation. A change that
  genuinely needs none is
  labelled `no-docs-needed`, or its latest commit carries the trailer `Docs-impact: none`.

## House style

No icons, emoji or decorative symbols in any file, commit message, issue or pull request.
`scripts/check_style.py` enforces it. Corporate, precise English: short sentences, explicit
terms, no marketing superlatives.
