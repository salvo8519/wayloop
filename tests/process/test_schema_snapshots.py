"""scripts/schema_snapshots.py: snapshots generated from the types, history kept per contract
(ADR 0030 G5, G6; ADR 0009 C5; SPEC-0001 R1, R16, AC3)."""

from __future__ import annotations

import json
import re
import subprocess
from pathlib import Path
from typing import Any

import protect_contracts as hook
import pytest
import schema_snapshots as snapshots
from conftest import ROOT, git
from schema_snapshots import Contract, File

ENVELOPE_TEXT = '{\n  "description": "è vero.",\n  "type": "object"\n}\n'


def contract(
    version: int = 1,
    text: str = ENVELOPE_TEXT,
    area: str = "envelope",
    name: str = "result",
    parts: tuple[tuple[str, str], ...] = (("schema", "output"),),
) -> Contract:
    """A contract of the registry as wayloop-schemagen prints it."""
    files = tuple(
        File(
            part=part,
            direction=direction,
            path=f"{area}/{name}/v{version}{'' if part == 'schema' else '.' + part}.json",
            text=text,
        )
        for part, direction in parts
    )
    return Contract(area=area, name=name, version=version, files=files)


def schema(**members: dict[str, Any]) -> str:
    """The text of an object schema with `members`, every member required."""
    return json.dumps(
        {"type": "object", "properties": members, "required": sorted(members)}, indent=2
    )


def write(root: Path, *contracts: Contract) -> None:
    assert snapshots.write(root, list(contracts)) == []


def read(root: Path, path: str) -> str:
    return (root / path).read_text(encoding="utf-8")


def manifest(root: Path, folder: str = "envelope/result") -> dict[str, Any]:
    found = snapshots.read_manifest(root, folder)
    assert found is not None
    return found


# ---- generate and check


def test_generate_writes_the_text_unchanged_its_history_and_its_embedded_copy(
    tmp_path: Path,
) -> None:
    text = '{"odd":   [1,2],\n"è": true}\n'
    write(tmp_path, contract(text=text))
    path = "schemas/envelope/result/v1.json"
    assert (tmp_path / path).read_bytes() == text.encode("utf-8"), "never reformatted"
    assert (tmp_path / snapshots.COPIES / "envelope/result/v1.json").read_bytes() == (
        tmp_path / path
    ).read_bytes()
    assert manifest(tmp_path) == {
        "current": "1",
        "format": 1,
        "versions": {
            "1": {
                "file": "envelope/result/v1.json",
                "introduced_in": "unreleased",
                "sha256": snapshots.sha256(text.encode("utf-8")),
            }
        },
    }
    rust = read(tmp_path, snapshots.EMBEDDED_RS)
    assert 'include_str!("../../snapshots/envelope/result/v1.json")' in rust
    assert snapshots.check(tmp_path, [contract(text=text)]) == []


def test_a_second_run_changes_no_byte(tmp_path: Path) -> None:
    contracts = [
        contract(),
        contract(area="commands", name="next", parts=(("schema", "input"), ("output", "output"))),
    ]
    write(tmp_path, *contracts)
    files = sorted(p for p in tmp_path.rglob("*") if p.is_file())
    before = {p: (p.read_bytes(), p.stat().st_mtime_ns) for p in files}
    write(tmp_path, *contracts)
    assert sorted(p for p in tmp_path.rglob("*") if p.is_file()) == files
    assert {p: (p.read_bytes(), p.stat().st_mtime_ns) for p in files} == before
    record = manifest(tmp_path, "commands/next")["versions"]["1"]
    assert record["output_file"] == "commands/next/v1.output.json"
    assert set(record) == {"file", "sha256", "output_file", "output_sha256", "introduced_in"}


def test_the_versions_of_a_history_are_in_numeric_order() -> None:
    versions = {str(v): {"file": f"a/b/v{v}.json"} for v in (10, 9, 1)}
    text = snapshots.manifest_text({"versions": versions, "format": 1, "current": "10"})
    assert re.findall(r'"(\d+)": \{', text) == ["1", "9", "10"]


