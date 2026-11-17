"""Keep the documentation in sync with the project, deterministically.

Two checks, used by `just`, CI and the Claude Code Stop hook:

  coverage   every ADR file has its row in the ADR index and its navigation entry, with a valid
             status line; every specification has a valid status; every page of the navigation
             exists and every page is in the navigation
  impact     files changed in a way that needs documentation (engine, command line, schemas,
             process scripts) come with a matching documentation change

    python scripts/docs_sync.py coverage
    python scripts/docs_sync.py impact --base origin/main     # CI, pull requests
    python scripts/docs_sync.py impact --staged               # pre-commit
    python scripts/docs_sync.py impact --worktree --hook      # Claude Code Stop hook

Only the standard library is used, so that hooks run without the project environment.
Escape hatch for a change that genuinely needs no documentation: the pull request label
`no-docs-needed`, or a commit trailer `Docs-impact: none` on the latest commit.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
DOCS = ROOT / "docs" / "en"
ADR_DIR = DOCS / "project" / "adr"
ADR_STATUS = re.compile(
    r"^Status: (Proposed|Accepted|Rejected|Superseded by ADR \d{4}|Deprecated)\b", re.MULTILINE
)
SPEC_STATUS = re.compile(
    r"^\| Status \| (Draft|Approved|Surveyed|Implementing|Done|Superseded) \|$", re.MULTILINE
)
# Folders of docs/en copied in at build time, not pages of the site.
NOT_PAGES = ("assets/", "stylesheets/")

# (changed path pattern, documentation that must change with it, reason)
RULES: list[tuple[str, str, str]] = [
    (
        r"^crates/wayloop-core/src/",
        r"^(docs/en/(concepts|reference)/|specs/\d{4}-[^/]+/)",
        "engine changed: update docs/en/concepts/, docs/en/reference/ or its specification",
    ),
    (
        r"^crates/wayloop-cli/src/",
        r"^docs/en/(reference|guides|getting-started)/",
        "command line changed: update docs/en/reference/, guides/ or getting-started/",
    ),
    (
        r"^schemas/",
        r"^docs/en/reference/",
        "schemas changed: update docs/en/reference/",
    ),
    (
        r"^scripts/(roadmap|sprint|conventions|bootstrap_github|required_checks|docs_sync)\.py$",
        r"^(docs/en/project/conventions\.md|CONTRIBUTING\.md|AGENTS\.md)$",
        "process script changed: update the conventions page, CONTRIBUTING.md or AGENTS.md",
    ),
]


def git(*args: str) -> list[str]:
    out = subprocess.run(
        ["git", *args], cwd=ROOT, capture_output=True, text=True, check=True, encoding="utf-8"
    )
    return [line for line in out.stdout.splitlines() if line]


def changed_files(args: argparse.Namespace) -> list[str]:
    if args.staged:
        return git("diff", "--name-only", "--cached")
    if args.worktree:
        return sorted(
            set(git("diff", "--name-only", "HEAD"))
            | set(git("ls-files", "--others", "--exclude-standard"))
        )
    return sorted(set(git("diff", "--name-only", f"{args.base}...HEAD")))


def missing_docs(files: list[str]) -> list[str]:
    """Reasons for every rule whose code changed without its documentation."""
    missing: list[str] = []
    for pattern, docs, reason in RULES:
        if any(re.match(pattern, f) for f in files) and not any(re.match(docs, f) for f in files):
            missing.append(reason)
    return missing


def impact(args: argparse.Namespace) -> int:
    if not args.worktree and not args.staged:
        # In pull-request CI, HEAD is the merge commit GitHub creates, which carries no
        # trailers; the waiver belongs to the latest commit of the branch itself.
        trailer = git("log", "-1", "--no-merges", "--format=%(trailers:key=Docs-impact,valueonly)")
        if any(t.strip().lower() == "none" for t in trailer):
            print("Docs impact waived by 'Docs-impact: none' trailer.")
            return 0
    missing = missing_docs(changed_files(args))
    if not missing:
        print("Docs impact check passed.")
        return 0
    stream = sys.stderr if args.hook else sys.stdout
    print("Documentation is out of sync with the changes:", file=stream)
    for m in missing:
        print(f"  - {m}", file=stream)
    if args.hook:
        print("Update the documentation before ending the session.", file=stream)
        return 2
    print(
        "Fix it, or label the pull request `no-docs-needed` if the change needs no documentation."
    )
    return 1


def nav_pages(nav: Any) -> list[str]:
    if isinstance(nav, str):
        return [nav]
    if isinstance(nav, list):
        return [p for item in nav for p in nav_pages(item)]
    if isinstance(nav, dict):
        return [p for value in nav.values() for p in nav_pages(value)]
    return []


def coverage_problems(root: Path = ROOT) -> list[str]:
    docs = root / "docs" / "en"
    adr_dir = docs / "project" / "adr"
    nav = nav_pages(
        tomllib.loads((root / "zensical.toml").read_text(encoding="utf-8"))["project"]["nav"]
    )
    index = (adr_dir / "index.md").read_text(encoding="utf-8")
    problems: list[str] = []
    for adr in sorted(adr_dir.glob("[0-9][0-9][0-9][0-9]-*.md")):
        number, rel = adr.name[:4], f"project/adr/{adr.name}"
        if f"[{number}]({adr.name})" not in index:
            problems.append(f"ADR {number}: no row in docs/en/project/adr/index.md")
        if rel not in nav:
            problems.append(f"ADR {number}: {rel} is not in the navigation of zensical.toml")
        if not ADR_STATUS.search(adr.read_text(encoding="utf-8")):
            problems.append(f"ADR {number}: no valid 'Status:' line")
    for row in re.findall(r"\]\((\d{4}-[^)]+\.md)\)", index):
        if not (adr_dir / row).exists():
            problems.append(f"ADR index links {row}, which does not exist")
    for spec in sorted((root / "specs").glob("[0-9][0-9][0-9][0-9]-*/spec.md")):
        if not SPEC_STATUS.search(spec.read_text(encoding="utf-8")):
            problems.append(f"{spec.relative_to(root).as_posix()}: no valid '| Status |' row")
    pages = {
        p.relative_to(docs).as_posix()
        for p in docs.rglob("*.md")
        if not p.relative_to(docs).as_posix().startswith(NOT_PAGES)
    }
    problems += [
        f"navigation lists {p}, which does not exist in docs/en/" for p in nav if p not in pages
    ]
    problems += [
        f"docs/en/{p} is not in the navigation of zensical.toml" for p in sorted(pages - set(nav))
    ]
    return problems


def coverage() -> int:
    problems = coverage_problems()
    if problems:
        print("Documentation coverage gaps:\n" + "\n".join(f"  - {p}" for p in problems))
        return 1
    print("Docs coverage passed: ADRs, specifications and navigation agree.")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    sub = parser.add_subparsers(dest="cmd", required=True)
    sub.add_parser("coverage")
    p = sub.add_parser("impact")
    group = p.add_mutually_exclusive_group(required=True)
    group.add_argument("--base")
    group.add_argument("--staged", action="store_true")
    group.add_argument("--worktree", action="store_true")
    p.add_argument("--hook", action="store_true", help="exit 2 with stderr (Claude Code hook)")
    args = parser.parse_args(argv)
    return coverage() if args.cmd == "coverage" else impact(args)


if __name__ == "__main__":
    sys.exit(main())
