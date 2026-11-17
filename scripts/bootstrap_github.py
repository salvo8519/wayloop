"""Reproduce the plan of roadmap/roadmap.yaml on a GitHub repository.

Creates what is missing: labels, milestones (gate and release in the description), epics
(label `epic`), tasks as sub-issues of their epic, and the native blocked-by dependencies
listed under "## Depends on" in the task bodies. Issues are matched by number when the number
carries the same title, otherwise by title; nothing is ever deleted or renamed.

With `--source OWNER/REPO` the bodies of new issues are copied from the issues of that
repository, with every `#N` rewritten to the number of the same task in the target, so a fork
or a restored repository gets the full issues, not only their titles.

    python scripts/bootstrap_github.py --dry-run                      # this repository
    python scripts/bootstrap_github.py --repo you/wayloop --source salvo8519/wayloop

Uses the REST API only. Day-to-day changes go through `scripts/roadmap.py`.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import yaml

ROOT = Path(__file__).resolve().parents[1]
ROADMAP = ROOT / "roadmap" / "roadmap.yaml"
DEFAULT_REPO = "salvo8519/wayloop"
TAG_LABELS = {
    "parallel": "parallel",
    "maintainer": "maintainer",
    "needs-decision": "needs-decision",
}
DEPENDS = re.compile(r"^## Depends on\s*\n((?:- #\d+.*\n?)+)", re.MULTILINE)
REFERENCE = re.compile(r"#(\d+)\b")


# ---- the plan: pure functions, tested offline


@dataclass
class Item:
    """An epic or a task of the roadmap and the issue that carries it."""

    kind: str  # "epic" or "task"
    planned: int | None  # issue number in the roadmap file
    title: str
    milestone: str
    labels: list[str]
    parent: Item | None = None
    number: int | None = None  # issue number in the target repository, once matched or created
    how: str = ""  # "number", "title" or "create"

    def target(self) -> int:
        """The issue number in the target repository; apply() sets it before reading it."""
        if self.number is None:
            raise UnmatchedItemError(self)
        return self.number


class UnmatchedItemError(LookupError):
    """An item was read before it was matched to an issue or created."""

    def __init__(self, item: Item) -> None:
        super().__init__(f"{item.kind} '{item.title}' has no issue in the target repository")


@dataclass
class Plan:
    labels: list[dict[str, str]] = field(default_factory=list)
    milestones: list[dict[str, str]] = field(default_factory=list)
    items: list[Item] = field(default_factory=list)


def milestone_description(m: dict[str, Any]) -> str:
    return f"Gate: {m['gate']} Release {m['release']}."


def items_of(data: dict[str, Any]) -> list[Item]:
    out: list[Item] = []
    for m in data["milestones"]:
        for e in m["epics"]:
            epic = Item(
                "epic", e.get("issue"), e["title"], m["title"], ["epic", *e.get("labels", [])]
            )
            out.append(epic)
            for t in e["tasks"]:
                labels = [*e.get("labels", []), *(TAG_LABELS[x] for x in t.get("tags", []))]
                out.append(Item("task", t["issue"], t["title"], m["title"], labels, parent=epic))
    return out


def plan(
    data: dict[str, Any], labels: set[str], milestones: set[str], issues: dict[int, dict[str, Any]]
) -> Plan:
    """What must be created so that the target matches the roadmap, and how each item matches."""
    p = Plan()
    p.labels = [lbl for lbl in data["labels"] if lbl["name"] not in labels]
    p.milestones = [
        {"title": m["title"], "description": milestone_description(m)}
        for m in data["milestones"]
        if m["title"] not in milestones
    ]
    by_title: dict[str, int] = {}
    for number, issue in sorted(issues.items()):
        by_title.setdefault(issue["title"].casefold(), number)
    taken: set[int] = set()
    for item in items_of(data):
        same = issues.get(item.planned) if item.planned is not None else None
        if same is not None and same["title"].casefold() == item.title.casefold():
            item.number, item.how = item.planned, "number"
        elif item.title.casefold() in by_title and by_title[item.title.casefold()] not in taken:
            item.number, item.how = by_title[item.title.casefold()], "title"
        else:
            item.how = "create"
        if item.number is not None:
            taken.add(item.number)
        p.items.append(item)
    return p


def renumber(body: str, mapping: dict[int, int]) -> str:
    """Rewrite every #N of a source body to the number of the same item in the target."""
    return REFERENCE.sub(lambda m: f"#{mapping.get(int(m.group(1)), int(m.group(1)))}", body)


def depends_on(body: str) -> list[int]:
    """Issue numbers listed under the "## Depends on" heading of a task body."""
    m = DEPENDS.search(body or "")
    return [int(n) for n in re.findall(r"^- #(\d+)", m.group(1), re.MULTILINE)] if m else []


# ---- GitHub