@pytest.mark.parametrize(
    ("change", "problem"),
    [
        (
            lambda root: (root / "schemas/envelope/result/v1.json").write_text("{}\n"),
            "schemas/envelope/result/v1.json: differs from the generated file",
        ),
        (
            lambda root: (root / "schemas/envelope/result/v1.json").unlink(),
            "schemas/envelope/result/v1.json: missing",
        ),
        (
            lambda root: (root / snapshots.COPIES / "envelope/result/v1.json").write_text("{}\n"),
            f"{snapshots.COPIES}/envelope/result/v1.json: differs from the generated file",
        ),
        (
            lambda root: (root / snapshots.EMBEDDED_RS).write_text("//! edited\n"),
            f"{snapshots.EMBEDDED_RS}: differs from the generated file",
        ),
        (
            lambda root: (root / "schemas/envelope/result/v9.json").write_text("{}\n"),
            "schemas/envelope/result/v9.json: neither generated nor recorded in a history",
        ),
        (
            lambda root: (root / snapshots.COPIES / "envelope/result/v9.json").write_text("{}\n"),
            f"{snapshots.COPIES}/envelope/result/v9.json: a copy of no snapshot",
        ),
    ],
)
def test_check_fails_when_a_committed_file_differs(
    tmp_path: Path, change: Any, problem: str
) -> None:
    write(tmp_path, contract())
    change(tmp_path)
    assert problem in snapshots.check(tmp_path, [contract()])


def test_check_fails_on_a_folder_the_registry_does_not_list(tmp_path: Path) -> None:
    write(tmp_path, contract(), contract(area="artifacts", name="brief"))
    found = snapshots.check(tmp_path, [contract()])
    assert (
        "schemas/artifacts/brief/index.json: not in a folder of a contract of the registry" in found
    )
    assert "schemas/artifacts/brief/v1.json: not in a folder of a contract of the registry" in found


def test_an_earlier_version_stays_and_is_embedded_after_a_bump(tmp_path: Path) -> None:
    write(tmp_path, contract())
    write(tmp_path, contract(version=2, text="{}\n"))
    assert read(tmp_path, "schemas/envelope/result/v1.json") == ENVELOPE_TEXT
    history = manifest(tmp_path)
    assert history["current"] == "2"
    assert sorted(history["versions"]) == ["1", "2"]
    assert read(tmp_path, f"{snapshots.COPIES}/envelope/result/v1.json") == ENVELOPE_TEXT
    rust = read(tmp_path, snapshots.EMBEDDED_RS)
    assert rust.index("v1.json") < rust.index("v2.json")
    assert snapshots.check(tmp_path, [contract(version=2, text="{}\n")]) == []


def test_check_fails_when_an_earlier_version_changed_or_its_history_has_a_gap(
    tmp_path: Path,
) -> None:
    write(tmp_path, contract())
    write(tmp_path, contract(version=2))
    (tmp_path / "schemas/envelope/result/v1.json").write_text("{}\n")
    found = snapshots.check(tmp_path, [contract(version=2)])
    assert "schemas/envelope/result/v1.json: its hash differs from its history" in found
    history = manifest(tmp_path)
    del history["versions"]["1"]
    (tmp_path / "schemas/envelope/result/index.json").write_text(json.dumps(history))
    found = snapshots.check(tmp_path, [contract(version=2)])
    assert "schemas/envelope/result/index.json: versions 2 do not run from 1 to 2" in found


def test_a_recorded_snapshot_replaced_by_a_symlink_fails_the_check(tmp_path: Path) -> None:
    write(tmp_path, contract())
    write(tmp_path, contract(version=2))
    path = tmp_path / "schemas/envelope/result/v1.json"
    (tmp_path / "copy.json").write_bytes(path.read_bytes())
    path.unlink()
    path.symlink_to(tmp_path / "copy.json")
    found = snapshots.check(tmp_path, [contract(version=2)])
    assert "schemas/envelope/result/v1.json: recorded in its history, not a regular file" in found


