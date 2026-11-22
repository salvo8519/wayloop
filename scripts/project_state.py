"""Where the project stands, for the start of a working session (human or agent).

Reads git and GitHub (REST, through the GitHub CLI) and prints a Markdown briefing:
current branch and its issue, uncommitted work, recent session journal, open pull requests
with CI and review state, the active sprint, issues in progress and issues ready to start. Open
issues labelled `bug` or `maintenance` (failures on main, weekly maintenance) come first; while
a failure issue of `main` is open, the first line says to stop and merge nothing but its fix.

    uv run python scripts/project_state.py            # briefing in Markdown
    uv run python scripts/project_state.py --json     # the same data, for agents
"""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any

import _git
import _github
import yaml
from _labels import (
    AREA_PREFIX,
    BLOCKED,
    EPIC,
    IN_PROGRESS,
    PARALLEL,
    REPORT_LABELS,
    label_names,
)
from conventions import branch_issue
from failure_issue import reports_red_main
from sprint import state_files

ROOT = Path(__file__).resolve().parents[1]


# Pull requests that change the engine, an adapter or a contract wait for the maintainer;
# documentation, tooling and CI are merged by the agent (AGENTS.md, Merging).
CONFIRMATION_PATHS = re.compile(r"^(crates/|schemas/|python/|typescript/|packages/)")


def merge_tier(paths: list[str]) -> str:
    """Who merges a pull request that changes these paths."""
    if any(CONFIRMATION_PATHS.match(p) for p in paths):
        return "maintainer confirmation"
    return "agent"


