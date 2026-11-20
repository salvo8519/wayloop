"""GitHub access shared by the process scripts: the repository, the CLI, the REST API, pages.

Standard library only, so that hooks and workflows can import it without the project
environment. Every call goes through the GitHub CLI (`gh`), which holds the credentials.

    import _github
    repo = _github.repository()
    issue = _github.api(f"repos/{repo}/issues/42")
    labels = _github.pages(f"repos/{repo}/labels")
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import time
from collections.abc import Callable, Sequence
from typing import Any

import _git

DEFAULT_REPO = "salvo8519/wayloop"
# `https://github.com/owner/name(.git)` or `git@github.com:owner/name(.git)`.
REMOTE = re.compile(r"github\.com[:/](?P<repo>[^/\s]+/[^/\s]+?)(?:\.git)?/?$")
# A page of a list endpoint holds at most this many items, and a list is read in at most
# MAX_PAGES pages: an endpoint that never ends stops the script instead of looping.
PER_PAGE = 100
MAX_PAGES = 100


def repo_of_url(url: str) -> str | None:
    """`owner/name` of a GitHub remote URL, or None for another host."""
    m = REMOTE.search(url.strip())
    return m["repo"] if m else None


def repository() -> str:
    """The repository to work on: `GITHUB_REPOSITORY` in Actions, else the `origin` remote,
    else the canonical repository."""
    if env := os.environ.get("GITHUB_REPOSITORY"):
        return env
    return repo_of_url(_git.output("remote", "get-url", "origin")) or DEFAULT_REPO


def cli(
    args: Sequence[str], stdin: str | None = None, *, retries: int = 0, required: bool = True
) -> str:
    """Output of `gh <args>`. A failure is retried `retries` times with a growing pause; then it
    stops the script with the message of `gh` when `required`, or gives an empty string."""
    for attempt in range(retries + 1):
        out = subprocess.run(
            ["gh", *args],
            input=stdin,
            capture_output=True,
            text=True,
            encoding="utf-8",
            check=False,
        )
        if out.returncode == 0:
            return out.stdout
        if attempt < retries:
            time.sleep(2 * (attempt + 1))
    if required:
        sys.exit(f"gh {' '.join(args[:2])} failed: {out.stderr.strip()}")
    return ""


def api(
    path: str,
    *,
    method: str = "GET",
    payload: dict[str, Any] | None = None,
    retries: int = 0,
    required: bool = True,
) -> Any:
    """Decoded JSON of one REST call (`gh api`), or None for an empty answer or a failure that
    is not `required`."""
    args = ["api", path] if method == "GET" else ["api", path, "-X", method]
    if payload is not None:
        args += ["--input", "-"]
    text = cli(
        args,
        json.dumps(payload) if payload is not None else None,
        retries=retries,
        required=required,
    )
    return json.loads(text) if text.strip() else None


def pages(path: str, fetch: Callable[[str], Any] = api, per_page: int = PER_PAGE) -> list[Any]:
    """Every item of a paginated list endpoint. Explicit page numbers: `--paginate` follows links
    of the form repositories/<id>/..., which some proxies refuse."""
    sep = "&" if "?" in path else "?"
    items: list[Any] = []
    for page in range(1, MAX_PAGES + 1):
        batch = fetch(f"{path}{sep}per_page={per_page}&page={page}") or []
        if not isinstance(batch, list):
            sys.exit(f"{path}: page {page} is not a list: {str(batch)[:200]}")
        items.extend(batch)
        if len(batch) < per_page:
            return items
    sys.exit(f"{path}: more than {MAX_PAGES} pages of {per_page} items; narrow the request.")
