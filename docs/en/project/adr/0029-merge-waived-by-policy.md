# ADR 0029: Merge of small changes waived by policy, recorded as a waiver

Status: Accepted · 2026-11-17 (decided by the maintainer in #233)

## Context

Constitution principle IV makes every approval state a gate that the loop cannot pass. This
repository already works otherwise for small changes: the agent that opens a pull request of
tier small (documentation, tooling, CI) merges it when its checks are green, and only engine,
adapter and contract changes wait for the maintainer. Without a decision, the engine has two
bad choices: stop the loop at every small change, or record a merge approval by a person who
did not act. The ledger is the audit record (ADR 0003), so it must say what actually happened.

## Options

1. **Every merge is a person's gate in version 1.** Honest, but it stops the loop at every
   change and differs from how this project is built.
2. **A standing approval recorded as the maintainer's approval at each merge.** Convenient, but
   it puts in the ledger an act that no person performed.
3. **A waiver by policy, recorded as a waiver.** The project policy names the tiers whose merge
   is not an approval state; the engine records that the gate was waived and by which policy.

## Decision

Option 3.

- The project policy may waive the merge gate for tier `small` only. Tiers for engine logic,
  adapters and contracts always keep the merge gate.
- The tier is computed by the engine from the paths changed on the branch, as observed in git,
  never from an agent's report.
- When the merge gate is waived, the engine records `gate_waived` with the policy revision and
  the person who approved that revision. It never records a waiver as a person's approval.
- A policy change that adds a waiver, widens tier `small` or relaxes a limit opens a gate of
  kind `configuration`, which only a recorded approver can resolve. A change that narrows
  applies at once.
- Constitution principle IV is amended to state this exception.

## Consequences

- The loop does not stop at small changes, and the audit record stays true.
- An agent cannot widen its own autonomy: every widening passes a person.
- The tier rules become part of the contract of the policy (SPEC-0021) and of the state machine
  (SPEC-0002).
