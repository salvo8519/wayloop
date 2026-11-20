"""Naming conventions of Wayloop, as code: commit headers, pull request titles, branches and
issue links (closing keywords as GitHub reads them). One module, used by the commit-msg hook,
by CI, by the other scripts and by the agents' skills, so the rule written in
`docs/en/project/conventions.md` and the rule enforced are the same.

    uv run python scripts/conventions.py commit-msg .git/COMMIT_EDITMSG
    uv run python scripts/conventions.py pr           # in GitHub Actions (reads the event)
    uv run python scripts/conventions.py pr --title "feat(ledger): chain events by hash" \
        --branch feat/42-ledger-hash-chain --body "Closes #42"
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
from pathlib import Path

TYPES = ("feat", "fix", "perf", "refactor", "docs", "test", "build", "ci", "chore", "revert")
SCOPES = (
    # engine
    "core", "runtime", "ledger", "state", "tracker", "survey", "schemas", "security",
    # adapters
    "cli", "python", "typescript", "mcp", "plugins",
    # documentation and project
    "docs", "i18n", "brand", "specs", "adr", "agents", "project", "roadmap",
    # build and delivery
    "ci", "deps", "release", "scripts", "benchmark",
)  # fmt: skip
MAX_HEADER = 72
BOTS = ("dependabot[bot]", "github-actions[bot]", "renovate[bot]")

HEADER = re.compile(
    r"^(?P<type>[a-z]+)(?:\((?P<scope>[a-z0-9-]+)\))?(?P<breaking>!)?: (?P<subject>.+)$"
)
SPEC_REF = re.compile(r"\[SPEC-\d{4}(?: R\d+(?:,\s*R\d+)*)?\]$")
SLUG = r"[a-z0-9]+(?:-[a-z0-9]+)*"
# A branch is `<type>/<issue>-<slug>`, or `sprint/SNN/<issue>-<slug>` in a sprint lane
# (sprints/README.md). The hook .claude/hooks/session-briefing.sh keeps a copy of this rule in
# sed, because it runs without the project environment; tests/process/test_claude_hooks.py keeps
# the copy equal.
BRANCH = re.compile(
    rf"^(?:(?P<type>{'|'.join(TYPES)})|sprint/(?P<sprint>S\d+))/(?P<issue>\d+)-(?P<slug>{SLUG})$"
)
BRANCH_EXEMPT = re.compile(r"^(dependabot/|release/v\d|revert-\d+-)")
# GitHub closes an issue when a pull request body names it after one of these keywords, in any
# case and anywhere in the body, with an optional colon after the keyword.
CLOSING = re.compile(r"\b(?:close[sd]?|fix(?:e[sd])?|resolve[sd]?):?[ \t]+#(\d+)\b", re.IGNORECASE)
# A link that does not close: `Refs #42`, or the `Refs: #42` trailer of a commit.
REFS = re.compile(r"\brefs?:?[ \t]+#(\d+)\b", re.IGNORECASE)


def branch_issue(branch: str) -> int | None:
    """The issue of a branch that follows the conventions, or None."""
    m = BRANCH.match(branch)
    return int(m["issue"]) if m else None


def sprint_of(branch: str) -> str | None:
    """The sprint id of a sprint branch (`sprint/S01/42-slug` gives `S01`), or None."""
    m = BRANCH.match(branch)
    return m["sprint"] if m else None


def closed_issues(body: str) -> set[int]:
    """Issues that GitHub closes when a pull request with this body is merged."""
    return {int(n) for n in CLOSING.findall(body)}


def linked_issues(body: str) -> set[int]:
    """Issues that a body closes or refers to."""
    return closed_issues(body) | {int(n) for n in REFS.findall(body)}


def check_header(header: str, *, bot: bool = False) -> list[str]:
    """Problems with a Conventional Commit header (commit subject or pull request title)."""
    m = HEADER.match(header)
    if not m:
        return [f"'{header}' is not '<type>(<scope>): <subject>'."]
    problems = []
    if m["type"] not in TYPES:
        problems.append(f"type '{m['type']}' is not one of: {', '.join(TYPES)}.")
    if m["type"] != "revert":
        if not m["scope"]:
            problems.append(f"a scope is required, one of: {', '.join(SCOPES)}.")
        elif m["scope"] not in SCOPES:
            problems.append(f"scope '{m['scope']}' is not one of: {', '.join(SCOPES)}.")
    subject = SPEC_REF.sub("", m["subject"]).strip()
    if not bot:
        if len(header) > MAX_HEADER:
            problems.append(f"header is {len(header)} characters, the limit is {MAX_HEADER}.")
        if subject[:1].isupper():
            problems.append("subject starts with a lowercase imperative verb ('add', not 'Added').")
        if subject.endswith("."):
            problems.append("subject does not end with a full stop.")
    return problems


def check_commit_message(text: str) -> list[str]:
    lines = [ln for ln in text.splitlines() if not ln.startswith("#")]
    if not lines or not lines[0].strip():
        return ["empty commit message."]
    header = lines[0].strip()
    if header.startswith(("Merge ", "fixup! ", "squash! ", "amend! ", 'Revert "')):
        return []
    problems = check_header(header)
    if len(lines) > 1 and lines[1].strip():
        problems.append("the second line must be blank (header, blank line, body).")
    if not any(ln.startswith("Signed-off-by: ") for ln in lines):
        problems.append("missing 'Signed-off-by:' trailer: commit with `git commit -s` (DCO).")
    return problems


def check_pull_request(
    title: str, branch: str, body: str, labels: list[str], author: str
) -> list[str]:
    bot = author in BOTS
    problems = [f"title: {p}" for p in check_header(title, bot=bot)]
    if bot:
        return problems
    header = HEADER.match(title)
    breaking = bool(header and header["breaking"])
    if breaking and "breaking-change" not in labels:
        problems.append("labels: a breaking title ('!') needs the label 'breaking-change'.")
    if breaking and not re.search(r"^BREAKING CHANGE: \S", body or "", re.MULTILINE):
        problems.append("body: a breaking title ('!') needs a 'BREAKING CHANGE: <what>' paragraph.")
    if "breaking-change" in labels and not breaking:
        problems.append("title: the label 'breaking-change' needs '!' after the scope.")
    linked = linked_issues(body or "")
    issue = branch_issue(branch)
    if issue is None and not BRANCH_EXEMPT.match(branch):
        problems.append(
            f"branch: '{branch}' is not '<type>/<issue>-<slug>' or 'sprint/SNN/<issue>-<slug>'"
            " (for example feat/42-ledger-hash-chain)."
        )
    if "no-issue" not in labels:
        if not linked:
            problems.append(
                "body: link the issue with 'Closes #<n>' (or 'Refs #<n>'), "
                "or label the pull request 'no-issue' for trivial changes."
            )
        elif issue is not None and issue not in linked:
            problems.append(f"branch refers to #{issue} but the body links {sorted(linked)}.")
    return problems


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="cmd", required=True)
    c = sub.add_parser("commit-msg")
    c.add_argument("file")
    p = sub.add_parser("pr")
    p.add_argument("--title")
    p.add_argument("--branch")
    p.add_argument("--body", default="")
    p.add_argument("--label", action="append", default=[])
    p.add_argument("--author", default="")
    args = parser.parse_args()

    if args.cmd == "commit-msg":
        problems = check_commit_message(Path(args.file).read_text(encoding="utf-8"))
        where = "Commit message"
    else:
        title, branch, body, labels, author = (
            args.title, args.branch, args.body, args.label, args.author
        )  # fmt: skip
        event = os.environ.get("GITHUB_EVENT_PATH")
        if title is None and event:
            pr = json.loads(Path(event).read_text(encoding="utf-8"))["pull_request"]
            title, branch, body = pr["title"], pr["head"]["ref"], pr.get("body") or ""
            labels = [lbl["name"] for lbl in pr["labels"]]
            author = pr["user"]["login"]
        if title is None or branch is None:
            parser.error("pr needs --title and --branch outside GitHub Actions")
        problems = check_pull_request(title, branch, body, labels, author)
        where = "Pull request"
    if problems:
        print(f"{where} does not follow docs/en/project/conventions.md:")
        for problem in problems:
            print(f"  - {problem}")
        return 1
    print(f"{where} follows the conventions.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
