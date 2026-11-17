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
  `pyproject.toml` and never reach users.

## How the project learns that an update is needed

| Signal | Tool | Cadence |
| --- | --- | --- |
| New versions of crates, Python packages and GitHub Actions | Dependabot version updates, grouped per ecosystem | Weekly |
| Known vulnerabilities | Dependabot security alerts; `cargo deny check advisories`; `pip-audit` | Immediate; every pull request and weekly |
| A dependency with a vulnerability or a disallowed licence added by a pull request | Dependency review | Every pull request |
| Vulnerable patterns in our code and workflows | CodeQL (Python, Rust, GitHub Actions) | Every push, pull request, weekly |
| Supply-chain practices of the project | OpenSSF Scorecard | Weekly and on `main` |
| Breakage coming with the next compatible versions | Weekly maintenance: tests with upgraded lock files | Weekly |
| Unmaintained, yanked or wildcard crates, unknown sources | `cargo deny` | Every pull request |
| Unsafe workflows: injection, permissions, persisted credentials, unpinned actions | zizmor | Every pull request |
| Workflow syntax, runner labels, shell mistakes | actionlint with shellcheck | Every pull request |
| Rust dependencies declared but not used | cargo-machete | Every pull request |
| Broken links in the documentation | lychee | Weekly |
| Required checks that no longer match the ruleset of `main` | `scripts/required_checks.py remote` | Weekly |

A failure of the weekly maintenance opens one issue labelled `maintenance`, or comments on the
open one. Cargo tools pinned in the workflows (`tool: <name>@<version>` and the
`cargo install --locked` fallback next to it) are not updated by Dependabot: the maintainer
checks them at each weekly maintenance and updates both lines together in one `ci(deps)` pull
request.

## When to update

| Kind of update | When | How |
| --- | --- | --- |
| Security fix | Within seven days of the alert, immediately for a critical one | Dependabot security pull request, or a manual `fix(deps)` |
| Patch and minor versions | Weekly, one grouped pull request per ecosystem | Merge after a green CI |
| Major versions | Within one month, after reading the upstream changelog | One pull request per dependency |
| Rust toolchain for the minimum supported version | Only on purpose, with `rust-version` in `Cargo.toml` | `build(deps)` pull request with the reason |

A release is never made with an open security alert, a failed weekly maintenance run or a
dependency pull request older than one month.

## GitHub Actions are pinned to commits

A tag such as `v7` can be moved to other code by whoever controls the action; a commit hash
cannot. Every `uses:` names a commit with the tag as a comment;
`python scripts/pin_actions.py` pins new ones and `--check` fails CI on any action left
unpinned. Dependabot updates the hash and the comment together.
