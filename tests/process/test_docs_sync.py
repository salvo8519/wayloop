"""scripts/docs_sync.py: ADRs, specifications and navigation agree; changes bring their docs."""

from __future__ import annotations

import importlib.util
import re
import shutil
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location("docs_sync", ROOT / "scripts/docs_sync.py")
assert _spec and _spec.loader
ds = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(ds)


@pytest.fixture
def site(tmp_path: Path) -> Path:
    """A copy of the documentation, the navigation and the specifications."""
    shutil.copytree(ROOT / "docs" / "en", tmp_path / "docs" / "en")
    shutil.copytree(ROOT / "specs", tmp_path / "specs")
    shutil.copy(ROOT / "zensical.toml", tmp_path / "zensical.toml")
    return tmp_path


def adr(site: Path) -> Path:
    return site / "docs/en/project/adr/0001-rust-engine-thin-adapters.md"


def test_the_committed_documentation_is_covered() -> None:
    assert ds.coverage_problems() == []


def test_an_adr_without_index_row_or_navigation_entry_is_reported(site: Path) -> None:
    new = site / "docs/en/project/adr/0099-example.md"
    new.write_text("# ADR 0099: Example\n\nStatus: Proposed · 2026-11-17\n", encoding="utf-8")
    problems = ds.coverage_problems(site)
    assert "ADR 0099: no row in docs/en/project/adr/index.md" in problems
    assert any("0099-example.md is not in the navigation" in p for p in problems)


def test_an_adr_needs_a_valid_status(site: Path) -> None:
    text = adr(site).read_text(encoding="utf-8")
    adr(site).write_text(
        re.sub(r"^Status: .*$", "Status: maybe", text, flags=re.MULTILINE), encoding="utf-8"
    )
    assert "ADR 0001: no valid 'Status:' line" in ds.coverage_problems(site)


def test_an_index_row_without_its_file_is_reported(site: Path) -> None:
    adr(site).unlink()
    problems = ds.coverage_problems(site)
    assert any(
        "links 0001-rust-engine-thin-adapters.md, which does not exist" in p for p in problems
    )
    assert any("navigation lists project/adr/0001" in p for p in problems)


def test_a_specification_needs_a_valid_status(site: Path) -> None:
    spec = next((site / "specs").glob("0001-*/spec.md"))
    spec.write_text(
        spec.read_text(encoding="utf-8").replace("| Status | Draft |", "| Status | Done-ish |")
    )
    assert any("0001-" in p and "Status" in p for p in ds.coverage_problems(site))


def test_a_page_outside_the_navigation_is_reported(site: Path) -> None:
    (site / "docs/en/concepts/orphan.md").write_text("# Orphan\n", encoding="utf-8")
    assert (
        "docs/en/concepts/orphan.md is not in the navigation of zensical.toml"
        in ds.coverage_problems(site)
    )


@pytest.mark.parametrize(
    ("files", "expected"),
    [
        (["crates/wayloop-core/src/lib.rs"], ["engine changed"]),
        (["crates/wayloop-core/src/lib.rs", "specs/0002-state-machine/spec.md"], []),
        (["crates/wayloop-cli/src/main.rs", "docs/en/reference/commands.md"], []),
        (["schemas/commands/next/v1.json"], ["schemas changed"]),
        (["scripts/roadmap.py"], ["process script changed"]),
        (["scripts/roadmap.py", "docs/en/project/conventions.md"], []),
        (["README.md", "scripts/brand.py"], []),
    ],
)
def test_impact_rules(files: list[str], expected: list[str]) -> None:
    missing = ds.missing_docs(files)
    assert len(missing) == len(expected)
    assert all(m.startswith(e) for m, e in zip(missing, expected, strict=True))
