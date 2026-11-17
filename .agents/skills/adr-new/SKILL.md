---
name: adr-new
description: Records an architecture decision as a new ADR with context, options, decision and consequences, adds it to the ADR index and the site navigation, and passes the documentation coverage check. Use when a change affects module boundaries, adapters, schemas, dependencies, a convention or a principle of the constitution, or when the maintainer asks for an ADR.
---

Talk to the maintainer in their preferred language (`CLAUDE.local.md`); everything written to the repository or to GitHub is in English, without icons.

A decision is recorded once and changed only by a new record that supersedes it
(`docs/en/project/adr/index.md`). ADR pages are kept in English only (`i18n/it.yaml`).

## Produces
`docs/en/project/adr/NNNN-<slug>.md` with status `Proposed`, its row in
`docs/en/project/adr/index.md`, its entry in the "Decisions (ADR)" navigation of
`zensical.toml`, and `scripts/docs_sync.py coverage` passing. On the maintainer's acceptance,
status `Accepted`; when it replaces a record, that record becomes `Superseded by ADR NNNN`.

## Steps
1. **Context, deterministically.** Run `ls docs/en/project/adr/` and take the highest number
   plus one. Read `docs/en/project/adr/index.md` and the records the decision touches; read
   `specs/constitution.md`. Find the roadmap task that tracks the decision
   (`uv run python scripts/roadmap.py list --open`); if none exists, propose one with the
   `roadmap-update` skill (label `adr`).
2. **Overlap.** If an existing record already decides the question, the change is a
   superseding record or no record at all. Say which, with the number, and stop if it is none.
3. **Draft** with the `architect` role (`.agents/agents/architect.md`), passing the problem,
   the constraints and the records to read. The file follows the existing records:
   - title `# ADR NNNN: <decision as a short sentence>`;
   - status line, exactly `Status: Proposed · YYYY-MM-DD (milestone M<n> · <name>; decision
     tracked in #<issue>)`; the coverage check validates it;
   - `## Context`: the forces and the options considered, each with why it was chosen or
     rejected;
   - `## Decision`: what is decided, in rules a reviewer can check;
   - `## Consequences`: what becomes easier, what becomes harder, what follows (specifications,
     tasks, migrations).
4. **Index.** Add the row `| [NNNN](NNNN-<slug>.md) | <decision> | Proposed |` to
   `docs/en/project/adr/index.md`, in numeric order.
5. **Navigation.** Add `"project/adr/NNNN-<slug>.md",` to the "Decisions (ADR)" list of
   `zensical.toml`, in numeric order, with the indentation of the surrounding entries.
6. **Verify.** Run `uv run python scripts/docs_sync.py coverage` (index row, navigation entry
   and status line agree) and `uv run python scripts/check_style.py`. Then `uv run just docs`
   when the build tools are available; otherwise record it as not verified.
7. **Supersede**, when the record replaces another: set the old status line to
   `Status: Superseded by ADR NNNN`, update its row in the index, and link both records to each
   other.
8. **Commit** with scope `adr`, for example `docs(adr): record <decision>`, signed off.
9. **Acceptance.** Present the decision and its consequences to the maintainer. Only on their
   explicit acceptance change `Proposed` to `Accepted` in the record and in the index row.

## Stop and ask
- More than one option remains viable after the analysis: present them with a recommendation
  and let the maintainer choose before drafting the Decision.
- The decision conflicts with the constitution, an accepted ADR or an approved specification.
- Acceptance and supersession are the maintainer's decisions only.

## Report
- Done: record number and path, status, index and navigation updated, checks run and their
  result.
- Not done or not verified: site build not run, acceptance pending, specifications or tasks
  that must follow.
- Risks: records or specifications affected, migrations implied, conventions to update.
