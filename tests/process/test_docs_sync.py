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
        re.sub(
            r"\| Status \| \w+ \|",
            "| Status | Done-ish |",
            spec.read_text(encoding="utf-8"),
            count=1,
        )
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
        (["crates/wayloop-runtime/src/lib.rs"], ["engine changed"]),
        (["crates/wayloop-schemas/src/lib.rs"], ["engine changed"]),
        (["crates/wayloop-runtime/src/lib.rs", "docs/en/concepts/architecture.md"], []),
        # Every crate but the command line is engine, including crates added later.
        (["crates/wayloop-mcp/src/lib.rs"], ["engine changed"]),
        (["crates/wayloop-cli/src/main.rs"], ["command line changed"]),
        (["crates/wayloop-core/Cargo.toml"], []),
        # The guards of the pure crates are described on the architecture page.
        (["scripts/check_core_io.py"], ["purity guard changed"]),
        (["crates/wayloop-core/clippy.toml"], ["purity guard changed"]),
        (["crates/wayloop-schemas/clippy.toml"], ["purity guard changed"]),
        (["crates/wayloop-core/clippy.toml", "docs/en/concepts/architecture.md"], []),
        (["scripts/check_core_io.py", "docs/en/concepts/index.md"], ["purity guard changed"]),
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


@pytest.fixture
def public(tmp_path: Path) -> Path:
    """A minimal repository with one public page and one project page per language."""
    for lang in ("en", "it"):
        (tmp_path / "docs" / lang / "concepts").mkdir(parents=True)
        (tmp_path / "docs" / lang / "project").mkdir(parents=True)
        (tmp_path / "docs" / lang / "concepts" / "page.md").write_text(
            "# Page\n\nThe engine is a Rust library.\n", encoding="utf-8"
        )
    (tmp_path / "README.md").write_text("# Readme\n", encoding="utf-8")
    return tmp_path


def test_the_committed_public_pages_state_no_plans() -> None:
    assert ds.claims_problems() == []


@pytest.mark.parametrize(
    "line",
    [
        "The MCP server will be available next month.",
        "A Python SDK is planned.",
        "Support for GitLab is coming soon.",
        "Plugins are on the roadmap.",
        "The adapter ships in M3.",
        "Il server MCP sarà disponibile a breve.",
        "## Architettura prevista",
    ],
)
def test_a_plan_in_prose_is_reported(public: Path, line: str) -> None:
    page = public / "docs/en/concepts/page.md"
    page.write_text(page.read_text(encoding="utf-8") + line + "\n", encoding="utf-8")
    assert ds.claims_problems(public) == [f"docs/en/concepts/page.md:4: {line}"]


@pytest.mark.parametrize(
    "line",
    [
        "What comes next is in the [roadmap](../project/roadmap.md).",
        "[![SDK](https://img.shields.io/badge/SDK-planned-blue)](https://example.org/roadmap/)",
        "Run `wayloop steps --will` to list the steps.",
        "<!-- planned: nothing -->",
        "A proposed change to the roadmap is a readable difference.",
    ],
)
def test_links_code_and_comments_are_not_prose(public: Path, line: str) -> None:
    (public / "README.md").write_text(f"# Readme\n\n{line}\n", encoding="utf-8")
    assert ds.claims_problems(public) == []


def test_a_fenced_block_is_not_prose(public: Path) -> None:
    (public / "README.md").write_text(
        "# Readme\n\n```text\nthis will not be checked\n```\n", encoding="utf-8"
    )
    assert ds.claims_problems(public) == []


def test_project_pages_may_state_plans(public: Path) -> None:
    (public / "docs/en/project/roadmap.md").write_text(
        "# Roadmap\n\nThe SDK will follow.\n", encoding="utf-8"
    )
    assert ds.claims_problems(public) == []


def test_italian_pages_and_translated_readmes_are_public(public: Path) -> None:
    (public / "README.it.md").write_text("# Leggimi\n\nIl motore sarà pronto.\n", encoding="utf-8")
    (public / "docs/it/concepts/page.md").write_text(
        "# Pagina\n\nLe guide verranno pubblicate.\n", encoding="utf-8"
    )
    problems = ds.claims_problems(public)
    assert "README.it.md:3: Il motore sarà pronto." in problems
    assert "docs/it/concepts/page.md:3: Le guide verranno pubblicate." in problems


def test_message_trailer_waives_the_staged_check(tmp_path: Path) -> None:
    message = tmp_path / "COMMIT_EDITMSG"
    message.write_text("fix(scripts): plain errors\n\nDocs-impact: none\n", encoding="utf-8")
    assert ds.waived_by_message(message)


def test_message_without_trailer_does_not_waive(tmp_path: Path) -> None:
    message = tmp_path / "COMMIT_EDITMSG"
    message.write_text("fix(scripts): plain errors\n\nRefs: #47\n", encoding="utf-8")
    assert not ds.waived_by_message(message)


def test_trailer_must_be_in_the_last_paragraph(tmp_path: Path) -> None:
    message = tmp_path / "COMMIT_EDITMSG"
    message.write_text(
        "fix(scripts): plain errors\n\nDocs-impact: none\n\nSigned-off-by: A <a@b.c>\n",
        encoding="utf-8",
    )
    assert not ds.waived_by_message(message)


def branch(monkeypatch: pytest.MonkeyPatch, trailers: dict[str, list[str]]) -> None:
    """Stubs git: the branch has the commits of `trailers`, each with its trailer values."""

    def fake_git(*args: str) -> list[str]:
        if args[0] == "rev-list":
            return list(trailers)
        if args[0] == "log":
            return trailers[args[-1]]
        raise AssertionError(args)

    monkeypatch.setattr(ds, "git", fake_git)


def test_a_waiver_on_every_commit_waives_the_branch(monkeypatch: pytest.MonkeyPatch) -> None:
    branch(monkeypatch, {"b2": ["none"], "a1": ["None"]})
    assert ds.waived_by_branch("origin/main")


def test_a_waiver_on_the_last_commit_only_does_not_waive_the_branch(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # The latest commit touches only tests and carries the waiver; an earlier one changed the
    # engine without documentation: the whole diff is still checked.
    branch(monkeypatch, {"b2": ["none"], "a1": []})
    assert not ds.waived_by_branch("origin/main")


def test_an_empty_branch_is_not_waived(monkeypatch: pytest.MonkeyPatch) -> None:
    branch(monkeypatch, {})
    assert not ds.waived_by_branch("origin/main")
