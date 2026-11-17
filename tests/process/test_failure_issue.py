"""scripts/failure_issue.py: one issue per failing workflow, updated and closed (#240)."""

from __future__ import annotations

import importlib.util
import json
import sys
from pathlib import Path
from typing import Any

import pytest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location(
    "failure_issue", ROOT / "scripts" / "failure_issue.py"
)
assert spec is not None and spec.loader is not None
failure_issue = importlib.util.module_from_spec(spec)
sys.modules["failure_issue"] = failure_issue
spec.loader.exec_module(failure_issue)

RUN = "https://github.com/o/r/actions/runs/"


class FakeGh:
    """The subset of the GitHub CLI that the script calls, on an in-memory issue list."""

    def __init__(self, failed_jobs: list[str] | None = None) -> None:
        self.issues: dict[int, dict[str, Any]] = {}
        self.comments: dict[int, list[str]] = {}
        self.failed_jobs = failed_jobs or []
        self.calls: list[list[str]] = []

    @staticmethod
    def flag(args: list[str], name: str) -> str:
        return args[args.index(name) + 1]

    def __call__(self, args: list[str]) -> str:
        self.calls.append(args)
        match args[:2]:
            case ["issue", "list"]:
                label = self.flag(args, "--label")
                return json.dumps(
                    [
                        {"number": n, "title": i["title"], "body": i["body"]}
                        for n, i in self.issues.items()
                        if i["state"] == "open" and label in i["labels"]
                    ]
                )
            case ["issue", "create"]:
                number = len(self.issues) + 1
                self.issues[number] = {
                    "title": self.flag(args, "--title"),
                    "body": self.flag(args, "--body"),
                    "labels": [self.flag(args, "--label")],
                    "state": "open",
                }
                return f"https://github.com/o/r/issues/{number}"
            case ["issue", "edit"]:
                self.issues[int(args[2])]["body"] = self.flag(args, "--body")
                return ""
            case ["issue", "comment"]:
                self.comments.setdefault(int(args[2]), []).append(self.flag(args, "--body"))
                return ""
            case ["issue", "close"]:
                self.issues[int(args[2])]["state"] = "closed"
                self.comments.setdefault(int(args[2]), []).append(self.flag(args, "--comment"))
                return ""
            case ["api", path] if path.endswith("/jobs?per_page=100"):
                return "\n".join(self.failed_jobs)
        pytest.fail(f"unexpected gh call: {args}")


def report(gh: FakeGh, conclusion: str, run_id: int, **kw: Any) -> str:
    options: dict[str, Any] = {
        "repo": "o/r",
        "workflow": "CI",
        "run_id": run_id,
        "run_url": f"{RUN}{run_id}",
        "conclusion": conclusion,
        "event": "push",
        "branch": "main",
        "label": "bug",
        "title": None,
        "jobs": None,
        "details": "",
    }
    options.update(kw)
    return str(failure_issue.report(gh, **options))


def open_issues(gh: FakeGh) -> list[dict[str, Any]]:
    return [i for i in gh.issues.values() if i["state"] == "open"]


def test_a_failure_opens_one_issue_with_the_jobs_and_the_log() -> None:
    gh = FakeGh(failed_jobs=["House style, translations and Rust"])
    assert report(gh, "failure", 11) == "opened #1"
    (issue,) = open_issues(gh)
    assert issue["labels"] == ["bug"]
    assert issue["title"] == "CI passes again on main"
    assert "House style, translations and Rust" in issue["body"]
    assert f"{RUN}11" in issue["body"]


def test_a_second_failure_updates_the_same_issue() -> None:
    gh = FakeGh(failed_jobs=["Build"])
    report(gh, "failure", 11)
    gh.failed_jobs = ["Test"]
    assert report(gh, "timed_out", 12) == "updated #1"
    (issue,) = open_issues(gh)
    assert "Test" in issue["body"] and f"{RUN}12" in issue["body"]
    assert f"{RUN}12" in gh.comments[1][0]


def test_a_success_closes_the_issue_and_a_later_failure_opens_a_new_one() -> None:
    gh = FakeGh(failed_jobs=["Build"])
    report(gh, "failure", 11)
    assert report(gh, "success", 13) == "closed #1"
    assert open_issues(gh) == []
    assert f"{RUN}13" in gh.comments[1][-1]
    assert report(gh, "failure", 14) == "opened #2"


def test_a_success_without_an_open_issue_changes_nothing() -> None:
    gh = FakeGh()
    assert report(gh, "success", 13) == "nothing to close"
    assert all(call[:2] == ["issue", "list"] for call in gh.calls)


def test_each_workflow_has_its_own_issue() -> None:
    gh = FakeGh(failed_jobs=["Build"])
    report(gh, "failure", 11)
    assert report(gh, "failure", 12, workflow="Security", event="schedule") == "opened #2"
    assert report(gh, "success", 13) == "closed #1"
    assert [i["title"] for i in open_issues(gh)] == ["Security passes again on main"]


@pytest.mark.parametrize(
    ("conclusion", "event", "branch"),
    [
        ("cancelled", "push", "main"),
        ("skipped", "schedule", "main"),
        ("failure", "pull_request", "main"),
        ("failure", "push", "feature/1-x"),
    ],
)
def test_runs_that_are_not_failures_on_main_are_ignored(
    conclusion: str, event: str, branch: str
) -> None:
    gh = FakeGh(failed_jobs=["Build"])
    assert report(gh, conclusion, 11, event=event, branch=branch).startswith("ignored")
    assert gh.calls == []


def test_explicit_jobs_and_details_and_a_legacy_issue_found_by_title() -> None:
    gh = FakeGh()
    gh.issues[1] = {
        "title": "Weekly maintenance report",
        "body": "older report",
        "labels": ["maintenance"],
        "state": "open",
    }
    result = report(
        gh,
        "failure",
        21,
        workflow="Weekly maintenance",
        event="schedule",
        label="maintenance",
        title="Weekly maintenance report",
        jobs=["mutants=failure"],
        details="Surviving mutants:\n- lib.rs:3: replace f with true",
    )
    assert result == "updated #1"
    body = gh.issues[1]["body"]
    assert "mutants=failure" in body and "replace f with true" in body
    assert not any(call[0] == "api" for call in gh.calls)


def test_long_details_are_cut_to_the_size_of_an_issue_body() -> None:
    gh = FakeGh(failed_jobs=["Build"])
    report(gh, "failure", 11, details="x" * 100_000)
    assert len(open_issues(gh)[0]["body"]) <= failure_issue.MAX_BODY


def test_main_reads_the_details_file(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    details = tmp_path / "missed.md"
    details.write_text("Surviving mutants: 2", encoding="utf-8")
    gh = FakeGh(failed_jobs=["Build"])
    monkeypatch.setattr(failure_issue, "gh_cli", gh)
    argv = [
        "report",
        "--repo=o/r",
        "--workflow=CI",
        "--run-id=5",
        f"--run-url={RUN}5",
        "--conclusion=failure",
        "--event=push",
        "--branch=main",
        f"--details={details}",
    ]
    assert failure_issue.main(argv) == 0
    assert "Surviving mutants: 2" in open_issues(gh)[0]["body"]