@pytest.mark.parametrize(
    ("version", "problem"),
    [
        (1, "envelope/result: the current version would go back from v2 to v1"),
        (4, "envelope/result: v4 would follow v2, leaving a gap"),
    ],
)
def test_generate_refuses_to_rewrite_an_earlier_version_or_leave_a_gap(
    tmp_path: Path, version: int, problem: str
) -> None:
    write(tmp_path, contract())
    write(tmp_path, contract(version=2))
    before = {p: p.read_bytes() for p in tmp_path.rglob("*") if p.is_file()}
    refused = snapshots.write(tmp_path, [contract(version=version, text="[]\n")])
    assert any(r.startswith(problem) for r in refused), refused
    assert {p: p.read_bytes() for p in tmp_path.rglob("*") if p.is_file()} == before


def test_a_new_contract_starts_at_version_one(tmp_path: Path) -> None:
    refused = snapshots.write(tmp_path, [contract(version=2)])
    assert refused == ["envelope/result: v2 would follow v0, leaving a gap"]


def test_generate_removes_copies_of_no_snapshot_and_reports_strays(tmp_path: Path) -> None:
    write(tmp_path, contract())
    stale = tmp_path / snapshots.COPIES / "old/x/v1.json"
    stale.parent.mkdir(parents=True)
    stale.write_text("{}\n")
    stray = tmp_path / "schemas/envelope/result/notes.txt"
    stray.write_text("x\n")
    found = snapshots.write(tmp_path, [contract()])
    assert not stale.exists()
    assert found == [
        "schemas/envelope/result/notes.txt: neither generated nor recorded in a history"
    ]


def test_a_history_listing_a_missing_file_is_refused(tmp_path: Path) -> None:
    write(tmp_path, contract())
    write(tmp_path, contract(version=2))
    (tmp_path / "schemas/envelope/result/v1.json").unlink()
    refused = snapshots.write(tmp_path, [contract(version=2)])
    assert refused == ["schemas/envelope/result/v1.json: recorded in its history and missing"]


def test_the_embedded_list_of_no_snapshot_is_empty_and_formatted() -> None:
    text = snapshots.embedded_rs([])
    assert text.endswith("pub const EMBEDDED: &[Embedded] = &[];\n")
    assert text.startswith("//! Generated by `scripts/schema_snapshots.py`")


def test_the_embedded_list_of_the_repository_is_generated() -> None:
    paths = re.findall(r'path: "([^"]+)"', read(ROOT, snapshots.EMBEDDED_RS))
    assert paths and paths == sorted(paths)
    assert read(ROOT, snapshots.EMBEDDED_RS) == snapshots.embedded_rs(paths)


def test_no_crate_embeds_a_file_outside_its_folder() -> None:
    # `cargo package` packs only the folder of a crate (ADR 0030, amendment of 2026-11-20).
    for source in sorted((ROOT / "crates").glob("*/**/*.rs")):
        crate = source.relative_to(ROOT / "crates").parts[0]
        for target in re.findall(r'include_(?:str|bytes)!\(\s*"([^"]+)"', read(ROOT, str(source))):
            resolved = (source.parent / target).resolve()
            assert resolved.is_relative_to(ROOT / "crates" / crate), f"{source}: {target}"
            assert resolved.is_file(), f"{source}: {target}"


# ---- the history assembled


def test_schema_history_assembles_every_manifest(tmp_path: Path) -> None:
    write(tmp_path, contract(), contract(area="commands", name="next"))
    history = snapshots.schema_history(tmp_path)
    assert history["format"] == 1
    assert sorted(history["contracts"]) == ["commands/next", "envelope/result"]
    assert history["contracts"]["envelope/result"]["current"] == "1"
    assert "format" not in history["contracts"]["envelope/result"]
    assert snapshots.schema_history(tmp_path / "nowhere") == {"contracts": {}, "format": 1}


# ---- the classifier of the current version


def classify(old: dict[str, Any], new: dict[str, Any], output: bool) -> list[str]:
    found: list[str] = snapshots.breaking("", old, new, output)
    return found


OBJECT: dict[str, Any] = {
    "type": "object",
    "properties": {"a": {"type": "string"}},
    "required": ["a"],
}


