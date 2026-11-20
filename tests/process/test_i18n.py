"""Translations of the root documents are tracked like the documentation pages."""

from __future__ import annotations

from pathlib import Path

import pytest

import i18n


def test_root_documents_are_required_translations() -> None:
    names = {name: (dst.name, required) for name, _src, dst, required in i18n.pairs("it", {})}
    assert names["README.md"] == ("README.it.md", True)


def test_stamp_tracks_a_root_document(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    (tmp_path / "README.md").write_text("# Wayloop\n", encoding="utf-8")
    translation = tmp_path / "README.it.md"
    translation.write_text("# Wayloop (italiano)\n", encoding="utf-8")
    monkeypatch.setattr(i18n, "ROOT", tmp_path)
    i18n.stamp(translation)
    meta = i18n.meta(translation)
    assert meta["source"] == "README.md"
    assert meta["source_hash"] == i18n.digest(tmp_path / "README.md")


def test_stamp_refuses_an_english_source(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    source = tmp_path / "README.md"
    source.write_text("# Wayloop\n", encoding="utf-8")
    monkeypatch.setattr(i18n, "ROOT", tmp_path)
    with pytest.raises(SystemExit):
        i18n.stamp(source)
    assert source.read_text(encoding="utf-8") == "# Wayloop\n"