def _state_line(path: Path) -> str:
    """One line of per-issue sprint state for the briefing (scripts/sprint.py state files)."""
    data = yaml.safe_load(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        return ""
    bits = [f"#{data.get('issue', path.stem)}"]
    if data.get("branch"):
        bits.append(f"branch {data['branch']}")
    if data.get("step"):
        bits.append(f"step {data['step']}")
    if data.get("pull_request"):
        bits.append(f"PR #{data['pull_request']}")
    blockers = data.get("blockers") or []
    bits.append(f"{len(blockers)} blocker" if len(blockers) == 1 else f"{len(blockers)} blockers")
    if data.get("agent_steps"):
        bits.append(f"{data['agent_steps']} agent steps")
    if data.get("updated"):
        bits.append(f"updated {str(data['updated'])[:16]}")
    return ", ".join(bits)


JOURNAL = ROOT / ".agents" / "journal"


def gh(path: str) -> Any:
    """One GitHub call; the briefing goes on without GitHub, so a failure gives None."""
    return _github.api(path, required=False)


def blocked_natively(issue: dict[str, Any]) -> bool:
    """True when the issue has an open native blocked-by dependency."""
    return bool((issue.get("issue_dependencies_summary") or {}).get("blocked_by"))


def ready_issues(pool: list[dict[str, Any]], busy_branches: set[str]) -> list[dict[str, Any]]:
    """Issues of the pool that can start now: not an epic, not taken, not blocked."""
    return [
        {
            "number": i["number"],
            "title": i["title"],
            "parallel": PARALLEL in label_names(i),
            "areas": sorted(lbl for lbl in label_names(i) if lbl.startswith(AREA_PREFIX)),
        }
        for i in pool
        if not label_names(i) & {EPIC, IN_PROGRESS, BLOCKED}
        and not blocked_natively(i)
        and not any(branch_issue(b) == i["number"] for b in busy_branches)
    ][:12]


def attention_issues(issues: list[dict[str, Any]]) -> list[str]:
    """Open bug issues, then maintenance issues, each by number: they go before new work."""
    lines: list[str] = []
    seen: set[int] = set()
    for label in REPORT_LABELS:
        for i in sorted(issues, key=lambda i: int(i["number"])):
            if i["number"] not in seen and label in label_names(i):
                seen.add(i["number"])
                lines.append(f"#{i['number']} {i['title']} ({label})")
    return lines


def red_main(issues: list[dict[str, Any]]) -> list[str]:
    """Open failure issues of `main`, by number: while one is open, main is red."""
    return [
        f"#{i['number']} {i['title']}"
        for i in sorted(issues, key=lambda i: int(i["number"]))
        if reports_red_main(i)
    ]


def checks_summary(r: str, sha: str) -> str:
    data = gh(f"repos/{r}/commits/{sha}/check-runs?per_page=100") or {}
    runs = data.get("check_runs", [])
    if not runs:
        return "no checks"
    pending = sum(c["status"] != "completed" for c in runs)
    failed = [c["name"] for c in runs if c.get("conclusion") in ("failure", "timed_out")]
    if failed:
        return "failing: " + ", ".join(sorted(set(failed)))
    return f"running ({pending})" if pending else "green"


def collect() -> dict[str, Any]:
    r = _github.repository()
    branch = _git.output("rev-parse", "--abbrev-ref", "HEAD")
    _git.output("fetch", "--quiet", "origin", "main")
    state: dict[str, Any] = {
        "repository": r,
        "branch": branch,
        "branch_issue": branch_issue(branch),
        "uncommitted_files": [ln for ln in _git.output("status", "--porcelain").splitlines() if ln],
        "ahead_of_main": _git.output("rev-list", "--count", "origin/main..HEAD") or "0",
        "behind_main": _git.output("rev-list", "--count", "HEAD..origin/main") or "0",
        "recent_commits": _git.output("log", "-8", "--format=%h %s").splitlines(),
        "journal": [
            p.name
            for p in sorted(JOURNAL.glob("*.md"), reverse=True)
            if p.name not in ("README.md", "_template.md")
        ][:5],
    }
    prs = _github.pages(f"repos/{r}/pulls?state=open", gh)
    state["pull_requests"] = [
        {
            "number": p["number"],
            "title": p["title"],
            "branch": p["head"]["ref"],
            "draft": p["draft"],
            "labels": sorted(label_names(p)),
            "checks": checks_summary(r, p["head"]["sha"]),
            "merge": merge_tier(
                [f["filename"] for f in _github.pages(f"repos/{r}/pulls/{p['number']}/files", gh)]
            ),
            "reviews": sorted(
                {rv["state"] for rv in _github.pages(f"repos/{r}/pulls/{p['number']}/reviews", gh)}
            ),
        }
        for p in prs
    ]
    issues = [
        i for i in _github.pages(f"repos/{r}/issues?state=open", gh) if "pull_request" not in i
    ]

    state["red_main"] = red_main(issues)
    state["attention"] = attention_issues(issues)
    state["in_progress"] = [
        f"#{i['number']} {i['title']}" for i in issues if IN_PROGRESS in label_names(i)
    ]
    state["blocked"] = [f"#{i['number']} {i['title']}" for i in issues if BLOCKED in label_names(i)]
    sprint = None
    for f in sorted((ROOT / "sprints").glob("S*.yaml")):
        data = yaml.safe_load(f.read_text(encoding="utf-8"))
        if data.get("status") == "active":
            sprint = {
                "file": f.name,
                "goal": data["goal"],
                "end": str(data["end"]),
                "issues": [n for lane in data["lanes"] for n in lane.get("issues", [])],
            }
            sprint["state"] = [line for line in (_state_line(p) for p in state_files(f)) if line]
    state["active_sprint"] = sprint
    open_numbers = {i["number"] for i in issues}
    if sprint:
        pool = [i for i in issues if i["number"] in sprint["issues"]]
        state["sprint_done"] = [n for n in sprint["issues"] if n not in open_numbers]
    else:
        milestones = sorted({i["milestone"]["title"] for i in issues if i.get("milestone")})
        current = milestones[0] if milestones else None
        pool = [i for i in issues if i.get("milestone") and i["milestone"]["title"] == current]
        state["current_milestone"] = current
    busy = {pr["branch"] for pr in state["pull_requests"]}
    state["ready"] = ready_issues(pool, busy)
    return state


def render(s: dict[str, Any]) -> str:
    red = s.get("red_main") or []
    stop = (
        [
            f"stop: main is red ({'; '.join(red)}); merge nothing but its fix "
            "(docs/en/project/conventions.md, Pull requests).",
            "",
        ]
        if red
        else []
    )
    out = [
        *stop,
        f"# Session briefing — {s['repository']}",
        "",
        f"Branch `{s['branch']}`"
        + (f", issue #{s['branch_issue']}" if s["branch_issue"] else "")
        + f"; {s['ahead_of_main']} commit(s) ahead of main, {s['behind_main']} behind; "
        f"{len(s['uncommitted_files'])} uncommitted file(s).",
        "",
        "## Bugs and maintenance first",
        *([f"- {x}" for x in s.get("attention") or []] or ["- none"]),
        "",
        "## Recent commits",
        *[f"- {c}" for c in s["recent_commits"]],
        "",
        "## Latest journal entries",
        *([f"- .agents/journal/{j}" for j in s["journal"]] or ["- none"]),
        "",
        "## Open pull requests",
    ]
    if s["pull_requests"]:
        out += [
            "| PR | Title | Checks | Reviews | Merge | Labels |",
            "| --- | --- | --- | --- | --- | --- |",
        ]
        for p in s["pull_requests"]:
            draft = " (draft)" if p["draft"] else ""
            out.append(
                f"| #{p['number']}{draft} | {p['title']} | {p['checks']} | "
                f"{', '.join(p['reviews']) or 'none'} | {p.get('merge', 'agent')} | "
                f"{', '.join(p['labels'])} |"
            )
    else:
        out.append("None.")
    sp = s["active_sprint"]
    out += ["", "## Sprint"]
    if sp:
        out.append(
            f"{sp['file']}: {sp['goal']} (ends {sp['end']}); "
            f"{len(s['sprint_done'])}/{len(sp['issues'])} issues closed."
        )
        out += (
            ["Per-issue state (sprints state files, local recovery):"]
            + [f"- {line}" for line in sp.get("state") or []]
            if sp.get("state")
            else []
        )
    else:
        out.append(f"No active sprint. Current milestone: {s.get('current_milestone')}.")
    out += [
        "",
        "## In progress",
        *([f"- {x}" for x in s["in_progress"]] or ["- none"]),
        "",
        "## Blocked",
        *([f"- {x}" for x in s["blocked"]] or ["- none"]),
        "",
        "## Ready to start",
        "| Issue | Title | Parallel | Areas |",
        "| --- | --- | --- | --- |",
    ]
    out += [
        f"| #{i['number']} | {i['title']} | {'yes' if i['parallel'] else 'no'} | "
        f"{', '.join(i['areas'])} |"
        for i in s["ready"]
    ]
    return "\n".join(out)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    state = collect()
    print(json.dumps(state, indent=2) if args.json else render(state))


if __name__ == "__main__":
    main()
