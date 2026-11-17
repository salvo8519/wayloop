# Behavioural case: spec-new

## Input
The maintainer asks for the engine to refuse a state transition when a required gate label is missing. The roadmap task exists without a specification; no existing specification covers gate enforcement for this capability.

## Expected
A folder `specs/NNNN-<slug>/` with the next free number and `spec.md` in status `Draft`: EARS requirements `R1` and following, each with an acceptance criterion that cites it and its verification technique, and at most five open questions ordered by impact, each with a recommended answer. The status stays `Draft` until the maintainer approves; `plan.md` and `tasks.md` are written only after approval. The roadmap task references the specification and `roadmap.py check` passes.
