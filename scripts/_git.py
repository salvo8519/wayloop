"""git access shared by the process scripts and the Claude Code hooks.

Standard library only: the hooks run with the system `python3`, without the project
environment, and import this file from `scripts/` (see `.claude/hooks/quality_gate.py`).

    import _git
    _git.ls_files("*.yml", "*.yaml")           # tracked files matching the pathspecs
    _git.changed("origin/main")                # files changed on the branch
    _git.changed(staged=True)                  # files in the index
    _git.changed(untracked=True)               # the working tree against HEAD, new files too
    _git.tags("v*")                            # tags matching a pattern
    _git.holds("v0.1.0", "schemas/x.json")     # whether a revision holds a path
"""

from __future__ import annotations

import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


class NotARevisionError(ValueError):
    """A name that git would not read as a revision."""

    def __init__(self, name: str) -> None:
        super().__init__(f"{name!r} is not a revision")


def revision(name: str) -> str:
    """`name`, checked to be read by git as a revision: a name starting with `-` would be read as
    an option (`--output=<file>` writes a file), so it is refused with NotARevisionError."""
    if not name or name.startswith("-"):
        raise NotARevisionError(name)
    return name


def run(*args: str, cwd: Path | str = ROOT) -> subprocess.CompletedProcess[str]:
    """`git <args>`, whatever its exit code: the caller reads `returncode`."""
    return subprocess.run(
        ["git", *args], cwd=cwd, capture_output=True, text=True, encoding="utf-8", check=False
    )


def output(*args: str, cwd: Path | str = ROOT) -> str:
    """The output of `git <args>` without surrounding blanks, or "" when git fails: for callers
    that go on without git."""
    out = run(*args, cwd=cwd)
    return out.stdout.strip() if out.returncode == 0 else ""


def lines(*args: str, cwd: Path | str = ROOT) -> list[str]:
    """The non-empty output lines of `git <args>`; a failure raises CalledProcessError."""
    out = run(*args, cwd=cwd)
    out.check_returncode()
    return [line for line in out.stdout.splitlines() if line]


def _names(*args: str, cwd: Path | str) -> list[str]:
    """Paths printed with -z by `git <args>`: exact names, whatever characters they hold."""
    command, *rest = args
    out = run(command, "-z", *rest, cwd=cwd)
    out.check_returncode()
    return [name for name in out.stdout.split("\0") if name]


def ls_files(*pathspec: str, others: bool = False, cwd: Path | str = ROOT) -> list[str]:
    """Tracked files matching the pathspecs (every file without one), sorted; with `others`,
    untracked files that are not ignored too."""
    extra = ["--others", "--exclude-standard"] if others else []
    return sorted(_names("ls-files", "--cached", *extra, "--", *pathspec, cwd=cwd))


def changed(
    base: str | None = None,
    *,
    staged: bool = False,
    untracked: bool = False,
    cwd: Path | str = ROOT,
) -> list[str]:
    """Changed paths, sorted, renames as a deletion and an addition.

    `staged`: the index against HEAD. `base`: the branch against its merge base with `base`, or
    against `base` itself when they share no history; a `base` that git would read as an option
    raises ValueError. Otherwise: the working tree against HEAD. `untracked` adds the new files
    that are not ignored. A failure of git raises CalledProcessError.
    """
    diff = ["diff", "--name-only", "--no-renames"]
    if staged:
        names = _names(*diff, "--cached", cwd=cwd)
    elif base is not None:
        revision(base)
        try:
            names = _names(*diff, "--end-of-options", f"{base}...HEAD", cwd=cwd)
        except subprocess.CalledProcessError:
            names = _names(*diff, "--end-of-options", base, "HEAD", cwd=cwd)
    else:
        names = _names(*diff, "HEAD", cwd=cwd)
    if untracked:
        names += _names("ls-files", "--others", "--exclude-standard", cwd=cwd)
    return sorted(set(names))


def tags(pattern: str, cwd: Path | str = ROOT) -> list[str]:
    """The tags matching the glob `pattern`, sorted; a failure raises CalledProcessError."""
    return sorted(lines("tag", "--list", "--", pattern, cwd=cwd))


def holds(name: str, path: str, cwd: Path | str = ROOT) -> bool:
    """Whether the tree of the revision `name` holds `path`. An absent path is False; any other
    failure of git, such as a revision that does not exist, raises CalledProcessError, never a
    silent False. A `name` that git would read as an option raises ValueError."""
    revision(name)
    entries = _names("ls-tree", "--name-only", "--end-of-options", name, "--", path, cwd=cwd)
    return path in entries
