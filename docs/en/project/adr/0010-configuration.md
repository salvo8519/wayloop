# ADR 0010: One configuration file with layered precedence

Status: Accepted · 2026-11-17 (decided by the maintainer in #46)

## Context

Executors, budgets, roles, limits and tracker settings must be set per project and overridden per run.

## Decision

- **Project file.** `wayloop.toml` at the repository root, with a published schema, holds the
  settings of the project: policy (ADR 0026), budgets, roles, limits, tracker. It is committed
  and reviewed.
- **User file.** `~/.config/wayloop/config.toml` (or `$XDG_CONFIG_HOME/wayloop/config.toml`)
  holds only personal keys, marked as such in the schema: executors, model routing,
  notification channels. A project key in the user file is refused; project rules cannot be
  overridden from a personal file.
- **Environment.** Variables named `WAYLOOP_<SECTION>_<KEY>`, in upper case.
- **Precedence**, highest first: command flags, environment variables, user file (personal keys
  only), project file, defaults.
- **Transparency.** `wayloop config show` returns the effective configuration in the envelope of
  ADR 0009, with the source of every value (flag, environment, user file, project file,
  default).
- **Secrets** are never stored in either file; the files name the environment variable that
  holds them.

## Amendments

- 2026-11-17, at acceptance: the user file for personal keys, the naming of environment variables and `wayloop config show` were added.

## Consequences

One place to review the rules of a project, and one place for each person's own choices, which never leak into the repository. Unknown keys are rejected in both files. A personal key needs its marker in the schema before it can be set per user.
