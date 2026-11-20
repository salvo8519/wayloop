"""Claude Code hooks in .claude/: registered, executable, and correct on real input."""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
from collections.abc import Mapping
from pathlib import Path

import conventions
import pytest
from conftest import ROOT, copy_hooks, git

HOOKS = ROOT / ".claude" / "hooks"


def registered() -> list[str]:
    settings = json.loads((ROOT / ".claude" / "settings.json").read_text(encoding="utf-8"))
    return [
        hook["command"].split("/.claude/hooks/")[1]
        for groups in settings["hooks"].values()
        for group in groups
        for hook in group["hooks"]
    ]


def run(hook: str, payload: dict[str, object], cwd: Path) -> subprocess.CompletedProcess[str]:
    return run_env(hook, payload, cwd)


def test_every_hook_is_registered_and_executable() -> None:
    files = sorted(p.name for p in HOOKS.glob("*.sh"))
    assert sorted(set(registered())) == files
    assert all(os.access(HOOKS / f, os.X_OK) for f in files)


@pytest.fixture
def project(git_repo: Path) -> Path:
    """A git repository with the hooks and the scripts they import or run."""
    copy_hooks(git_repo)
    git(git_repo, "add", ".")
    git(git_repo, "commit", "-q", "-m", "base")
    return git_repo


def test_quality_gate_runs_when_the_session_and_a_subagent_stop() -> None:
    settings = json.loads((ROOT / ".claude" / "settings.json").read_text(encoding="utf-8"))
    for event in ("Stop", "SubagentStop"):
        commands = [h["command"] for g in settings["hooks"][event] for h in g["hooks"]]
        assert any(c.endswith("/.claude/hooks/quality-gate.sh") for c in commands), event


def test_stop_hook_sends_missing_documentation_back(project: Path) -> None:
    (project / "schemas").mkdir()
    (project / "schemas" / "x.json").write_text("{}\n", encoding="utf-8")
    result = run("docs-impact.sh", {"stop_hook_active": False}, project)
    assert result.returncode == 2
    assert "schemas changed" in result.stderr


def test_stop_hook_passes_with_documentation_and_never_loops(project: Path) -> None:
    (project / "schemas").mkdir()
    (project / "schemas" / "x.json").write_text("{}\n", encoding="utf-8")
    assert run("docs-impact.sh", {"stop_hook_active": True}, project).returncode == 0
    (project / "docs" / "en" / "reference").mkdir(parents=True)
    (project / "docs" / "en" / "reference" / "x.md").write_text("# X\n", encoding="utf-8")
    assert run("docs-impact.sh", {"stop_hook_active": False}, project).returncode == 0


@pytest.mark.skipif(shutil.which("uv") is None, reason="uv is needed to run the formatter")
def test_format_hook_formats_python_and_ignores_other_files(project: Path) -> None:
    source = project / "x.py"
    source.write_text("x = {  'a':1 }\n", encoding="utf-8")
    note = project / "note.md"
    note.write_text("text  \n", encoding="utf-8")
    for path in (source, note, project / "missing.py"):
        assert run("format.sh", {"tool_input": {"file_path": str(path)}}, ROOT).returncode == 0
    assert source.read_text(encoding="utf-8") == 'x = {"a": 1}\n'
    assert note.read_text(encoding="utf-8") == "text  \n"


def test_format_hook_ignores_input_without_a_file(project: Path) -> None:
    assert run("format.sh", {"tool_input": {}}, project).returncode == 0
    assert run("format.sh", {}, project).returncode == 0


def run_env(
    hook: str, payload: Mapping[str, object], cwd: Path, **env: str
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [str(cwd / ".claude" / "hooks" / hook)],
        input=json.dumps(payload),
        capture_output=True,
        text=True,
        env={**os.environ, "CLAUDE_PROJECT_DIR": str(cwd), **env},
        cwd=cwd,
        check=False,
    )


@pytest.fixture
def contracts(project: Path) -> Path:
    for path in ("schemas/commands/next/v1.json", "schemas/handover/v1.input.json"):
        snapshot = project / path
        snapshot.parent.mkdir(parents=True)
        snapshot.write_text("{}\n", encoding="utf-8")
    git(project, "add", ".")
    git(project, "commit", "-q", "-m", "snapshot")
    git(project, "checkout", "-q", "-b", "feat/7-next")
    return project


