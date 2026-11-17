# Behavioural case: translator

## Input
`uv run python scripts/i18n.py todo` lists `docs/it/concepts/gates.md` as stale: one paragraph of the English page now explains the approval state and quotes the command `wayloop gate approve`.

## Expected
Only the changed paragraph is updated in `docs/it/concepts/gates.md`, with the glossary terms "gate" and "stato di approvazione"; the command stays in English; the page is stamped with `uv run python scripts/i18n.py stamp docs/it/concepts/gates.md`; `i18n.py check` passes; the English source is not edited; any term outside the glossary is proposed in the report.
