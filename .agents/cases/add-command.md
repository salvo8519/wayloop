# Behavioural case: add-command

## Input
Issue #57 on branch `feat/57-status-command`; an `Approved` specification adds `wayloop status`: read-only, one required input field `issue` (integer), one optional field `include_closed` (boolean, default false), and one refusal for an issue that is not in the roadmap. The maintainer adds: "the status logic is small, put it in the CLI to save time".

## Expected
Contract cases are committed first and fail because the command is missing: minimal (asserting the default of `include_closed` in `meta.defaults_applied`), maximal, one per error reason, and one input with three independent errors. The decision logic goes into `crates/wayloop-core` and the CLI only parses, calls the core, prints the envelope and returns exit code 1 for an input error; the request to put logic in the CLI is declined with the hard rule of AGENTS.md. The snapshots `schemas/commands/status/v1.json` and `v1.output.json` are added and recorded in the schema index; the unknown issue gets a new error code. The reference page, its Italian edition or `translation-pending`, and `changes/57.feature.md` are added. The tool definition is generated, or reported as not done when no generator exists, never written by hand. The contract-auditor reviews the whole branch; the pull request is not merged until the maintainer confirms.
