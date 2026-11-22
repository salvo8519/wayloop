"""Direct edits of issue titles and bodies, recognised in a shell command.

The issues of the roadmap change only through `scripts/roadmap.py edit`, which keeps the parent
line of the body and the roadmap file in step (#312). The PreToolUse hook
`.claude/hooks/protect_issues.py` asks `reason_to_block` about every Bash command of an agent and
refuses:

- `gh issue edit` with `--title`, `--body` or `--body-file` (or `-t`, `-b`, `-F`);
- `gh api` on `repos/<owner>/<repo>/issues/<n>` with the method PATCH, POST or PUT (explicit, or
  POST implied by a field) that sets `title` or `body` with a field, or sends `--input` (whose
  content the hook cannot see);
- `gh api graphql` with an `updateIssue` mutation written in a field.

Comments (`issues/<n>/comments`), labels, assignees, closing and reading stay allowed. Commands
inside `bash -c`, `sh -c` and `eval` are read too. This is a guard against the common mistake,
not a sandbox: `roadmap.py links` and the Roadmap sync workflow remain the second line.

Standard library only: the hook runs with the system python3, outside the project environment.
"""

from __future__ import annotations

import re
import shlex
from collections.abc import Iterator, Sequence

EDIT_COMMAND = "uv run python scripts/roadmap.py edit <issue> [--title T] [--body FILE]"
# A command that may edit an issue. The hook keeps a copy to fail closed when this module cannot
# load; tests/process/test_issue_edits.py keeps the copy equal to this pattern.
LOOKS_LIKE_EDIT = re.compile(r"\bgh\b[\s\S]*\b(issue\s+edit|api\b[\s\S]*(issues/\d|graphql))")
ISSUE_PATH = re.compile(r"(^|/)repos/[^/\s]+/[^/\s]+/issues/\d+/?(\?.*)?$")
WRITES = {"PATCH", "POST", "PUT"}
EDITED = {"title", "body"}
SHELLS = {"bash", "sh", "zsh", "dash"}
# Options of `gh issue edit` and `gh api` that take a value as the next argument.
VALUE_OPTIONS = {
    "-X", "--method", "-f", "--raw-field", "-F", "--field", "-H", "--header", "--input",
    "-q", "--jq", "-t", "--template", "--hostname", "--cache", "-p", "--preview", "-R", "--repo",
    "--title", "-b", "--body", "--body-file", "--add-label", "--remove-label", "--milestone",
    "--add-assignee", "--remove-assignee", "--add-project", "--remove-project",
}  # fmt: skip
ISSUE_EDIT_OPTIONS = {"-t", "--title", "-b", "--body", "-F", "--body-file"}
FIELD_OPTIONS = {"-f", "--raw-field", "-F", "--field"}
OPERATORS = set("();<>|&")


def parse(args: Sequence[str]) -> tuple[list[tuple[str, str]], list[str]]:
    """(option, value) pairs and positional arguments of a gh command line."""
    options: list[tuple[str, str]] = []
    positional: list[str] = []
    i = 0
    while i < len(args):
        arg = args[i]
        if arg.startswith("--") and "=" in arg:
            name, value = arg.split("=", 1)
            options.append((name, value))
        elif arg.startswith("-") and not arg.startswith("--") and len(arg) > 2:
            options.append((arg[:2], arg[2:]))
        elif arg.startswith("-") and arg != "-":
            value = args[i + 1] if arg in VALUE_OPTIONS and i + 1 < len(args) else ""
            i += 1 if arg in VALUE_OPTIONS else 0
            options.append((arg, value))
        else:
            positional.append(arg)
        i += 1
    return options, positional


def _gh_issue_edit(options: list[tuple[str, str]], positional: list[str]) -> str | None:
    if positional[:2] != ["issue", "edit"]:
        return None
    used = sorted({name for name, _ in options if name in ISSUE_EDIT_OPTIONS})
    return f"`gh issue edit` with {', '.join(used)}" if used else None


def _gh_api(options: list[tuple[str, str]], positional: list[str]) -> str | None:
    if positional[:1] != ["api"] or len(positional) < 2:
        return None
    endpoint = positional[1]
    fields = [value for name, value in options if name in FIELD_OPTIONS]
    if endpoint == "graphql":
        return (
            "`gh api graphql` with updateIssue" if any("updateIssue" in f for f in fields) else None
        )
    if not ISSUE_PATH.search(endpoint):
        return None
    methods = [value.upper() for name, value in options if name in ("-X", "--method")]
    sends_input = any(name == "--input" for name, _ in options)
    method = methods[-1] if methods else ("POST" if fields or sends_input else "GET")
    if method not in WRITES:
        return None
    keys = {f.split("=", 1)[0].strip() for f in fields}
    if keys & EDITED:
        return f"`gh api {endpoint}` with {method} setting {', '.join(sorted(keys & EDITED))}"
    if sends_input:
        return f"`gh api {endpoint}` with {method} and --input, which may set the title or body"
    return None


def _segments(tokens: list[str]) -> Iterator[list[str]]:
    """The simple commands of a token list, split at shell operators."""
    current: list[str] = []
    for token in tokens:
        if token and set(token) <= OPERATORS:
            yield current
            current = []
        else:
            current.append(token)
    yield current


def _tokens(command: str) -> list[str]:
    lexer = shlex.shlex(command, posix=True, punctuation_chars=True)
    lexer.whitespace_split = True
    return list(lexer)


def reason_to_block(command: str, depth: int = 0) -> str | None:
    """What in `command` edits the title or body of an issue directly, or None. Raises
    ValueError when the command cannot be read (unbalanced quotes); the hook then fails closed
    on a command that looks like an edit."""
    for segment in _segments(_tokens(command)):
        for i, token in enumerate(segment):
            name = token.rsplit("/", 1)[-1]
            nested = None
            if name in SHELLS and depth < 3:
                flags = [j for j in range(i + 1, len(segment) - 1) if segment[j].startswith("-")]
                nested = next(
                    (segment[j + 1] for j in flags if "c" in segment[j].lstrip("-")), None
                )
            elif name == "eval" and depth < 3:
                nested = " ".join(segment[i + 1 :])
            if nested and (found := reason_to_block(nested, depth + 1)):
                return found
            if name == "gh":
                options, positional = parse(segment[i + 1 :])
                if found := _gh_issue_edit(options, positional) or _gh_api(options, positional):
                    return found
    return None


def message(reason: str) -> str:
    return (
        f"Blocked: {reason} changes an issue directly. Issues of the roadmap are changed only "
        f"with `{EDIT_COMMAND}`: it keeps the 'Part of #<epic>.' line and the roadmap file in "
        "step (AGENTS.md, Hard rules). Comments, labels and closing stay allowed.\n"
    )