@pytest.mark.parametrize("output", [True, False])
def test_documentation_and_a_new_definition_are_additive(output: bool) -> None:
    new = {**OBJECT, "description": "now documented", "$defs": {"X": {"type": "string"}}}
    new["properties"] = {"a": {"type": "string", "description": "the a", "title": "A"}}
    assert classify(OBJECT, new, output) == []


def test_a_new_output_member_is_additive_even_when_required() -> None:
    new = {**OBJECT, "properties": {**OBJECT["properties"], "b": {"type": "integer"}}}
    assert classify(OBJECT, {**new, "required": ["a", "b"]}, output=True) == []


def test_a_new_optional_input_member_is_additive_a_required_one_is_breaking() -> None:
    new = {**OBJECT, "properties": {**OBJECT["properties"], "b": {"type": "integer"}}}
    assert classify(OBJECT, new, output=False) == []
    assert classify(OBJECT, {**new, "required": ["a", "b"]}, output=False) == [
        "/properties/b: now required"
    ]


@pytest.mark.parametrize("output", [True, False])
def test_a_removed_or_retyped_member_is_breaking(output: bool) -> None:
    assert classify(OBJECT, {**OBJECT, "properties": {}, "required": []}, output) == [
        "/properties/a: member removed"
    ]
    retyped = {**OBJECT, "properties": {"a": {"type": "integer"}}}
    assert classify(OBJECT, retyped, output) == ["/properties/a: `type` changed"]


def test_an_output_member_no_longer_required_is_breaking() -> None:
    assert classify(OBJECT, {**OBJECT, "required": []}, output=True) == [
        "/properties/a: may now be absent"
    ]
    assert classify(OBJECT, {**OBJECT, "required": []}, output=False) == []


@pytest.mark.parametrize("output", [True, False])
def test_a_new_enum_value_is_additive_a_removed_one_breaking(output: bool) -> None:
    old = {"type": "string", "enum": ["a", "b"]}
    assert classify(old, {**old, "enum": ["a", "b", "c"]}, output) == []
    assert classify(old, {**old, "enum": ["a"]}, output) == ['/: value "b" removed']
    assert classify(old, {"type": "string"}, output) == ["/: `enum` changed"]


def test_a_change_of_draft_is_breaking() -> None:
    old = {**OBJECT, "$schema": "https://json-schema.org/draft/2020-12/schema"}
    new = {**OBJECT, "$schema": "https://json-schema.org/draft/2019-09/schema"}
    assert classify(old, new, output=True) == ["/: `$schema` changed"]


def test_closing_an_open_output_is_breaking() -> None:
    assert classify(OBJECT, {**OBJECT, "additionalProperties": False}, output=True) == [
        "/: `additionalProperties` changed"
    ]


def test_items_definitions_and_conditions_are_compared() -> None:
    old = {
        "type": "array",
        "items": {"$ref": "#/$defs/A"},
        "$defs": {"A": OBJECT, "B": {"type": "string"}},
        "allOf": [{"if": {"required": ["a"]}, "then": {"required": ["b"]}}],
    }
    new = {
        "type": "array",
        "items": {"$ref": "#/$defs/C"},
        "$defs": {"A": {**OBJECT, "properties": {}, "required": []}},
        "allOf": [],
    }
    assert classify(old, new, output=True) == [
        "/items: `$ref` changed",
        "/$defs/A/properties/a: member removed",
        "/$defs/B: definition removed",
        "/: `allOf` changed",
    ]
    assert classify({"items": {}}, {}, output=True) == ["/items: changed"]


# ---- the history against a base


def base_branch(root: Path, introduced_in: str | None, tag: str | None) -> Path:
    """`main` holds the envelope at v1, stamped `introduced_in` when given and tagged `tag`
    when given; the work happens on a branch."""
    (root / "README.md").write_text("x\n")
    write(root, contract(text=schema(a={"type": "string"})))
    if introduced_in is not None:
        edit_history(root, lambda h: h["versions"]["1"].update(introduced_in=introduced_in))
    git(root, "add", ".")
    git(root, "commit", "-q", "-m", "base")
    if tag is not None:
        git(root, "tag", tag)
    git(root, "checkout", "-q", "-b", "feat/1-x")
    return root


