# Gates

A gate is a state that the loop cannot pass on its own. The loop runs unattended between
gates; at a gate it stops, asks, and waits for a person.

## Approval states

| Gate | Decided by |
| --- | --- |
| Council report | Maintainer |
| Roadmap and every proposed change to it | Maintainer |
| Specification `Approved` | Maintainer or the owner of the area |
| Merge | Maintainer, or automatically for the smallest risk tier when the maintainer allows it |

## Questions

When an agent needs a decision, the issue receives the label `needs-decision` and a question
in a fixed format:

1. **Context**, in three lines at most.
2. **Options**, each with its cost.
3. **Recommendation** of the agent, with its reason.

The answer is a comment or the choice of an option. The loop resumes from there. Questions do
not get lost in a closed chat session: they stay on the issue, next to the work they concern.

## When an agent does not stop

Between gates an agent decides and records a ruling in the ledger: what it decided, why, and
what it costs if the decision is wrong. It stops only for an irreversible or destructive
operation, a security-sensitive action, a side effect outside its worktree, a plan so broken
that every path is a guess, or a methodology choice that belongs to the maintainer.
