"""Check skills, agents and hooks against the standard of ADR 0027.

Skills (`.agents/skills/<name>/SKILL.md`) and agents (`.agents/agents/<name>.md`):

- front matter with `name` equal to the folder or file name, and a `description` whose trigger
  starts with "Use when";
- the sections of the standard, in order: skills `Produces`, `Steps`, `Stop and ask`,
  `Report`; agents `Reads`, `Does`, `Never`, `Output`, `Hands off to`;
- agents declare `tools`; read-only roles have no tool that writes files;
- a size budget, estimated at four characters per token;
- every script and repository path cited in backticks exists;
- a behavioural case in `.agents/cases/<name>.md` with `Input` and `Expected` sections.

Hooks (`.claude/hooks/*.sh`): registered in `.claude/settings.json` and executable.

    python scripts/check_agents.py
"""

from __future__ import annotations

import json
import os
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SKILL_SECTIONS = ["Produces", "Steps", "Stop and ask", "Report"]
AGENT_SECTIONS = ["Reads", "Does", "Never", "Output", "Hands off to"]
CASE_SECTIONS = ["Input", "Expected"]
SKILL_BUDGET = 3000  # tokens
AGENT_BUDGET = 1500
READ_ONLY = {
    "reviewer",
    "critic",
    "explorer",
    "security-reviewer",
    "contract-auditor",
    "simplifier",
    "ci-investigator",
}
WRITE_TOOLS = {"Edit", "Write", "MultiEdit", "NotebookEdit"}
CITED = re.compile(
    r"`((?:scripts|\.agents|\.claude|\.github|docs|specs|roadmap|sprints|changes)/[^`\s]+)`"
)
PLACEHOLDER = re.compile(r"[<>*{}]|NNNN|SNN|YYYY")


def front_matter(text: str) -> tuple[dict[str, str], str]:
    m = re.match(r"^---\n(.*?)\n---\n", text, re.DOTALL)
    if not m:
        return {}, text
    fields = {}
    for line in m.group(1).splitlines():
        key, _, value = line.partition(":")
        if value:
            fields[key.strip()] = value.strip()
    return fields, text[m.end() :]


def sections(body: str) -> list[str]:
    return re.findall(r"^## (.+?)\s*$", body, re.MULTILINE)


def check_document(path: Path, kind: str, root: Path = ROOT) -> list[str]:
    """Problems of one skill or agent file."""
    rel = path.relative_to(root).as_posix()
    text = path.read_text(encoding="utf-8")
    fields, body = front_matter(text)
    name = path.parent.name if kind == "skill" else path.stem
    problems = []
    if fields.get("name") != name:
        problems.append(f"{rel}: front matter `name` must be '{name}'")
    if "Use when" not in fields.get("description", ""):
        problems.append(f"{rel}: the description must state its trigger with 'Use when'")
    expected = SKILL_SECTIONS if kind == "skill" else AGENT_SECTIONS
    found = [s for s in sections(body) if s in expected]
    if found != expected:
        problems.append(f"{rel}: sections must be {expected} in this order, found {found}")
    if kind == "agent":
        tools = {t.strip() for t in fields.get("tools", "").split(",") if t.strip()}
        if not tools:
            problems.append(f"{rel}: agents declare their `tools`")
        if name in READ_ONLY and tools & WRITE_TOOLS:
            problems.append(f"{rel}: a read-only role cannot have {sorted(tools & WRITE_TOOLS)}")
    budget = SKILL_BUDGET if kind == "skill" else AGENT_BUDGET
    if len(text) / 4 > budget:
        problems.append(f"{rel}: about {len(text) // 4} tokens, the budget is {budget}")
    for cited in CITED.findall(text):
        target = cited.split()[0].rstrip(".,:;)")
        if not PLACEHOLDER.search(target) and not (root / target).exists():
            problems.append(f"{rel}: cites `{target}`, which does not exist")
    case = root / ".agents" / "cases" / f"{name}.md"
    if not case.exists():
        problems.append(f"{rel}: no behavioural case in .agents/cases/{name}.md")
    elif [
        s for s in sections(case.read_text(encoding="utf-8")) if s in CASE_SECTIONS
    ] != CASE_SECTIONS:
        problems.append(f".agents/cases/{name}.md: sections must be {CASE_SECTIONS}")
    return problems


def check_hooks(root: Path = ROOT) -> list[str]:
    settings = root / ".claude" / "settings.json"
    if not settings.exists():
        return []
    registered = json.dumps(json.loads(settings.read_text(encoding="utf-8")).get("hooks", {}))
    problems = []
    for hook in sorted((root / ".claude" / "hooks").glob("*.sh")):
        if f"/.claude/hooks/{hook.name}" not in registered:
            problems.append(f".claude/hooks/{hook.name}: not registered in .claude/settings.json")
        if not os.access(hook, os.X_OK):
            problems.append(f".claude/hooks/{hook.name}: not executable")
    return problems


def problems_of(root: Path = ROOT) -> list[str]:
    problems = []
    for skill in sorted((root / ".agents" / "skills").glob("*/SKILL.md")):
        problems += check_document(skill, "skill", root)
    for agent in sorted((root / ".agents" / "agents").glob("*.md")):
        problems += check_document(agent, "agent", root)
    return problems + check_hooks(root)


def main() -> int:
    problems = problems_of()
    if problems:
        print("Skills, agents and hooks do not follow the standard (ADR 0027):")
        print("\n".join(f"  {p}" for p in problems))
        return 1
    skills = len(list((ROOT / ".agents" / "skills").glob("*/SKILL.md")))
    agents = len(list((ROOT / ".agents" / "agents").glob("*.md")))
    print(f"{skills} skills and {agents} agents follow the standard; hooks registered.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
