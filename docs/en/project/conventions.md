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
  when the pull request completes the issue, `Refs #42` when it does not. The checks read the
  closing keywords as GitHub does: `close`, `closes`, `closed`, `fix`, `fixes`, `fixed`,
  `resolve`, `resolves` and `resolved`, in any case, with an optional colon, anywhere in the
  body. Trivial changes without an issue (a typo) carry the label `no-issue`.
- **Branch and issue agree**: the issue number in the branch is the one linked in the body.
- **One definition**: `scripts/conventions.py` owns the branch rule (`branch_issue`,
  `sprint_of`) and the closing keywords (`closed_issues`, `linked_issues`); the other scripts
  import them. The session briefing hook keeps a copy of the branch rule in shell, held equal
  by a test.
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

## Contract snapshots

A schema under `schemas/` named `v<N>.json`, `v<N>.output.json` or `v<N>.input.json` is a
snapshot (ADR 0009, C5). Once on `main` it never changes and is never deleted: a schema change
ships as a new `v<N+1>` beside the unchanged `v<N>`, and the versions of one folder run from 1
without a gap; `v0` and zero-padded versions such as `v01` are refused. The hook `.claude/hooks/protect_contracts.py` blocks the edit;
`scripts/check_schemas.py` refuses it in `just process` and in CI.

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
  genuinely needs none is labelled `no-docs-needed`, or every commit of its branch carries the
  trailer `Docs-impact: none`: a waiver on one commit does not cover the others. The local check runs as a commit-msg hook, so the trailer of the
  commit being written applies there too.

## Code quality

Linters enforce the quality of the code and how it handles errors; `just check`, pre-commit and
CI run the same commands.

- **Rust toolchain**: one version, pinned in `rust-toolchain.toml` at the root with the
  `rustfmt` and `clippy` components. rustup reads the file for every local `cargo` command, and
  CI installs the version written there, so a pull request fails only on its own changes and
  never on a lint released upstream. The weekly maintenance workflow builds, tests and runs
  clippy with the newest stable instead, and lists its findings in the maintenance issue. A
  bump is a deliberate step of the maintainer: one small `build(deps)` pull request that
  changes the `channel` in `rust-toolchain.toml` and fixes the new lints in the same change.
  The minimum supported version (`rust-version` in `Cargo.toml`) is separate from the pinned
  one: the CI job "Minimum supported Rust version" builds every crate with it on each pull
  request.

- **Rust**: `cargo clippy --workspace --all-targets -- -D warnings` with the `all` and
  `pedantic` groups and the restriction lints of the workspace manifest that refuse a panic in
  the code that ships: `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`,
  `unreachable`, `indexing_slicing`, `string_slice`, `get_unwrap`, `panic_in_result_fn`, `exit`
  and `mem_forget`. Tests may unwrap, expect, panic and index (`clippy.toml`). A failure is a
  value: the error type of the core carries the code, reason and pointer of ADR 0009 (C3).
- **Python** (`scripts/`, `tests/`, `.claude/hooks/`): `ruff check` with the rules of
  `pyproject.toml`: errors and style (`E`, `W`), likely mistakes (`F`, `B`), imports (`I`),
  modern syntax (`UP`), simpler code (`SIM`), ruff's own checks (`RUF`), an explicit encoding on
  every file read and write (`PLW1514`), and error handling: no blind `except Exception`
  (`BLE`), no silent `try`-`except`-`pass` (`S110`), exceptions raised and caught with care
  (`TRY`). `ruff format --check` keeps one format, and `mypy` in strict mode checks the types.
  `just python` runs the three; `just fmt` formats and applies the safe fixes. A finding is
  fixed, not silenced: a `noqa` or `type: ignore` needs a comment that says why.

- **One source**: every rule and value is defined once and imported where it is used. The
  process scripts share their helpers through `scripts/_*.py`, written with the standard
  library only, so that hooks and workflows can import them without the project environment:
  `_github.py` (the repository, one call of the GitHub CLI with retries, the REST API, every
  page of a list) and `_git.py` (tracked files, changed files, one call of git). The hooks
  run with the system `python3` and put `scripts/` on their import path. Scripts import each
  other by module name, as `python scripts/<name>.py` allows; the tests import the scripts and
  the hooks the same way and share their fixtures in `tests/process/conftest.py` (a throwaway
  git repository, the hooks with the scripts they need). The model of the roadmap file
  (`load`, `epics`, `tasks`, `depends_on` and the task tags `TAGS`) lives in
  `scripts/roadmap.py`; `bootstrap_github.py` and `journal.py` import it.

Problems are fixed where they appear, without the maintainer watching for them:

- **Agents**: the Claude Code Stop hook `.claude/hooks/quality-gate.sh` runs the fast checks on
  the files changed against `HEAD` (staged, unstaged and untracked): `rustfmt --check` and
  `cargo clippy -p <crate> --all-targets -- -D warnings` for the touched crates, `ruff check`,
  `ruff format --check` and `mypy` for changed Python. It runs when the session and when a
  subagent stop. Findings block the end of the turn, so the agent fixes them first; the same
  findings never block twice in a row, and a session is blocked at most three times. The checks
  share a budget of 60 seconds; a check that does not finish in time, or a missing tool, is
  reported without blocking.
- **Commits**: pre-commit applies `cargo fmt`, `ruff check --fix` and `ruff format` to the
  staged files. A commit with unformatted code stops once with the files already fixed: stage
  them and commit again. CI runs the same tools in check mode.
- **Coverage**: CI runs the Rust tests instrumented by cargo-llvm-cov and the process tests with
  pytest-cov, and writes both line coverage tables to the job summary. The line coverage of
  each pure crate, `wayloop-core` and `wayloop-schemas`, must stay on its own at or above the
  floor in the `justfile` (`pure_coverage_floor`, 95 percent). They hold the decision logic
  and the contract types and perform no I/O, so every line can be tested without mocks. The
  floor stays below 100 because a `const fn` that runs only at compile time is counted as not
  covered, and because mutation testing judges the strength of the tests better than the last
  few percent would. `just coverage` measures locally (it needs the `llvm-tools-preview`
  component); `just coverage 100` tries a floor.
- **Mutation testing**: every week the maintenance workflow runs cargo-mutants on `wayloop-core`
  and `wayloop-schemas` (`just mutants` locally). A mutant that survives means a change in the
  logic that no test notices; the surviving mutants are listed in the maintenance issue.
- **Failures**: a failure of CI or of the documentation on `main`, or of a scheduled workflow,
  opens or updates one issue labelled `bug` per workflow with the failing jobs and the link to
  the log, and the next green run closes it (`scripts/failure_issue.py`). A fix references such
  an issue with `Refs #<issue>`, not `Closes`, because the green run closes it. The session
  briefing lists open `bug` and `maintenance` issues before anything else.
- **Dependency updates**: Dependabot pull requests are merged by the session agent in
  pull-request triage once the required checks are green; Rust crates and ruff wait for review
  (`docs/en/project/dependencies.md`).

## House style

No icons, emoji or decorative symbols in any file, commit message, issue or pull request.
`scripts/check_style.py` enforces it. Corporate, precise English: short sentences, explicit
terms, no marketing superlatives.