@pytest.mark.parametrize(
    "path", ["schemas/commands/next/v1.json", "schemas/handover/v1.input.json"]
)
def test_an_existing_contract_snapshot_cannot_be_edited(contracts: Path, path: str) -> None:
    existing = contracts / path
    result = run_env(
        "protect-contracts.sh",
        {"tool_input": {"file_path": str(existing)}},
        contracts,
        WAYLOOP_CONTRACT_BASE="main",
    )
    assert result.returncode == 2
    assert "write the next version" in result.stderr


def test_a_new_snapshot_and_other_files_stay_editable(contracts: Path) -> None:
    for path in (
        "schemas/commands/next/v2.json",
        "schemas/handover/v2.input.json",
        "schemas/index.json",
        "docs/en/x.md",
    ):
        payload = {"tool_input": {"file_path": str(contracts / path)}}
        assert (
            run_env(
                "protect-contracts.sh", payload, contracts, WAYLOOP_CONTRACT_BASE="main"
            ).returncode
            == 0
        )


def test_briefing_names_branch_issue_and_next_step(contracts: Path) -> None:
    journal = contracts / ".agents" / "journal"
    journal.mkdir(parents=True)
    (journal / "2026-11-17-feat-7-next.md").write_text(
        "# 2026-11-17 feat/7-next\n\n## Next steps\n1. Write the replay test.\n", encoding="utf-8"
    )
    out = run_env("session-briefing.sh", {}, contracts).stdout
    assert "Branch: feat/7-next." in out and "Issue: #7." in out
    assert "Write the replay test." in out


def test_a_session_that_committed_without_hand_over_is_reminded_once(contracts: Path) -> None:
    (contracts / "x.txt").write_text("x\n", encoding="utf-8")
    git(contracts, "add", ".")
    git(contracts, "commit", "-q", "-m", "work")
    result = run_env(
        "handover-reminder.sh", {"stop_hook_active": False}, contracts, WAYLOOP_CONTRACT_BASE="main"
    )
    assert result.returncode == 2 and "without a hand-over" in result.stderr
    assert (
        run_env(
            "handover-reminder.sh",
            {"stop_hook_active": True},
            contracts,
            WAYLOOP_CONTRACT_BASE="main",
        ).returncode
        == 0
    )
    today = subprocess.run(
        ["date", "+%Y-%m-%d"], capture_output=True, text=True, check=True
    ).stdout.strip()
    journal = contracts / ".agents" / "journal"
    journal.mkdir(parents=True, exist_ok=True)
    (journal / f"{today}-feat-7-next.md").write_text("# hand-over\n", encoding="utf-8")
    assert (
        run_env(
            "handover-reminder.sh",
            {"stop_hook_active": False},
            contracts,
            WAYLOOP_CONTRACT_BASE="main",
        ).returncode
        == 0
    )


def test_no_reminder_on_main_or_without_commits(contracts: Path) -> None:
    assert (
        run_env("handover-reminder.sh", {}, contracts, WAYLOOP_CONTRACT_BASE="main").returncode == 0
    )
    git(contracts, "checkout", "-q", "main")
    assert (
        run_env("handover-reminder.sh", {}, contracts, WAYLOOP_CONTRACT_BASE="main").returncode == 0
    )


def briefing_branch_issue(branch: str) -> str:
    """The issue that the sed program of session-briefing.sh reads from a branch name."""
    text = (HOOKS / "session-briefing.sh").read_text(encoding="utf-8")
    programs = re.findall(r"sed -nE '([^']+)'", text)
    assert len(programs) == 1, "session-briefing.sh reads the issue with one sed -nE program"
    return subprocess.run(
        ["sed", "-nE", programs[0]],
        input=branch + "\n",
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()


@pytest.mark.parametrize(
    "branch",
    [
        *(f"{t}/42-ledger-hash" for t in conventions.TYPES),
        "sprint/S01/42-ledger-hash",
        "sprint/S12/7-x",
        "wip/42-x",
        "feat/42-Upper",
        "feat/ledger",
        "sprint/01/42-x",
        "main",
        "dependabot/cargo/serde-1.0.200",
        "release/v0.1.0",
        "revert-57-feat/42-x",
    ],
)
def test_briefing_reads_the_branch_issue_as_conventions_does(branch: str) -> None:
    # The hook runs without the project environment, so it keeps its own copy of the rule;
    # this test keeps the copy equal to scripts/conventions.py.
    issue = conventions.branch_issue(branch)
    assert briefing_branch_issue(branch) == ("" if issue is None else str(issue))
