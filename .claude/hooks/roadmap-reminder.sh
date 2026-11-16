#!/usr/bin/env bash
# UserPromptSubmit hook: every request is read with the roadmap in mind. The agent decides
# whether the request changes the plan; the reminder makes sure the question is asked.
cat <<'TEXT'
Roadmap check: if this request adds, changes, moves or drops planned work, propose the
roadmap change (roadmap/roadmap.yaml) to the maintainer before doing the work, or say in one
line that it has no roadmap impact. Never create issues or change milestones outside
scripts/roadmap.py.
TEXT
