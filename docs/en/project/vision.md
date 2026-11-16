# Vision

Wayloop exists to make agentic software development dependable at the scale of a whole
project, not only of a single task.

## The problem

Coding agents produce code quickly. Projects still fail in the work around the code: intent
that was never written down, plans that drift from what is built, sessions that the next agent
cannot resume, and costs that nobody measures. Spec-driven tools capture intent but stop at the
plan file; agentic loops keep an agent working but do not know the plan.

## The thesis

1. **The engine decides, agents write.** A deterministic state machine runs the loop and
   verifies every step with checks, not with the word of a model.
2. **The tracker is the database.** The plan lives where the team already works. Remove
   Wayloop and the project still reads.
3. **Humans at the gates.** The loop runs unattended between approval states and stops at them.
4. **Cost is measured.** Every phase has a token budget, and the ledger records the cost of
   every issue.

## Who it is for

- **Analysts and designers**, who write intent and want to see it carried through to the code.
- **Maintainers**, who want agents to work while they are away and to find questions, not
  surprises, when they come back.
- **Developers**, who want a process that agents follow without being reminded.

## Three tests of success

1. **Removal.** Without Wayloop, the project is still understandable from the tracker and the
   repository.
2. **Hand-over.** Any agent resumes the work from an issue and its specification alone.
3. **Cost.** The cost per issue is measured and comparable with other approaches on the same
   work.

## Out of scope

- Writing a coding agent: Wayloop drives existing agents.
- Semantic search with embeddings: see ADR 0004.
- Hosting: Wayloop is a local-first tool and a set of workflows; there is no server to run.
