<!--
Title: a Conventional Commit header with a scope, for example `feat(ledger): chain events by hash`.
It becomes the commit on main and the changelog line. See docs/en/project/conventions.md.
-->

## Summary
<!-- Two to four sentences: what changes and why, for a reader who has not seen the issue. -->

## Linked issue
Closes #

## Risk tier
<!-- Small, Substantial or Contract change (AGENTS.md, review effort proportional to risk). -->

## Changes
<!-- One line per change, grouped by area: core, runtime, adapters, documentation, CI. -->

## Testing
<!-- Tests added; commands run and their result. -->

## Checklist
- [ ] Title, branch and linked issue follow the conventions
- [ ] `uv run just check` passes locally
- [ ] Documentation updated; Italian edition updated or label `translation-pending`
- [ ] Release note fragment in `changes/`, or label `no-release-note`
- [ ] Commits signed off (DCO)
