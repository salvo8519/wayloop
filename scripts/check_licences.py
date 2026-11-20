"""Every licence that `cargo deny` allows is also allowed by Dependency review.

`cargo deny` (deny.toml) checks the licences of the normal Rust dependencies; Dependency review
(the Security workflow) checks every dependency a pull request adds, development dependencies
and Python packages included. Its list may hold more licences (Python packages), never fewer:
a Rust licence accepted locally must not be refused on the pull request. A licence with an
exception (`Apache-2.0 WITH LLVM-exception`) counts by its base identifier.

    python scripts/check_licences.py
"""

from __future__ import annotations

import tomllib
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[1]
DENY = "deny.toml"
WORKFLOW = ".github/workflows/security.yml"
ACTION = "actions/dependency-review-action"


def deny_licences(root: Path) -> set[str]:
    with (root / DENY).open("rb") as handle:
        allowed = tomllib.load(handle)["licenses"]["allow"]
    return {entry.split(" WITH ")[0].strip() for entry in allowed}


def review_licences(root: Path) -> set[str]:
    workflow = yaml.safe_load((root / WORKFLOW).read_text(encoding="utf-8"))
    for job in workflow["jobs"].values():
        for step in job.get("steps", []):
            if str(step.get("uses", "")).startswith(ACTION):
                text = str(step.get("with", {}).get("allow-licenses", ""))
                return {entry.strip() for entry in text.split(",") if entry.strip()}
    message = f"{WORKFLOW}: no {ACTION} step with allow-licenses"
    raise SystemExit(message)


def main() -> None:
    missing = sorted(deny_licences(ROOT) - review_licences(ROOT))
    if missing:
        message = (
            f"Licences allowed in {DENY} but refused by Dependency review in {WORKFLOW}: "
            + ", ".join(missing)
        )
        raise SystemExit(message)
    print("Licence allow-lists agree.")


if __name__ == "__main__":
    main()