@pytest.fixture
def repo(git_repo: Path) -> Path:
    """A repository whose `main` holds the envelope at v1, released as 0.1.0."""
    return base_branch(git_repo, "0.1.0", "v0.1.0")


@pytest.fixture
def draft(git_repo: Path) -> Path:
    """A repository whose `main` holds the envelope at v1, not yet released."""
    return base_branch(git_repo, None, None)


def diff(root: Path, *contracts: Contract, base: str = "main") -> snapshots.Diff:
    return snapshots.diff_against(root, base, list(contracts))


def test_an_unchanged_history_passes(repo: Path) -> None:
    assert diff(repo, contract(text=schema(a={"type": "string"}))) == ([], [], [], [])


def test_an_additive_change_of_the_current_version_passes(repo: Path) -> None:
    current = contract(text=schema(a={"type": "string"}, b={"type": "integer"}))
    write(repo, current)
    assert diff(repo, current) == (
        [],
        [],
        [],
        ["schemas/envelope/result/v1.json: changed, additively"],
    )


def test_a_breaking_change_of_the_current_version_is_classified(repo: Path) -> None:
    current = contract(text=schema(a={"type": "integer"}))
    write(repo, current)
    assert diff(repo, current) == (
        [],
        ["schemas/envelope/result/v1.json#/properties/a: `type` changed"],
        [],
        [],
    )
    git(repo, "commit", "-q", "-am", "breaking")
    assert diff(repo, current)[1] != [], "committed or not, the base decides"


def test_a_breaking_change_as_a_new_version_passes(repo: Path) -> None:
    following = contract(version=2, text=schema(a={"type": "integer"}))
    write(repo, following)
    added = ["schemas/envelope/result/v2.json: added"]
    assert diff(repo, following) == ([], [], [], added)
    write(repo, contract(version=2, text="{}\n"))
    assert diff(repo, contract(version=2, text="{}\n")) == ([], [], [], added), "not on main yet"


def test_an_earlier_version_changed_is_a_rewritten_history(repo: Path) -> None:
    following = contract(version=2, text="{}\n")
    write(repo, following)
    (repo / "schemas/envelope/result/v1.json").write_text("{}\n")
    rewritten = diff(repo, following).rewritten
    assert rewritten == [
        "schemas/envelope/result/v1.json: released on main as an earlier version, and changed"
    ]


@pytest.mark.parametrize(
    ("change", "problem"),
    [
        (
            lambda root: (root / "schemas/envelope/result/v1.json").unlink(),
            "schemas/envelope/result/v1.json: released on main and deleted",
        ),
        (
            lambda root: (root / "schemas/envelope/result/v1.json").chmod(0o755),
            "schemas/envelope/result/v1.json: released on main and its mode changed "
            "(100644 to 100755)",
        ),
        (
            lambda root: (root / "schemas/envelope/result/index.json").unlink(),
            "envelope/result: the contract was removed",
        ),
    ],
)
def test_a_released_file_deleted_or_replaced_is_a_rewritten_history(
    repo: Path, change: Any, problem: str
) -> None:
    change(repo)
    assert problem in diff(repo, contract(text=schema(a={"type": "string"})))[0]


def test_a_released_snapshot_replaced_by_a_symlink_is_a_rewritten_history(repo: Path) -> None:
    path = repo / "schemas/envelope/result/v1.json"
    (repo / "copy.json").write_bytes(path.read_bytes())
    path.unlink()
    path.symlink_to(repo / "copy.json")
    assert diff(repo)[0] == [
        "schemas/envelope/result/v1.json: released on main and replaced by a symlink"
    ]


def edit_history(root: Path, edit: Any) -> None:
    history = manifest(root)
    edit(history)
    (root / "schemas/envelope/result/index.json").write_text(snapshots.manifest_text(history))


