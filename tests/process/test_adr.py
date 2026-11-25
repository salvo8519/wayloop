"""scripts/_adr.py: the rule of the file names of ADRs, equal to the one of the engine."""

from __future__ import annotations

import json
import re
from typing import Any

import _adr
from conftest import ROOT

# The cases shared with the test of the runtime that reads them through `locations::adr_numbers`.
CASES: dict[str, Any] = json.loads(
    (ROOT / "tests" / "process" / "adr_names.json").read_text(encoding="utf-8")
)


def test_the_folder_is_the_one_of_the_engine() -> None:
    assert CASES["folder"] == _adr.FOLDER


def test_each_shared_case_names_the_number_of_the_engine() -> None:
    for case in CASES["names"]:
        assert _adr.adr_number(case["name"]) == case["number"], case["name"]
    expected = sorted({c["number"] for c in CASES["names"] if c["number"] is not None})
    assert _adr.adr_numbers(c["name"] for c in CASES["names"]) == expected


def test_the_adrs_of_the_repository_are_the_rows_of_their_index() -> None:
    folder = ROOT / _adr.FOLDER
    numbers = _adr.adr_numbers(p.name for p in folder.iterdir() if p.is_file())
    index = (folder / _adr.INDEX).read_text(encoding="utf-8")
    rows = sorted(int(n) for n in re.findall(rf"^\| \[({_adr.NUMBER})\]\(", index, re.MULTILINE))
    assert numbers == rows
    assert len(rows) == len(set(rows))
