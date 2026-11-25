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
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path

from _labels import BREAKING_CHANGE, NO_ISSUE

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
REVERT = "revert"  # the type whose scope is optional; GitHub's Revert branch starts `revert-`
MAX_HEADER = 72
BRANCH_MAX_CHARS = 100
SLUG_MAX_WORDS = 5
SPRINT = "sprint"
SPRINT_MIN_DIGITS = 2
BRANCH_EXEMPT = ("dependabot/", "release/v")
# The largest issue number: the largest safe integer of the contracts, 2^53 - 1.
SAFE_INTEGER = 2**53 - 1
# Headers that git writes itself are exempt, by their form and never by their author.
GENERATED = ("Merge ", 'Revert "', "fixup! ", "squash! ")
# Bots skip the issue link of a pull request, which they cannot write; never the naming rules.
BOTS = ("dependabot[bot]", "github-actions[bot]", "renovate[bot]")
# The paragraph or footer that a breaking commit needs in its body.
BREAKING_CHANGE_TEXT = "BREAKING CHANGE:"
# The white space of a blank line of a body and around the text of a `BREAKING CHANGE:` line.
BLANK = " \t"
# Unicode White_Space, as Rust's `char::is_whitespace` reads it. `str.isspace` also counts
# U+001C to U+001F, so the rules below never use it.
WHITE_SPACE = (
    "\t\n\x0b\x0c\r \x85\xa0\u1680\u2000\u2001\u2002\u2003\u2004\u2005\u2006\u2007"
    "\u2008\u2009\u200a\u2028\u2029\u202f\u205f\u3000"
)

# The grammar of branches and headers is SPEC-0001 R11, which the engine checks too
# (crates/wayloop-core/src/naming/). These functions follow the engine step by step, so that the
# two agree: tests/process/test_conventions.py sends the shared cases of
# tests/process/naming_cases.json, every header of the history of main and every remote branch
# through both.


@dataclass(frozen=True)
class Header:
    """A header in the form of the grammar, in its parts."""

    type: str
    scope: str | None
    breaking: bool
    subject: str  # without its reference


def _digits(text: str) -> bool:
    return text != "" and all("0" <= c <= "9" for c in text)


def _counter(text: str) -> bool:
    """`[1-9][0-9]*`."""
    return _digits(text) and not text.startswith("0")


def _sequence(text: str) -> bool:
    """`[0-9]{4}|[1-9][0-9]{4,}`."""
    return _digits(text) and len(text) >= 4 and (len(text) == 4 or not text.startswith("0"))


def _is_reference(inner: str) -> bool:
    """Whether `inner` is `SPEC-<seq>[ R<k>[, R<k>]*]`, a reference without its brackets."""
    spec, _, requirements = inner.partition(" ")
    if not (spec.startswith("SPEC-") and _sequence(spec[len("SPEC-") :])):
        return False
    if " " not in inner:
        return True
    return all(r.startswith("R") and _counter(r[1:]) for r in requirements.split(", "))


def _without_reference(subject: str) -> str:
    """The subject without its final reference ` [SPEC-<seq>[ R<k>[, R<k>]*]]`."""
    if subject.endswith("]"):
        at = subject[:-1].rfind(" [")
        if at >= 0 and _is_reference(subject[at + 2 : -1]):
            return subject[:at]
    return subject


def parse_header(header: str) -> Header | None:
    """The header in its parts, or None when it is not `<type>(<scope>)[!]: <subject>` with a
    type without white space and a subject of at least one character. The scope ends at the
    first `)`."""
    ends = [i for i in (header.find(c) for c in "(!:") if i >= 0]
    if not ends:
        return None
    kind, rest = header[: min(ends)], header[min(ends) :]
    if not kind or any(c in WHITE_SPACE for c in kind):
        return None
    scope = None
    if rest.startswith("("):
        close = rest.find(")")
        if close < 0:
            return None
        scope, rest = rest[1:close] or None, rest[close + 1 :]
    breaking = rest.startswith("!")
    rest = rest.removeprefix("!")
    if not rest.startswith(": ") or len(rest) == 2:
        return None
    return Header(kind, scope, breaking, _without_reference(rest[2:]))


def header_faults(header: str, types: Sequence[str], scopes: Sequence[str]) -> list[str]:
    """The faults of a header, in the order of the engine: `form`, `type`, `no_scope`, `scope`,
    `start`, `full_stop`, `length`."""
    parsed = parse_header(header)
    faults = []
    if parsed is None:
        faults.append("form")
    else:
        if parsed.type not in types:
            faults.append("type")
        if parsed.scope is None and parsed.type != REVERT:
            faults.append("no_scope")
        elif parsed.scope is not None and parsed.scope not in scopes:
            faults.append("scope")
        if not parsed.subject[:1].islower():
            faults.append("start")
        if parsed.subject.rstrip(WHITE_SPACE).endswith("."):
            faults.append("full_stop")
    if len(header) > MAX_HEADER:
        faults.append("length")
    return faults


