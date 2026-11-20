"""Names of the labels that the process scripts read or write, defined once.

Standard library only. Every name is planned in `roadmap/roadmap.yaml`, so that
`bootstrap_github.py` creates it; `tests/process/test_labels.py` checks it.
"""

from __future__ import annotations

from collections.abc import Mapping
from typing import Any

EPIC = "epic"
IN_PROGRESS = "in-progress"
BLOCKED = "blocked"
BUG = "bug"
MAINTENANCE = "maintenance"
PARALLEL = "parallel"
MAINTAINER = "maintainer"
NEEDS_DECISION = "needs-decision"
NO_ISSUE = "no-issue"
BREAKING_CHANGE = "breaking-change"
# The prefix of the one area label of an issue (`area:core`).
AREA_PREFIX = "area:"
# Issues the automation opens: a failing workflow, then the weekly maintenance run. The session
# briefing lists them in this order, before new work.
REPORT_LABELS = (BUG, MAINTENANCE)


def label_names(item: Mapping[str, Any]) -> set[str]:
    """Names of the labels of an issue or a pull request, as the REST API returns them."""
    return {label["name"] for label in item.get("labels") or []}
