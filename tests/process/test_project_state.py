"""scripts/project_state.py: the session briefing renders, with the shared sprint state (#333)."""

from __future__ import annotations

from collections.abc import Sequence
from typing import Any

import project_state as ps
import pytest
from conftest import ROOT

SCRIPT = ROOT / "scripts" / "project_state.py"


def briefing(sprint: dict[str, Any] | None) -> dict[str, Any]:
    return {
        "repository": "owner/wayloop",
        "branch": "fix/333-shared-sprint-state",
        "branch_issue": 333,
        "ahead_of_main": 1,
        "behind_main": 0,
        "uncommitted_files": [],
        "recent_commits": ["abc1234 fix(scripts): shared sprint state"],
        "journal": [],
        "pull_requests": [],
        "active_sprint": sprint,
        "sprint_done": [12],
        "current_milestone": "M0 · Foundations",
        "in_progress": [],
        "blocked": [],
        "ready": [],
    }


def test_briefing_lists_the_per_issue_state_of_the_active_sprint() -> None:
    sprint = {
        "file": "S01.yaml",
        "goal": "Shared state",
        "end": "2026-11-22",
        "issues": [12, 13],
        "state": ["#13, branch fix/13-x, step tests, 0 blockers"],
    }
    text = ps.render(briefing(sprint))
    assert "S01.yaml: Shared state (ends 2026-11-22); 1/2 issues closed." in text
    assert "Per-issue state (sprints state files, local recovery):" in text
    assert "- #13, branch fix/13-x, step tests, 0 blockers" in text


def test_briefing_without_state_files_has_no_state_heading() -> None:
    sprint = {"file": "S01.yaml", "goal": "g", "end": "2026-11-22", "issues": [12], "state": []}
    assert "Per-issue state" not in ps.render(briefing(sprint))


def test_briefing_without_an_active_sprint_names_the_milestone() -> None:
    assert "No active sprint. Current milestone: M0 · Foundations." in ps.render(briefing(None))


@pytest.mark.parametrize(
    ("paths", "tier"),
    [
        (["docs/en/index.md", "scripts/roadmap.py", ".github/workflows/ci.yml"], "agent"),
        (["crates/wayloop-core/src/lib.rs"], "maintainer confirmation"),
        (["README.md", "schemas/commands/next/v1.json"], "maintainer confirmation"),
        ([], "agent"),
    ],
)
def test_merge_tier_follows_the_changed_paths(paths: list[str], tier: str) -> None:
    assert ps.merge_tier(paths) == tier


def test_briefing_shows_who_merges_each_pull_request() -> None:
    state = briefing(None)
    state["pull_requests"] = [
        {
            "number": 7,
            "title": "feat(state): add guards",
            "branch": "feat/7-guards",
            "draft": False,
            "labels": [],
            "checks": "green",
            "reviews": [],
            "merge": "maintainer confirmation",
        }
    ]
    assert "| #7 | feat(state): add guards | green | none | maintainer confirmation |" in ps.render(
        state
    )


def _issue(number: int, labels: Sequence[str] = (), blocked_by: int = 0) -> dict[str, Any]:
    return {
        "number": number,
        "title": f"Issue {number}",
        "labels": [{"name": n} for n in labels],
        "issue_dependencies_summary": {"blocked_by": blocked_by, "blocking": 0},
    }


def test_ready_issues_leave_out_natively_blocked_issues() -> None:
    pool = [_issue(160, ["area:core"], blocked_by=1), _issue(3, ["area:core"])]
    assert [i["number"] for i in ps.ready_issues(pool, set())] == [3]


def test_ready_issues_leave_out_epics_taken_and_labelled_issues() -> None:
    pool = [
        _issue(1, ["epic"]),
        _issue(2, ["in-progress"]),
        _issue(3, ["blocked"]),
        _issue(4),
        _issue(5),
    ]
    ready = ps.ready_issues(pool, {"feat/5-something"})
    assert [i["number"] for i in ready] == [4]


def test_attention_lists_open_bug_and_maintenance_issues_bugs_first() -> None:
    def issue(number: int, *labels: str) -> dict[str, Any]:
        return {"number": number, "title": f"t{number}", "labels": [{"name": n} for n in labels]}

    pool = [
        issue(9, "maintenance"),
        issue(3, "task"),
        issue(7, "bug"),
        issue(5, "bug", "area:core"),
    ]
    assert ps.attention_issues(pool) == ["#5 t5 (bug)", "#7 t7 (bug)", "#9 t9 (maintenance)"]


def test_briefing_shows_bugs_and_maintenance_before_everything_else() -> None:
    data = {**briefing(None), "attention": ["#5 CI passes again on main (bug)"]}
    text = ps.render(data)
    assert text.index("## Bugs and maintenance first") < text.index("## Recent commits")
    assert "- #5 CI passes again on main (bug)" in text
    assert "## Bugs and maintenance first\n- none" in ps.render(briefing(None))


def _report(number: int, login: str, *labels: str) -> dict[str, Any]:
    return {
        "number": number,
        "title": f"Workflow {number} passes again on main",
        "user": {"login": login},
        "labels": [{"name": n} for n in labels],
    }


def test_red_main_lists_the_failure_issues_of_the_automation_only() -> None:
    pool = [
        _report(296, "github-actions[bot]", "bug"),
        _report(9, "github-actions[bot]", "maintenance"),
        _report(5, "someone", "bug"),
    ]
    assert ps.red_main(pool) == ["#296 Workflow 296 passes again on main"]


def test_briefing_starts_with_stop_while_main_is_red() -> None:
    data = {**briefing(None), "red_main": ["#296 Roadmap sync passes again on main"]}
    first = ps.render(data).splitlines()[0]
    assert first.startswith("stop: main is red")
    assert "#296 Roadmap sync passes again on main" in first
    assert "merge nothing but its fix" in first
    assert not ps.render(briefing(None)).startswith("stop:")
