"""GitHub access shared by the scripts (scripts/_github.py), without the network."""

from __future__ import annotations

import subprocess
import time
from collections.abc import Sequence
from typing import Any

import _github
import pytest


def test_pages_reads_every_page() -> None:
    pages = {1: list(range(100)), 2: list(range(100, 130))}
    calls: list[str] = []

    def fetch(path: str) -> list[int]:
        calls.append(path)
        return pages.get(int(path.rsplit("page=", 1)[1]), [])

    items = _github.pages("repos/o/r/issues?state=open", fetch)
    assert len(items) == 130
    assert calls == [
        "repos/o/r/issues?state=open&per_page=100&page=1",
        "repos/o/r/issues?state=open&per_page=100&page=2",
    ]
    assert _github.pages("repos/o/r/labels", lambda path: None) == []


@pytest.mark.parametrize(
    ("url", "repo"),
    [
        ("https://github.com/salvo8519/wayloop.git\n", "salvo8519/wayloop"),
        ("https://github.com/salvo8519/wayloop", "salvo8519/wayloop"),
        ("git@github.com:owner/name.git", "owner/name"),
        ("http://proxy@127.0.0.1:1/git/owner/name", None),
    ],
)
def test_repo_of_url(url: str, repo: str | None) -> None:
    assert _github.repo_of_url(url) == repo


def done(code: int, stdout: str = "", stderr: str = "") -> subprocess.CompletedProcess[str]:
    return subprocess.CompletedProcess([], code, stdout, stderr)


def test_repository_prefers_actions_then_origin_then_the_default(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setenv("GITHUB_REPOSITORY", "a/b")
    assert _github.repository() == "a/b"
    monkeypatch.delenv("GITHUB_REPOSITORY")
    monkeypatch.setattr(subprocess, "run", lambda *a, **k: done(0, "git@github.com:c/d.git\n"))
    assert _github.repository() == "c/d"
    monkeypatch.setattr(subprocess, "run", lambda *a, **k: done(2, "", "no remote"))
    assert _github.repository() == _github.DEFAULT_REPO


class FakeRun:
    def __init__(self, *results: subprocess.CompletedProcess[str]) -> None:
        self.results = list(results)
        self.calls: list[tuple[Sequence[str], Any]] = []

    def __call__(self, cmd: Sequence[str], **kwargs: Any) -> subprocess.CompletedProcess[str]:
        self.calls.append((cmd, kwargs.get("input")))
        return self.results.pop(0)


def test_api_sends_method_and_payload(monkeypatch: pytest.MonkeyPatch) -> None:
    run = FakeRun(done(0, '{"number": 7}'), done(0, ""))
    monkeypatch.setattr(subprocess, "run", run)
    assert _github.api("repos/o/r/issues", method="POST", payload={"title": "t"}) == {"number": 7}
    assert _github.api("repos/o/r/issues/7") is None
    assert run.calls == [
        (["gh", "api", "repos/o/r/issues", "-X", "POST", "--input", "-"], '{"title": "t"}'),
        (["gh", "api", "repos/o/r/issues/7"], None),
    ]


def test_a_failure_is_retried_then_stops_or_gives_nothing(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setattr(time, "sleep", lambda seconds: None)
    run = FakeRun(done(1, "", "busy"), done(0, "[1]"))
    monkeypatch.setattr(subprocess, "run", run)
    assert _github.api("repos/o/r/labels", retries=1) == [1]
    assert len(run.calls) == 2
    monkeypatch.setattr(subprocess, "run", FakeRun(done(1, "", "HTTP 404")))
    with pytest.raises(SystemExit, match="gh api repos/o/r/x failed: HTTP 404"):
        _github.api("repos/o/r/x")
    monkeypatch.setattr(subprocess, "run", FakeRun(done(1, "", "HTTP 404")))
    assert _github.api("repos/o/r/x", required=False) is None


def test_pages_stops_after_the_maximum_number_of_pages() -> None:
    def endless(path: str) -> list[int]:
        return list(range(_github.PER_PAGE))

    with pytest.raises(SystemExit, match="more than 100 pages"):
        _github.pages("repos/o/r/issues", endless)


def test_pages_refuses_an_answer_that_is_not_a_list() -> None:
    with pytest.raises(SystemExit, match="not a list"):
        _github.pages("repos/o/r/rulesets", lambda path: {"message": "Not Found"})
