# Session journal

Short, factual hand-over notes written at the end of every working session, so that the next
session (a person or an agent, on any machine) starts with full context instead of guessing.

The journal complements, and never replaces, the other sources of project memory:

| Source | Answers |
| --- | --- |
| Git history and `CHANGELOG.md` | What changed, technically |
| `changes/` fragments and `RELEASE_NOTES.md` | What changed, for users |
| Specs and ADRs | What was decided, and why |
| Issues and pull requests | What is planned, in progress, in review |
| This journal | Where a piece of work stopped, what is half done, what to do next |

## Rules

- One file per branch and day: `YYYY-MM-DD-<branch-slug>.md` (slashes in the branch name
  become dashes). Separate files avoid merge conflicts between parallel branches.
- Written by the `session-end` skill, read by the `session-start` skill, whatever agent runs
  them: an entry is complete when another agent of another vendor can continue from it alone.
- The Checkout section says who held the work, on which branch and worktree, which step was
  reached and whether the work is released. The `session-start` skill posts a checkout
  comment on the issue with a lease; the `session-end` skill posts the hand-over there.
- `python scripts/journal.py search <issue | area:name | date | text>` finds the entries.
- English, short sentences, no icons. Facts only: no client data, no secrets.
- Committed with the work it describes. Entries are never edited after merge; a later session
  writes a new entry.
- Entries older than two releases may be pruned by the `release` skill.

## Template

Copy `_template.md`.

When the engine runs this repository, checkout and hand-over become ledger events with a
lease (SPEC-0003 R10 to R13, #162); until then this journal and the issue comments are the
hand-over.
