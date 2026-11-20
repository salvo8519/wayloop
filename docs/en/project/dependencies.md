---
title: Dependency policy
description: Which dependencies Wayloop accepts, how updates arrive, when they are applied and how the project learns that one is needed.
---

# Dependency policy

Wayloop drives agents that write code in other people's repositories, so every dependency is
part of what its users trust. This page states what is accepted, how updates arrive, when they
are applied and who decides. Every tool listed is free for public open-source repositories.

## What may be added

- **Core (`wayloop-core`)**: as few crates as possible, and no network, file-system, process
  or async-runtime crates: decision logic has no I/O (ADR 0001, ADR 0006). A new runtime
  dependency of the core needs an ADR that names its maintainer, its licence, its scope, the
  absence of I/O and a fallback if it is abandoned. Upgrades of core dependencies are always
  reviewed, never merged automatically.
- **Runtime and adapters**: an actively maintained project with a release in the last twelve
  months, a licence on the allow-list of `deny.toml` and of the Security workflow, and a reason
  that a few lines of our own code would not cover.
- **Tooling**: the same rule; development tools live in the `dev` and `docs` groups of
  `pyproject.toml`, or in the `devDependencies` of `package.json` for a tool published only on
  npm, pinned to an exact version with its lock file and installed with
  `npm ci --ignore-scripts`. They never reach users.
- **Licences**: two lists, one rule. `cargo deny` (`deny.toml`) checks the normal Rust
  dependencies; it leaves out crates used only by tests. Dependency review (the Security
  workflow) checks every dependency a pull request adds, development dependencies and Python
  packages included. Its list may hold more licences, for Python packages, never fewer:
  `scripts/check_licences.py` refuses a licence allowed in `deny.toml` and missing from the
  workflow. A new licence is added to both lists in the same change.

## How the project learns that an update is needed

| Signal | Tool | Cadence |
| --- | --- | --- |
| New versions of crates, Python packages, npm tools and GitHub Actions | Dependabot version updates (`cargo`, `uv` with its lock file, `npm` with its lock file, `github-actions` for the workflows and the composite actions of `.github/actions`), minor and patch grouped per ecosystem | Weekly |
| Known vulnerabilities | Dependabot security alerts; `cargo deny check advisories`; `pip-audit` | Immediate; every pull request and weekly |
| A dependency with a vulnerability or a disallowed licence added by a pull request | Dependency review | Every pull request |
| Vulnerable patterns in our code and workflows | CodeQL (Python, Rust, GitHub Actions) | Every push, pull request, weekly |
| Supply-chain practices of the project | OpenSSF Scorecard | Weekly and on `main` |
| Breakage coming with the next compatible versions | Weekly maintenance: tests with upgraded lock files | Weekly |
| Tests that miss a change in the decision logic | Weekly maintenance: mutation testing of `wayloop-core` (cargo-mutants) | Weekly |
| A failure of CI on `main` or of a scheduled workflow | Failure issues workflow: one issue labelled `bug` per workflow | Every completed run |
| Unmaintained, yanked or wildcard crates, unknown sources | `cargo deny` | Every pull request |
| Unsafe workflows: injection, permissions, persisted credentials, unpinned actions | zizmor | Every pull request |
| Workflow syntax, runner labels, shell mistakes | actionlint with shellcheck | Every pull request |
| Rust dependencies declared but not used | cargo-machete | Every pull request |
| Duplicated code: none in the pure crates beyond a shrinking baseline, at most 1 percent elsewhere | jscpd (`just dup`) | Every pull request |
| Broken links in the documentation | lychee | Weekly |
| Required checks that no longer match the ruleset of `main` | `scripts/required_checks.py remote` | Weekly |

A failure of the weekly maintenance opens one issue labelled `maintenance`, or updates the open
one (its body with the latest report and the surviving mutants, a comment with the run); the
next green run closes it. A failure of CI, Docs or Security on `main` (on a push or a
schedule), or of a scheduled run of Roadmap sync, does the same with one issue labelled `bug`
per workflow (`scripts/failure_issue.py`). The session briefing lists these issues first.

Cargo tools pinned in the workflows (`tool: <name>@<version>` and the `cargo install --locked`
fallback next to it) are not updated by Dependabot: the maintainer checks them at each weekly
maintenance and updates both lines together in one `ci(deps)` pull request.

## When to update

| Kind of update | When | How |
| --- | --- | --- |
| Security fix | Within seven days of the alert, immediately for a critical one | Dependabot security pull request, or a manual `fix(deps)` |
| Patch and minor versions | Weekly, one grouped pull request per ecosystem, at least 7 days after the release | GitHub Actions and Python tools: merged by the session agent in pull-request triage once the required checks are green; Rust crates and ruff: merged after review |
| Major versions | Within one month, after reading the upstream changelog | One pull request per dependency |
| Rust toolchain of builds and CI (`rust-toolchain.toml`) | After a weekly maintenance run reports the newest stable, when the maintainer decides | One `build(deps)` pull request that changes the pinned version and fixes the new lints |
| Rust toolchain for the minimum supported version | Only on purpose, with `rust-version` in `Cargo.toml` | `build(deps)` pull request with the reason |

A release is never made with an open security alert, a failed weekly maintenance run or a
dependency pull request older than one month.

## Who merges dependency updates

Dependency pull requests are small (AGENTS.md, Review effort). The session agent merges a
Dependabot pull request of GitHub Actions or of the Python development tools during
pull-request triage (the `pr-triage` skill) once the required checks are green. Two kinds wait
for review: Rust crates (`cargo`), because they change what ships, and ruff, because it is
pinned to a minor range and a new minor version can change the lint rules. A merge by an agent
is a normal merge, so the push workflows on `main` (CI, Docs, Security and the failure issues)
run on it; a merge made by the `GITHUB_TOKEN` of a workflow would not start them.

Dependabot proposes a new version only after a cooldown of 7 days (`cooldown` in
`.github/dependabot.yml`): a compromised or broken release is usually yanked within that time.
Security updates are not delayed by the cooldown.

## GitHub Actions are pinned to commits

A tag such as `v7` can be moved to other code by whoever controls the action; a commit hash
cannot. Every `uses:` names a commit with the tag as a comment;
`python scripts/pin_actions.py` pins new ones and `--check` fails CI on any action left
unpinned. Dependabot updates the hash and the comment together.