def is_generated(header: str) -> bool:
    """Whether git wrote the header itself, so that it is exempt from the grammar."""
    return header.startswith(GENERATED)


def _lines(text: str) -> list[str]:
    """The lines of `text` as Rust's `str::lines` reads them: split at `\\n`, without a final
    `\\r`."""
    lines = text.split("\n")
    if lines and lines[-1] == "":
        lines.pop()
    return [line.removesuffix("\r") for line in lines]


def has_breaking_change(body: str) -> bool:
    """Whether a body states a breaking change: a line `BREAKING CHANGE: <text>` that starts a
    paragraph, or a footer, a line of the last paragraph."""
    starts_paragraph, footer = True, False
    for line in _lines(body):
        if line.strip(BLANK) == "":
            starts_paragraph = True
            continue
        states = line.startswith(BREAKING_CHANGE_TEXT) and bool(
            line[len(BREAKING_CHANGE_TEXT) :].strip(BLANK)
        )
        if starts_paragraph:
            if states:
                return True
            footer, starts_paragraph = False, False
        else:
            footer = footer or states
    return footer


def commit_faults(
    header: str, body: str | None, types: Sequence[str], scopes: Sequence[str]
) -> list[str]:
    """The faults of the header and the body of a commit under R11: none for a header git wrote;
    `requires` first when the header marks a breaking change and the body states none."""
    if is_generated(header):
        return []
    faults = header_faults(header, types, scopes)
    parsed = parse_header(header)
    if parsed is not None and parsed.breaking and not has_breaking_change(body or ""):
        faults.insert(0, "requires")
    return faults


def _is_slug(text: str) -> bool:
    return all(w != "" and all(c in _SLUG_CHARS for c in w) for w in text.split("-"))


_SLUG_CHARS = frozenset("abcdefghijklmnopqrstuvwxyz0123456789")


def _is_issue(text: str) -> bool:
    """`[1-9][0-9]*` within the range of an issue number; the length is tested first, since
    `int` refuses a text of thousands of digits."""
    return _counter(text) and len(text) <= len(str(SAFE_INTEGER)) and int(text) <= SAFE_INTEGER


def _is_exempt(branch: str) -> bool:
    """`dependabot/<anything>`, `release/v<anything>` or `revert-<n>-<anything>`."""
    if branch.startswith(BRANCH_EXEMPT):
        return True
    revert = f"{REVERT}-"
    number, dash, _ = branch.removeprefix(revert).partition("-")
    return branch.startswith(revert) and bool(dash) and _digits(number)


def _is_sprint(text: str) -> bool:
    return text.startswith("S") and len(text) - 1 >= SPRINT_MIN_DIGITS and _digits(text[1:])


def _branch_parts(branch: str, types: Sequence[str]) -> tuple[list[str], str | None]:
    """The faults of a branch that is not exempt, in the order of the engine (`type`, `form` or
    `words`, `length`), and its `<issue>-<slug>` when it has one."""
    faults = []
    named = None
    head, slash, rest = branch.partition("/")
    if slash and head == SPRINT and "/" in rest:
        sprint, _, after = rest.partition("/")
        named = after if _is_sprint(sprint) else None
    elif slash and head:
        if head not in types:
            faults.append("type")
        named = rest
    words = None
    if named is not None:
        issue, dash, after = named.partition("-")
        if dash and _is_issue(issue):
            words = after
    if words is None or not _is_slug(words):
        faults.append("form")
    elif len(words.split("-")) > SLUG_MAX_WORDS:
        faults.append("words")
    if len(branch) > BRANCH_MAX_CHARS:
        faults.append("length")
    return faults, named if words is not None else None


def branch_faults(branch: str, types: Sequence[str]) -> list[str]:
    """The faults of a branch name under R11; none for an exempt branch."""
    return [] if _is_exempt(branch) else _branch_parts(branch, types)[0]


def branch_issue(branch: str) -> int | None:
    """The issue of a branch that follows the conventions, or None."""
    if _is_exempt(branch):
        return None
    faults, named = _branch_parts(branch, TYPES)
    return int(named.partition("-")[0]) if named is not None and not faults else None


