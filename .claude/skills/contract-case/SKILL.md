---
name: contract-case
description: Writes the golden contract cases of an engine command before its code - a minimal and a maximal input, one case per error reason, one input with several independent errors - with expected envelopes, error reports and exit codes taken from the specification, and never edits a case already released. Use when a command is added or gains a field or an error code, or when an acceptance criterion of a contract needs coverage.
---

<!-- Generated from .agents/skills/contract-case/SKILL.md by scripts/sync_agents.py. Edit the source, not this file. -->

Talk to the maintainer in their preferred language (`CLAUDE.local.md`); everything written to the repository or to GitHub is in English, without icons.

A contract case is a fixed input sent to a command and the result an agent can rely on. It is
part of the contract (ADR 0009 C5): once released it never changes, and the tool definition of
the command takes its examples from it (ADR 0024).

## Produces
New case files for one command and version, in the folder for contract cases fixed by the
plan of SPEC-0004 (written below as `<cases>/<command>/v<N>/<case>.json`); each one failing
before the code exists for the reason "command missing" or "behaviour missing"; a commit
`test(<scope>): ...` that precedes the code; a coverage table.

## Steps
1. **Sources, deterministically.** Read `specs/NNNN-<slug>/spec.md` (requirements and
   acceptance criteria of the command), `specs/0004-cli-and-contracts/spec.md`,
   `docs/en/project/adr/0009-contract-guidelines.md` and the contract on paper of the
   `add-command` skill (fields, defaults, error table, exit codes). List the existing cases of
   the command and which of them are released:
   `git ls-tree -r --name-only origin/main -- <cases>/<command>/`.
2. **Released cases are frozen.** A case that exists on `origin/main` is never edited, renamed
   or deleted; check one path with `git cat-file -e origin/main:<path>`. The hook
   `.claude/hooks/protect_contracts.py` blocks edits to released snapshots; released cases
   follow the same rule. Never work around the hook (for example by changing
   `WAYLOOP_CONTRACT_BASE`). New behaviour gets a new case; a released case that is wrong is a
   question for the maintainer and, if confirmed, a new command version with new cases.
3. **Choose the set.** For the command version under test, one file per case:
   - `minimal`: only the required fields; asserts every entry of `meta.defaults_applied`
     (pointer, value, reason);
   - `maximal`: every optional field set, each enum kind used once, at the deepest level
     allowed (four);
   - one case per error reason the command can raise (`<reason>-<field>`), with the exact
     code, reason, pointer and expected values;
   - `three-errors`: three independent errors in one input, all listed in `errors` in one
     pass, the first equal to the top level (SPEC-0004 AC1);
   - `null-is-absent`: an optional field sent as null gives the same result as `minimal`;
   - `misspelt-key`: an unknown field close to a real one gives reason `unknown_field` and a
     `fix` that names the real field;
   - for a command that changes state: `stale-revision` (reason `stale`) and `dry-run` (no
     change applied, planned changes listed).
4. **Write each case** with: the requirement it covers (`SPEC-NNNN Rk`), the arguments and the
   exact input sent, and the expectation:
   - success: exit code 0, `command`, `meta.command_version`, the `data` fields asserted by
     JSON Pointer, and the lengths of the lists so that no extra entry passes unnoticed;
   - refusal: the exit code of C7 (1 for input errors), and the error report asserted by JSON
     Pointer: `/code`, `/reason`, `/pointer`, `/pointers`, `/expected`, `/errors/<i>/pointer`.
   Do not assert `engine_version` or the wording of `message`; assert `fix` only where it
   names a field. Data are synthetic: no real person, repository secret or token.
5. **Expected values come from the specification and the ADRs**, never from running the code
   under test. If the specification does not decide a value, stop and ask.
6. **Run them** with the narrowest command (`cargo test -p wayloop-cli <filter>` once the
   contract test harness exists) and confirm that each fails for the expected reason, not for
   a malformed fixture. A case that passes before the code exists proves nothing: rewrite it
   or explain why.
7. **Check and commit.** Run `uv run python scripts/check_style.py` and `uv run just check`
   for the parts that must stay green; commit with the `commit` skill as
   `test(<scope>): contract cases for <command> v<N> [SPEC-NNNN Rk]`, signed off, before any
   code of the command.

## Stop and ask
- The plan of SPEC-0004 does not yet fix the folder or the format of contract cases: ask
  before writing the first one; do not invent a layout.
- The specification leaves an expected value, a code or an exit code open.
- A released case looks wrong, or a change would make one fail: that is a breaking change for
  the maintainer to decide, never an edit.
- A case cannot be made to fail for the expected reason.

## Report
A table: case, requirement, kind (minimal, maximal, error reason, several errors, state),
status (failing as expected, passing, not written).
- Done: cases added and their commit.
- Not done or not verified: reasons without a case, cases not run, harness missing.
- Risks: values the specification leaves implicit, released cases that constrain the change.
