"""scripts/journal.py: index and search of the session hand-overs."""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location("journal", ROOT / "scripts/journal.py")
assert _spec and _spec.loader
jr = importlib.util.module_from_spec(_spec)
sys.modules["journal"] = jr  # dataclasses resolve their module by name
_spec.loader.exec_module(jr)

ROADMAP = """labels: []
milestones:
  - title: "M0 · Foundations"
    gate: "G."
    release: "0.0.1"
    epics:
      - title: "Agent process"
        issue: 19
        labels: ["area:agents"]
        tasks:
          - issue: 8
            title: "Session skills"
            status: open
      - title: "Ledger"
        issue: 22
        labels: ["area:core"]
        tasks:
          - issue: 12
            title: "Ledger spec"
            status: open
"""


@pytest.fixture
def journal(tmp_path: Path) -> tuple[Path, Path]:
    folder = tmp_path / "journal"
    folder.mkdir()
    (folder / "2026-11-17-feat-8-session-skills.md").write_text(
        "# 2026-11-17 feat/8-session-skills\n\n- Issue: #8 Session skills\n\n## Done\n"
        "- skills written (abc123)\n\n## Next steps\n1. Wire the ledger of #12.\n",
        encoding="utf-8",
    )
    (folder / "2026-11-16-feat-12-ledger.md").write_text(
        "# 2026-11-16 feat/12-ledger\n\n- Issue: #12 Ledger spec\n\n## Done\n- spec drafted\n",
        encoding="utf-8",
    )
    roadmap = tmp_path / "roadmap.yaml"
    roadmap.write_text(ROADMAP, encoding="utf-8")
    return folder, roadmap


def test_index_reads_issue_mentions_areas_and_summary(journal: tuple[Path, Path]) -> None:
    entries = {e.file: e for e in jr.build(*journal)}
    entry = entries["2026-11-17-feat-8-session-skills.md"]
    assert entry.issue == 8 and 12 in entry.mentions
    assert entry.areas == ["agents"]
    assert entry.summary.startswith("skills written")


def test_search_by_issue_lists_own_entries_then_mentions(journal: tuple[Path, Path]) -> None:
    files = [e.file for e, _ in jr.search(jr.build(*journal), "12")]
    assert files == ["2026-11-16-feat-12-ledger.md", "2026-11-17-feat-8-session-skills.md"]


def test_search_by_area_and_date(journal: tuple[Path, Path]) -> None:
    entries = jr.build(*journal)
    assert [e.file for e, _ in jr.search(entries, "area:core")] == ["2026-11-16-feat-12-ledger.md"]
    assert [e.file for e, _ in jr.search(entries, "2026-11-17")] == [
        "2026-11-17-feat-8-session-skills.md"
    ]


def test_template_has_the_hand_over_sections() -> None:
    text = (ROOT / ".agents" / "journal" / "_template.md").read_text(encoding="utf-8")
    for section in ("## Checkout", "## Done", "## Not done", "## Learnings", "## Next steps"):
        assert section in text