@pytest.mark.parametrize(
    ("edit", "problem"),
    [
        (
            lambda h: h["versions"]["1"].update(introduced_in="later"),
            "envelope/result: introduced_in of v1 changed to later",
        ),
        (
            lambda h: h["versions"]["1"].update(file="envelope/result/v1.other.json"),
            "envelope/result: the record of v1 changed in its history",
        ),
        (
            lambda h: h["versions"]["1"].update(deprecated=True),
            "envelope/result: the record of v1 changed in its history",
        ),
        (lambda h: h["versions"].clear(), "envelope/result: v1 removed from its history"),
        (
            lambda h: h.update(current="0"),
            "envelope/result: the current version went back from v1 to v0",
        ),
    ],
)
def test_a_history_edited_is_a_rewritten_history(repo: Path, edit: Any, problem: str) -> None:
    edit_history(repo, edit)
    assert problem in diff(repo)[0]


def test_an_earlier_record_edited_is_a_rewritten_history(repo: Path) -> None:
    following = contract(version=2, text="{}\n")
    write(repo, following)
    edit_history(repo, lambda h: h["versions"]["1"].update(deprecated=True))
    assert diff(repo, following)[0] == ["envelope/result: the record of v1 changed in its history"]
    edit_history(repo, lambda h: h["versions"]["1"].pop("deprecated"))
    edit_history(repo, lambda h: h["versions"]["1"].update(sha256="0" * 64))
    assert diff(repo, following)[0] == ["envelope/result: the record of v1 changed in its history"]


def test_check_refuses_a_key_of_a_record_it_does_not_know(tmp_path: Path) -> None:
    write(tmp_path, contract())
    write(tmp_path, contract(version=2))
    edit_history(tmp_path, lambda h: h["versions"]["1"].update(deprecated=True))
    assert "schemas/envelope/result/index.json: v1 has the unknown key deprecated" in (
        snapshots.check(tmp_path, [contract(version=2)])
    )


def test_a_released_file_the_pattern_misses_is_protected_through_its_history(
    git_repo: Path,
) -> None:
    # A part whose file name SNAPSHOT does not recognise is still released through the history
    # of its contract on the base, and an earlier version of it never changes.
    write(git_repo, contract())
    extra = git_repo / "schemas/envelope/result/v1.extra.json"
    extra.write_text("{}\n")
    edit_history(
        git_repo,
        lambda h: h["versions"]["1"].update(
            extra_file="envelope/result/v1.extra.json", extra_sha256=snapshots.sha256(b"{}\n")
        ),
    )
    git(git_repo, "add", ".")
    git(git_repo, "commit", "-q", "-m", "base")
    git(git_repo, "checkout", "-q", "-b", "feat/1-x")
    extra.write_text("[]\n")
    assert diff(git_repo, contract())[0] == [
        "schemas/envelope/result/v1.extra.json: released on main as an earlier version, and changed"
    ]


def test_generation_refuses_a_file_name_the_pattern_does_not_recognise(tmp_path: Path) -> None:
    refused = snapshots.write(
        tmp_path, [contract(parts=(("schema", "output"), ("extra", "output")))]
    )
    assert refused == [
        "schemas/envelope/result/v1.extra.json: a file name SNAPSHOT does not recognise; "
        "extend SNAPSHOT in scripts/schema_snapshots.py and the hook fallback"
    ]


def test_every_part_of_the_generator_is_recognised_by_the_pattern() -> None:
    document = {"parts": [{"part": "schema", "first_file": "v1.json"}], "contracts": []}
    assert snapshots.parse(document) == []
    document["parts"].append({"part": "extra", "first_file": "v1.extra.json"})
    with pytest.raises(
        SystemExit, match=r"part extra: v1\.extra\.json is not recognised by SNAPSHOT"
    ):
        snapshots.parse(document)


def test_a_release_stamps_an_unreleased_version(draft: Path) -> None:
    edit_history(draft, lambda h: h["versions"]["1"].update(introduced_in="0.1.0"))
    assert diff(draft) == ([], [], [], [])


BROKEN = "schemas/envelope/result/v1.json#/properties/a: `type` changed"


