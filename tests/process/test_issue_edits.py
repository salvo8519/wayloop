"""Direct edits of issue titles and bodies are refused (scripts/_issue_edits.py, #312)."""

from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path

import _issue_edits as ie
import protect_issues as hook
import pytest
from conftest import ROOT, copy_hooks

HOOK = "protect-issues.sh"

REFUSED = [
    # The exact commands of 2026-11-20 that dropped the parent line of #293-#295.
    "gh api repos/salvo8519/wayloop/issues/294 -X PATCH -F body=@/tmp/b294",
    "gh api repos/salvo8519/wayloop/issues/293 -X PATCH -F body=@/tmp/b293 "
    "&& gh api repos/salvo8519/wayloop/issues/295 -X PATCH -F body=@/tmp/b295",
    "gh api repos/o/r/issues/7 --method PATCH -f title='New title'",
    "gh api /repos/o/r/issues/7 -XPATCH --raw-field=body=text",
    "gh api repos/{owner}/{repo}/issues/7 -f body=x",  # a field implies POST
    "gh api repos/o/r/issues/7 -X POST -F title=x",
    "gh api repos/o/r/issues/7 -X patch --input /tmp/payload.json",
    "gh api https://api.github.com/repos/o/r/issues/7 -X PATCH -F body=@b",
    "gh issue edit 7 --body-file /tmp/b",
    "gh issue edit 7 --title 'Other'",
    "gh issue edit 7 -R o/r -b text",
    "gh issue edit 7 --body=text",
    "cd /tmp && GH_TOKEN=x gh issue edit 7 -F b.md",
    "echo start\ngh api repos/o/r/issues/7 -X PATCH -F body=@b",
    "bash -c 'gh issue edit 7 --title T'",
    'sh -lc "gh api repos/o/r/issues/7 -X PATCH -f body=x"',
    "eval gh issue edit 7 --title T",
    "/usr/bin/gh issue edit 7 --title T",
    'gh api graphql -f query=\'mutation { updateIssue(input: {id: "I", body: "x"}) { id } }\'',
]

ALLOWED = [
    "gh api repos/o/r/issues/7",
    "gh api repos/o/r/issues/7 --jq .body",
    "gh api repos/o/r/issues/7/comments -f body='A comment'",
    "gh api repos/o/r/issues/7/comments -X POST -F body=@c.md",
    "gh api repos/o/r/issues/7/labels -X POST -f 'labels[]=bug'",
    "gh api repos/o/r/issues/7 -X PATCH -f state=closed",
    "gh api repos/o/r/issues -X POST -f title=x",  # creation is not an edit of an issue
    "gh api repos/o/r/pulls/7 -X PATCH -f body=x",
    "gh issue edit 7 --add-label bug",
    "gh issue comment 7 --body 'Done'",
    "gh issue close 7 --comment 'Fixed'",
    "gh issue view 7",
    "gh pr create --title 'ci(scripts): x' --body-file b.md",
    "gh pr edit 7 --body-file b.md",
    "uv run python scripts/roadmap.py edit 294 --body /tmp/b294",
    "git commit -s -m 'ci(scripts): refuse gh issue edit --title in a hook'",
    "gh api graphql -f query='{ viewer { login } }'",
    "ls -la",
]


@pytest.mark.parametrize("command", REFUSED)
def test_direct_edits_of_titles_and_bodies_are_refused(command: str) -> None:
    assert ie.reason_to_block(command) is not None
    assert ie.LOOKS_LIKE_EDIT.search(command), "the fail-closed pattern must cover it too"


@pytest.mark.parametrize("command", ALLOWED)
def test_reads_comments_labels_and_other_commands_are_allowed(command: str) -> None:
    assert ie.reason_to_block(command) is None


def test_the_message_names_the_roadmap_command() -> None:
    reason = ie.reason_to_block(REFUSED[0]) or ""
    assert "PATCH" in reason and "body" in reason
    assert "uv run python scripts/roadmap.py edit" in ie.message(reason)


def test_the_fallback_pattern_of_the_hook_equals_its_source() -> None:
    # The hook keeps a copy to fail closed when scripts/_issue_edits.py cannot load.
    assert hook.FALLBACK_LOOKS_LIKE_EDIT.pattern == ie.LOOKS_LIKE_EDIT.pattern


def test_the_hook_is_registered_on_bash() -> None:
    settings = json.loads((ROOT / ".claude" / "settings.json").read_text(encoding="utf-8"))
    groups = [g for g in settings["hooks"]["PreToolUse"] if g["matcher"] == "Bash"]
    assert [h["command"] for g in groups for h in g["hooks"]] == [
        f'"$CLAUDE_PROJECT_DIR"/.claude/hooks/{HOOK}'
    ]


def run_hook(project: Path, command: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [str(project / ".claude" / "hooks" / HOOK)],
        input=json.dumps({"tool_name": "Bash", "tool_input": {"command": command}}),
        capture_output=True,
        text=True,
        env={**os.environ, "CLAUDE_PROJECT_DIR": str(project)},
        cwd=project,
        check=False,
    )


@pytest.fixture
def project(tmp_path: Path) -> Path:
    copy_hooks(tmp_path)
    return tmp_path


def test_the_hook_blocks_the_command_of_2026_11_20(project: Path) -> None:
    result = run_hook(project, REFUSED[0])
    assert result.returncode == 2
    assert "scripts/roadmap.py edit" in result.stderr
    assert run_hook(project, ALLOWED[2]).returncode == 0


def test_the_hook_fails_closed_when_its_module_cannot_load(project: Path) -> None:
    (project / "scripts" / "_issue_edits.py").write_text("def broken(:\n", encoding="utf-8")
    result = run_hook(project, REFUSED[0])
    assert result.returncode == 2
    assert "could not run" in result.stderr
    assert run_hook(project, "ls -la").returncode == 0


def test_an_unreadable_command_that_looks_like_an_edit_is_blocked(project: Path) -> None:
    result = run_hook(project, "gh issue edit 7 --title 'unbalanced")
    assert result.returncode == 2
    assert run_hook(project, "echo 'unbalanced").returncode == 0


def test_the_hook_ignores_input_without_a_command(project: Path) -> None:
    payloads: list[dict[str, object]] = [{}, {"tool_input": {}}]
    for payload in payloads:
        result = subprocess.run(
            [str(project / ".claude" / "hooks" / HOOK)],
            input=json.dumps(payload),
            capture_output=True,
            text=True,
            cwd=project,
            check=False,
        )
        assert result.returncode == 0