class GitHub:
    def __init__(self, repo: str, dry_run: bool) -> None:
        self.repo, self.dry_run = repo, dry_run

    def api(self, method: str, path: str, payload: dict[str, Any] | None = None) -> Any:
        if self.dry_run and method != "GET":
            print(
                f"[dry-run] {method} {path} {json.dumps(payload or {}, ensure_ascii=False)[:100]}"
            )
            return {"number": 0, "id": 0}
        cmd = ["gh", "api", "-X", method, f"repos/{self.repo}/{path}"]
        if payload is not None:
            cmd += ["--input", "-"]
        for attempt in range(3):
            out = subprocess.run(
                cmd,
                input=json.dumps(payload) if payload is not None else None,
                capture_output=True,
                text=True,
                encoding="utf-8",
                check=False,
            )
            if out.returncode == 0:
                break
            time.sleep(2 * (attempt + 1))
        else:
            sys.exit(f"{method} {path} failed: {out.stderr.strip()}")
        if method != "GET":
            time.sleep(0.5)  # stay below the secondary rate limit on content creation
        return json.loads(out.stdout) if out.stdout.strip() else None

    def pages(self, path: str) -> list[dict[str, Any]]:
        out: list[dict[str, Any]] = []
        page = 1
        while True:
            batch = self.api("GET", f"{path}{'&' if '?' in path else '?'}per_page=100&page={page}")
            out += batch
            if len(batch) < 100:
                return out
            page += 1

    def issues(self) -> dict[int, dict[str, Any]]:
        return {i["number"]: i for i in self.pages("issues?state=all") if "pull_request" not in i}


def apply(p: Plan, gh: GitHub, issues: dict[int, dict[str, Any]], source: GitHub | None) -> int:
    changes = 0
    for lbl in p.labels:
        gh.api("POST", "labels", {"name": lbl["name"], "color": lbl["color"]})
        changes += 1
    for m in p.milestones:
        gh.api("POST", "milestones", m)
        changes += 1
    numbers = {m["title"]: m["number"] for m in gh.pages("milestones?state=all")}
    source_issues = source.issues() if source else {}

    for item in p.items:
        if item.how == "create":
            created = gh.api(
                "POST",
                "issues",
                {
                    "title": item.title,
                    "body": "Details follow.",
                    "milestone": numbers.get(item.milestone),
                    "labels": item.labels,
                },
            )
            item.number = created["number"]
            issues[item.number] = created
            changes += 1
        elif item.how == "title":
            print(
                f"{item.kind} '{item.title}': planned as #{item.planned}, found as #{item.number}"
            )

    mapping = {i.planned: i.number for i in p.items if i.planned is not None and i.number}
    for item in p.items:
        if item.how != "create":
            continue
        planned_body = source_issues.get(item.planned or -1, {}).get("body") or ""
        parent = f"Part of #{item.parent.number}." if item.parent else ""
        body = renumber(planned_body, mapping) if planned_body else parent
        gh.api("PATCH", f"issues/{item.number}", {"body": body})

    for epic in (i for i in p.items if i.kind == "epic"):
        children = {
            c["number"]
            for c in (
                []
                if gh.dry_run and epic.how == "create"
                else gh.pages(f"issues/{epic.target()}/sub_issues")
            )
        }
        for task in (i for i in p.items if i.parent is epic):
            if task.target() not in children:
                gh.api(
                    "POST",
                    f"issues/{epic.target()}/sub_issues",
                    {"sub_issue_id": issues[task.target()]["id"], "replace_parent": True},
                )
                changes += 1

    for task in (i for i in p.items if i.kind == "task"):
        body = issues.get(task.target(), {}).get("body") or ""
        if task.how == "create" and source:
            body = renumber(source_issues.get(task.planned or -1, {}).get("body") or "", mapping)
        wanted = [mapping.get(n, n) for n in depends_on(body)]
        if not wanted:
            continue
        have = (
            set()
            if task.how == "create" and gh.dry_run
            else {d["number"] for d in gh.pages(f"issues/{task.target()}/dependencies/blocked_by")}
        )
        for blocker in wanted:
            if blocker not in have and blocker in issues:
                gh.api(
                    "POST",
                    f"issues/{task.target()}/dependencies/blocked_by",
                    {"issue_id": issues[blocker]["id"]},
                )
                changes += 1
    return changes


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--repo", default=DEFAULT_REPO, help="target repository, OWNER/REPO")
    parser.add_argument("--source", help="repository whose issue bodies are copied, OWNER/REPO")
    parser.add_argument("--dry-run", action="store_true", help="print the changes, make none")
    args = parser.parse_args(argv)
    data = yaml.safe_load(ROADMAP.read_text(encoding="utf-8"))
    gh = GitHub(args.repo, args.dry_run)
    issues = gh.issues()
    p = plan(
        data,
        {lbl["name"] for lbl in gh.pages("labels?")},
        {m["title"] for m in gh.pages("milestones?state=all")},
        issues,
    )
    source = GitHub(args.source, True) if args.source else None
    changes = apply(p, gh, issues, source)
    created = sum(1 for i in p.items if i.how == "create")
    dry_run = " (dry run)" if args.dry_run else ""
    print(f"{len(p.items)} epics and tasks, {created} to create; {changes} changes{dry_run}.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