def test_a_breaking_change_of_an_unreleased_version_is_allowed_and_reported(
    draft: Path,
) -> None:
    # ADR 0030, amendment of 2026-11-20: a version no release holds may change until the first
    # release; the change is still reported, so that the pull request shows it.
    current = contract(text=schema(a={"type": "integer"}))
    write(draft, current)
    assert diff(draft, current) == ([], [], [BROKEN], [])
    git(draft, "tag", "not-a-release", "main")
    git(draft, "tag", "v0.1.0-rc.1", "main")
    assert diff(draft, current) == ([], [], [BROKEN], []), "only a tag v<semver> is a release"


@pytest.mark.parametrize(
    ("introduced_in", "tag"),
    [("0.1.0", None), (None, "v0.1.0"), ("0.1.0", "v0.1.0")],
    ids=["stamped", "tagged", "both"],
)
def test_a_breaking_change_of_a_released_version_is_refused(
    git_repo: Path, introduced_in: str | None, tag: str | None
) -> None:
    root = base_branch(git_repo, introduced_in, tag)
    current = contract(text=schema(a={"type": "integer"}))
    write(root, current)
    assert diff(root, current) == ([], [BROKEN], [], [])


def test_a_release_tag_without_the_version_does_not_release_it(git_repo: Path) -> None:
    (git_repo / "README.md").write_text("x\n")
    git(git_repo, "add", ".")
    git(git_repo, "commit", "-q", "-m", "first release, no contract")
    git(git_repo, "tag", "v0.1.0")
    root = base_branch(git_repo, None, None)
    current = contract(text=schema(a={"type": "integer"}))
    write(root, current)
    assert diff(root, current) == ([], [], [BROKEN], [])


def test_a_snapshot_released_on_the_base_after_the_fork_is_not_deleted(repo: Path) -> None:
    git(repo, "checkout", "-q", "main")
    write(repo, contract(version=2))
    git(repo, "add", ".")
    git(repo, "commit", "-q", "-m", "released later")
    git(repo, "checkout", "-q", "feat/1-x")
    assert diff(repo) == ([], [], [], [])


def test_without_a_merge_base_the_tip_of_the_base_is_released(repo: Path) -> None:
    git(repo, "checkout", "-q", "--orphan", "other")
    git(repo, "commit", "-q", "-m", "unrelated history")
    (repo / "schemas/envelope/result/v1.json").write_text("[]\n")
    assert diff(repo, contract())[1] == ["schemas/envelope/result/v1.json#/: changed"]


@pytest.mark.parametrize("base", ["origin/nowhere", "--output=written-by-git"])
def test_a_base_that_is_not_a_commit_is_refused(repo: Path, base: str) -> None:
    with pytest.raises(SystemExit, match="is not a commit"):
        diff(repo, base=base)
    assert not (repo / "written-by-git").exists()


# ---- the command