def sprint_of(branch: str) -> str | None:
    """The sprint id of a sprint branch (`sprint/S01/42-slug` gives `S01`), or None."""
    parts = branch.split("/")
    if branch_issue(branch) is None or len(parts) != 3 or parts[0] != SPRINT:
        return None
    return parts[1]


# GitHub closes an issue when a pull request body names it after one of these keywords, in any
# case and anywhere in the body, with an optional colon after the keyword.
CLOSING = re.compile(r"\b(?:close[sd]?|fix(?:e[sd])?|resolve[sd]?):?[ \t]+#(\d+)\b", re.IGNORECASE)
# A link that does not close: `Refs #42`, or the `Refs: #42` trailer of a commit.
REFS = re.compile(r"\brefs?:?[ \t]+#(\d+)\b", re.IGNORECASE)


def closed_issues(body: str) -> set[int]:
    """Issues that GitHub closes when a pull request with this body is merged."""
    return {int(n) for n in CLOSING.findall(body)}


def linked_issues(body: str) -> set[int]:
    """Issues that a body closes or refers to."""
    return closed_issues(body) | {int(n) for n in REFS.findall(body)}


_HEADER_PROBLEMS = {
    "form": "'{header}' is not '<type>(<scope>)[!]: <subject>'.",
    "type": "the type is not one of: " + ", ".join(TYPES) + ".",
    "no_scope": "a scope is required, one of: " + ", ".join(SCOPES) + ".",
    "scope": "the scope is not one of: " + ", ".join(SCOPES) + ".",
    "start": "the subject starts with a lower-case letter ('add', not 'Added').",
    "full_stop": "the subject does not end with a full stop.",
    "length": "the header is {length} characters, the limit is " + str(MAX_HEADER) + ".",
    "requires": "a breaking header ('!') needs a 'BREAKING CHANGE: <what>' paragraph or footer.",
}


def check_header(header: str) -> list[str]:
    """Problems with a Conventional Commit header (commit subject or pull request title)."""
    return [
        _HEADER_PROBLEMS[f].format(header=header, length=len(header))
        for f in header_faults(header, TYPES, SCOPES)
    ]


_BRANCH_PROBLEMS = {
    "type": "the type is not one of: " + ", ".join(TYPES) + ".",
    "form": "'{branch}' is not '<type>/<issue>-<slug>' or 'sprint/SNN/<issue>-<slug>'"
    " (for example feat/42-ledger-hash-chain).",
    "words": f"the slug has more than {SLUG_MAX_WORDS} words.",
    "length": f"the branch has more than {BRANCH_MAX_CHARS} characters.",
}


def check_commit_message(text: str) -> list[str]:
    lines = [ln for ln in _lines(text) if not ln.startswith("#")]
    if not lines or not lines[0].strip(WHITE_SPACE):
        return ["empty commit message."]
    header = lines[0]
    if is_generated(header):
        return []
    body = "\n".join(lines[1:])
    problems = [
        _HEADER_PROBLEMS[f].format(header=header, length=len(header))
        for f in commit_faults(header, body, TYPES, SCOPES)
    ]
    if len(lines) > 1 and lines[1].strip(WHITE_SPACE):
        problems.append("the second line must be blank (header, blank line, body).")
    if not any(ln.startswith("Signed-off-by: ") for ln in lines):
        problems.append("missing 'Signed-off-by:' trailer: commit with `git commit -s` (DCO).")
    return problems


def check_pull_request(
    title: str, branch: str, body: str, labels: list[str], author: str
) -> list[str]:
    # The title and the branch follow R11 whoever opened the pull request; a bot only skips the
    # issue link, which it cannot write.
    problems = [f"title: {p}" for p in check_header(title)]
    problems += [
        f"branch: {_BRANCH_PROBLEMS[f].format(branch=branch)}" for f in branch_faults(branch, TYPES)
    ]
    if author in BOTS:
        return problems
    header = parse_header(title)
    breaking = header is not None and header.breaking
    if breaking and BREAKING_CHANGE not in labels:
        problems.append(f"labels: a breaking title ('!') needs the label '{BREAKING_CHANGE}'.")
    if breaking and not has_breaking_change(body or ""):
        problems.append(
            "body: a breaking title ('!') needs a 'BREAKING CHANGE: <what>' paragraph or footer."
        )
    if BREAKING_CHANGE in labels and not breaking:
        problems.append(f"title: the label '{BREAKING_CHANGE}' needs '!' after the scope.")
    linked = linked_issues(body or "")
    issue = branch_issue(branch)
    if NO_ISSUE not in labels:
        if not linked:
            problems.append(
                "body: link the issue with 'Closes #<n>' (or 'Refs #<n>'), "
                f"or label the pull request '{NO_ISSUE}' for trivial changes."
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
