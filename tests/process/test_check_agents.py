"""scripts/check_agents.py: the standard of ADR 0027 for skills, agents and hooks."""

from __future__ import annotations

import importlib.util
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location("check_agents", ROOT / "scripts/check_agents.py")
assert _spec and _spec.loader
ca = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(ca)

SKILL = """---
name: demo
description: Does the demo. Use when a demo is needed.
---

## Produces
A demo.

## Steps
1. Run `scripts/tool.py`.

## Stop and ask
- Never.

## Report
- Done, not done, risks.
"""

AGENT = """---
name: reviewer
description: Reviews a branch. Use when a pull request is ready.
tools: Read, Grep, Glob, Bash
---

## Reads
The diff.

## Does
Reviews it.

## Never
Edits files.

## Output
Findings.

## Hands off to
The implementer.
"""

CASE = "# Case\n\n## Input\nA diff.\n\n## Expected\nFindings.\n"


@pytest.fixture
def repo(tmp_path: Path) -> Path:
    (tmp_path / "scripts").mkdir()
    (tmp_path / "scripts" / "tool.py").write_text("", encoding="utf-8")
    (tmp_path / ".agents" / "skills" / "demo").mkdir(parents=True)
    (tmp_path / ".agents" / "skills" / "demo" / "SKILL.md").write_text(SKILL, encoding="utf-8")
    (tmp_path / ".agents" / "agents").mkdir()
    (tmp_path / ".agents" / "agents" / "reviewer.md").write_text(AGENT, encoding="utf-8")
    (tmp_path / ".agents" / "cases").mkdir()
    for name in ("demo", "reviewer"):
        (tmp_path / ".agents" / "cases" / f"{name}.md").write_text(CASE, encoding="utf-8")
    return tmp_path


def edit(path: Path, old: str, new: str) -> None:
    path.write_text(path.read_text(encoding="utf-8").replace(old, new), encoding="utf-8")


def test_the_committed_skills_and_agents_follow_the_standard() -> None:
    assert ca.problems_of() == []


def test_a_conforming_skill_and_agent_pass(repo: Path) -> None:
    assert ca.problems_of(repo) == []


@pytest.mark.parametrize(
    ("target", "old", "new", "message"),
    [
        ("skill", "name: demo", "name: other", "front matter `name` must be 'demo'"),
        ("skill", "Use when a demo", "For a demo", "trigger with 'Use when'"),
        ("skill", "## Stop and ask", "## Questions", "sections must be"),
        ("skill", "`scripts/tool.py`", "`scripts/missing.py`", "cites `scripts/missing.py`"),
        (
            "agent",
            "tools: Read, Grep, Glob, Bash",
            "tools: Read, Edit",
            "read-only role cannot have ['Edit']",
        ),
        ("agent", "tools: Read, Grep, Glob, Bash\n", "", "agents declare their `tools`"),
        ("agent", "## Never", "## Avoid", "sections must be"),
    ],
)
def test_each_rule_is_reported(repo: Path, target: str, old: str, new: str, message: str) -> None:
    path = (
        repo / ".agents" / "skills" / "demo" / "SKILL.md"
        if target == "skill"
        else repo / ".agents" / "agents" / "reviewer.md"
    )
    edit(path, old, new)
    assert any(message in p for p in ca.problems_of(repo))


def test_a_skill_over_budget_is_reported(repo: Path) -> None:
    edit(repo / ".agents" / "skills" / "demo" / "SKILL.md", "A demo.", "word " * 3000)
    assert any("the budget is 3000" in p for p in ca.problems_of(repo))


def test_placeholders_are_not_checked_as_paths(repo: Path) -> None:
    edit(
        repo / ".agents" / "skills" / "demo" / "SKILL.md",
        "A demo.",
        "See `specs/NNNN-<slug>/spec.md`.",
    )
    assert ca.problems_of(repo) == []


def test_a_missing_or_malformed_case_is_reported(repo: Path) -> None:
    (repo / ".agents" / "cases" / "demo.md").unlink()
    assert any("no behavioural case" in p for p in ca.problems_of(repo))
    (repo / ".agents" / "cases" / "demo.md").write_text("# Case\n\n## Input\nx\n", encoding="utf-8")
    assert any("sections must be ['Input', 'Expected']" in p for p in ca.problems_of(repo))


def test_an_unregistered_or_non_executable_hook_is_reported(repo: Path) -> None:
    hooks = repo / ".claude" / "hooks"
    hooks.mkdir(parents=True)
    (repo / ".claude" / "settings.json").write_text('{"hooks": {}}', encoding="utf-8")
    (hooks / "x.sh").write_text("#!/bin/sh\n", encoding="utf-8")
    problems = ca.problems_of(repo)
    assert ".claude/hooks/x.sh: not registered in .claude/settings.json" in problems
    assert ".claude/hooks/x.sh: not executable" in problems
