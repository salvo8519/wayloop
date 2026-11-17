# CLAUDE.md

@AGENTS.md

## Claude Code specifics

- Never use icons, emoji or decorative symbols, in files or in replies about this project.
- Talk to each person in the language they prefer: read `CLAUDE.local.md` if it exists. Write
  everything that goes into the repository in English.
- Roles live in `.agents/agents/`. Delegate instead of doing everything in the main context;
  subagents cannot start other subagents, so the main session is the orchestrator.
- Plan before multi-file changes; keep diffs small and focused on one issue.
- When a design choice is ambiguous, ask the maintainer instead of picking one.
- The main session merges the pull requests it opens, following the Merging section of
  AGENTS.md: small ones when their gate holds; engine, adapter and contract changes only after
  the maintainer confirms in the conversation.
