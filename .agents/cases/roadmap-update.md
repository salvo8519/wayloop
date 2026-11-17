# Behavioural case: roadmap-update

## Input
The maintainer asks for a command that prints the state of a sprint as JSON. No open task covers it; the epic "Process tooling" of milestone M0 exists.

## Expected
The request is classified as a new task in the existing epic. A proposal with milestone, epic, title in sentence case, labels, "no specification needed" (tooling) and dependencies is shown, and nothing is changed until the maintainer confirms. After confirmation the issue is created only with `uv run python scripts/roadmap.py new`, and `roadmap.py check` and `roadmap.py links` pass. The report names the issue number and the effect on the order of work.
