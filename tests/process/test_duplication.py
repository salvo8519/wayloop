"""Duplicated code: the settings of jscpd and the baseline of the pure crates (`just dup`)."""

from __future__ import annotations

import json

from conftest import ROOT

# The clones of the pure crates accepted until #299 deals with them: the clippy.toml of each
# pure crate, the test generator of envelope.rs and sha256.rs, and three in
# wayloop-schemas/src/ident.rs (the matcher of closed_enum and contract_enum, and the serde and
# JsonSchema impls of two macros), which #299 removes or keeps as deliberate. The baseline may
# only shrink.
KNOWN_CLONES = 5


def test_the_baseline_of_the_pure_crates_never_grows() -> None:
    baseline = json.loads((ROOT / ".jscpd-baseline.json").read_text(encoding="utf-8"))
    assert len(baseline["fingerprints"]) <= KNOWN_CLONES


def test_jscpd_is_pinned_to_an_exact_version() -> None:
    package = json.loads((ROOT / "package.json").read_text(encoding="utf-8"))
    version = package["devDependencies"]["jscpd"]
    assert version[0].isdigit(), version
    lock = json.loads((ROOT / "package-lock.json").read_text(encoding="utf-8"))
    assert lock["packages"]["node_modules/jscpd"]["version"] == version


def test_jscpd_reads_large_files_and_skips_generated_ones() -> None:
    settings = json.loads((ROOT / ".jscpd.json").read_text(encoding="utf-8"))
    assert settings["minLines"] == 8 and settings["minTokens"] == 60
    assert settings["maxSize"] == "20mb"
    for folder in ("schemas", "tests/contract", "docs/it", ".claude/agents", ".claude/skills"):
        assert f"{folder}/**" in settings["ignore"], folder