def test_generated_reads_the_document_of_the_generator(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    document = {
        "parts": [{"part": "schema", "first_file": "v1.json"}],
        "contracts": [
            {
                "area": "envelope",
                "name": "result",
                "version": 1,
                "files": [
                    {
                        "part": "schema",
                        "direction": "output",
                        "path": "envelope/result/v1.json",
                        "text": "{}\n",
                    }
                ],
            }
        ],
    }
    calls: list[Any] = []

    def run(args: Any, **kwargs: Any) -> subprocess.CompletedProcess[str]:
        calls.append((args, kwargs["cwd"]))
        return subprocess.CompletedProcess(args, 0, json.dumps(document), "")

    monkeypatch.setattr(subprocess, "run", run)
    assert snapshots.generated(tmp_path) == [
        Contract(
            "envelope", "result", 1, (File("schema", "output", "envelope/result/v1.json", "{}\n"),)
        )
    ]
    assert calls == [(snapshots.GENERATOR, tmp_path)]

    def failed(args: Any, **kwargs: Any) -> subprocess.CompletedProcess[str]:
        return subprocess.CompletedProcess(args, 1, "", "a refused schema")

    monkeypatch.setattr(subprocess, "run", failed)
    with pytest.raises(SystemExit, match="a refused schema"):
        snapshots.generated(tmp_path)


def test_main_writes_checks_and_diffs(
    repo: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    current = [contract(text=schema(a={"type": "string"}))]
    monkeypatch.setattr(snapshots, "generated", lambda root: current)
    snapshots.main(["--check"], root=repo)
    snapshots.main(["--diff-against", "main"], root=repo)
    assert "additive or none" in capsys.readouterr().out
    current[0] = contract(text=schema(a={"type": "integer"}))
    with pytest.raises(SystemExit, match="Run: uv run just generate"):
        snapshots.main(["--check"], root=repo)
    snapshots.main([], root=repo)
    with pytest.raises(SystemExit, match=r"Breaking changes(.|\n)*`type` changed"):
        snapshots.main(["--diff-against", "main"], root=repo)
    current[0] = contract(version=2, text="{}\n")
    snapshots.main([], root=repo)
    (repo / "schemas/envelope/result/v1.json").write_text("{}\n")
    with pytest.raises(SystemExit, match="Schema history rewritten"):
        snapshots.main(["--diff-against", "main"], root=repo)
    current[0] = contract(version=1)
    with pytest.raises(SystemExit, match="not written"):
        snapshots.main([], root=repo)


def test_main_lists_the_additive_changes_by_file(
    repo: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    current = [contract(text=schema(a={"type": "string"}, b={"type": "integer"}))]
    monkeypatch.setattr(snapshots, "generated", lambda root: current)
    snapshots.main([], root=repo)
    snapshots.main(["--diff-against", "main"], root=repo)
    out = capsys.readouterr().out
    assert "Additive changes of the contracts, allowed:" in out
    assert "schemas/envelope/result/v1.json: changed, additively" in out


def test_main_reports_a_breaking_change_of_an_unreleased_version_and_passes(
    draft: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    current = [contract(text=schema(a={"type": "integer"}))]
    monkeypatch.setattr(snapshots, "generated", lambda root: current)
    snapshots.main([], root=draft)
    snapshots.main(["--diff-against", "main"], root=draft)
    out = capsys.readouterr().out
    assert "Breaking changes of a version no release holds" in out
    assert BROKEN in out
    assert "the other changes against main are additive or none" in out


# ---- the pattern shared with the hook


def test_the_fallback_pattern_of_the_hook_equals_the_pattern_of_the_script() -> None:
    # The hook keeps a copy for the case where it cannot import schema_snapshots (fail closed).
    assert hook.FALLBACK_SNAPSHOT.pattern == snapshots.SNAPSHOT.pattern


def test_the_pattern_of_snapshots_shared_by_the_script_and_the_hook() -> None:
    found = snapshots.SNAPSHOT.search("schemas/a/b/v12.output.json")
    assert found is not None and found["version"] == "12"
    for path in ("schemas/a/b/v1.json", "schemas/a/b/v12.output.json", "schemas/a/b/v1.input.json"):
        assert snapshots.SNAPSHOT.search(path), path
    for path in (
        "schemas/a/b/index.json",
        "schemas/a/v1.yaml",
        "docs/a/v1.json",
        f"{snapshots.COPIES}/a/b/v1.json",
    ):
        assert not snapshots.SNAPSHOT.search(path), path
    assert hook.reason_to_block("docs/a/v1.json", "main") is None


def test_the_documentation_build_publishes_the_snapshots_and_the_history(tmp_path: Path) -> None:
    import docs

    write(tmp_path, contract())
    stale = tmp_path / "site/schemas/old.json"
    stale.parent.mkdir(parents=True)
    stale.write_text("{}\n")
    docs.publish_schemas(tmp_path)
    site = tmp_path / "site/schemas"
    assert read(site, "envelope/result/v1.json") == ENVELOPE_TEXT
    assert json.loads(read(site, "envelope/result/index.json")) == manifest(tmp_path)
    assert json.loads(read(site, "index.json")) == snapshots.schema_history(tmp_path)
    assert not stale.exists()
