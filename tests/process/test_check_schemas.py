"""scripts/check_schemas.py: released schema snapshots never change (SPEC-0001 R1, R16, AC3)."""

from __future__ import annotations

import importlib.util
import os
import subprocess
from pathlib import Path
from types import ModuleType

import pytest

ROOT = Path(__file__).resolve().parents[2]


def _load(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


schemas = _load("check_schemas", ROOT / "scripts" / "check_schemas.py")
hook = _load("protect_contracts", ROOT / ".claude" / "hooks" / "protect_contracts.py")

GIT_ENV = {
    "GIT_AUTHOR_NAME": "t",
    "GIT_AUTHOR_EMAIL": "t@t",
    "GIT_COMMITTER_NAME": "t",
    "GIT_COMMITTER_EMAIL": "t@t",
}


def git(cwd: Path, *args: str) -> None:
    subprocess.run(["git", *args], cwd=cwd, env={**os.environ, **GIT_ENV}, check=True)


def write(root: Path, path: str, text: str = "{}\n") -> None:
    target = root / path
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(text, encoding="utf-8")


def problems(root: Path, base: str = "main") -> list[str]:
    found: list[str] = schemas.problems(root, base)
    return found


@pytest.fixture
def repo(tmp_path: Path) -> Path:
    """A repository whose `main` holds released snapshots; the work happens on a branch."""
    write(tmp_path, "README.md", "x\n")
    for path in (
        "schemas/commands/next/v1.json",
        "schemas/commands/next/v1.output.json",
        "schemas/handover/v1.input.json",
        "schemas/index.json",
    ):
        write(tmp_path, path)
    git(tmp_path, "init", "-q", "-b", "main")
    git(tmp_path, "add", ".")
    git(tmp_path, "commit", "-q", "-m", "base")
    git(tmp_path, "checkout", "-q", "-b", "feat/1-x")
    return tmp_path


def test_an_empty_tree_passes(tmp_path: Path) -> None:
    write(tmp_path, "README.md", "x\n")
    git(tmp_path, "init", "-q", "-b", "main")
    git(tmp_path, "add", ".")
    git(tmp_path, "commit", "-q", "-m", "base")
    assert problems(tmp_path) == []


def test_unchanged_released_snapshots_pass(repo: Path) -> None:
    assert problems(repo) == []


@pytest.mark.parametrize(
    "path",
    [
        "schemas/commands/next/v1.json",
        "schemas/commands/next/v1.output.json",
        "schemas/handover/v1.input.json",
    ],
)
def test_a_changed_released_snapshot_fails(repo: Path, path: str) -> None:
    write(repo, path, '{"type": "object"}\n')
    assert problems(repo) == [f"{path}: released on main and changed"]
    git(repo, "commit", "-q", "-am", "change")
    assert problems(repo) == [f"{path}: released on main and changed"]


def test_a_deleted_released_snapshot_fails(repo: Path) -> None:
    (repo / "schemas/commands/next/v1.output.json").unlink()
    assert problems(repo) == ["schemas/commands/next/v1.output.json: released on main and deleted"]


def test_a_new_version_beside_the_unchanged_one_passes(repo: Path) -> None:
    write(repo, "schemas/commands/next/v2.json", '{"type": "object"}\n')
    write(repo, "schemas/commands/next/v2.output.json")
    write(repo, "schemas/handover/v2.input.json")
    write(repo, "schemas/artifacts/brief/v1.json")
    assert problems(repo) == []
    git(repo, "add", ".")
    git(repo, "commit", "-q", "-m", "next version")
    write(repo, "schemas/commands/next/v2.json", '{"type": "array"}\n')
    assert problems(repo) == [], "a snapshot added on the branch stays editable until it merges"


def test_other_files_under_schemas_may_change(repo: Path) -> None:
    write(repo, "schemas/index.json", '{"next": 2}\n')
    assert problems(repo) == []


def test_a_gap_in_versions_fails(repo: Path) -> None:
    write(repo, "schemas/commands/next/v3.json")
    write(repo, "schemas/artifacts/brief/v2.json")
    assert problems(repo) == [
        "schemas/artifacts/brief: version 1 is missing (versions found: 2)",
        "schemas/commands/next: version 2 is missing (versions found: 1, 3)",
    ]


@pytest.mark.parametrize("name", ["v0.json", "v01.json", "v02.output.json"])
def test_a_version_that_is_not_a_positive_integer_fails(repo: Path, name: str) -> None:
    write(repo, f"schemas/commands/next/{name}")
    found = problems(repo)
    assert f"schemas/commands/next/{name}: the version is not a positive integer" in found


def test_an_unknown_base_is_refused(repo: Path) -> None:
    with pytest.raises(SystemExit, match="base 'origin/nowhere' is not a commit"):
        problems(repo, "origin/nowhere")


def test_main_exits_with_the_problems(repo: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(schemas, "ROOT", repo)
    schemas.main(["--base", "main"])
    write(repo, "schemas/handover/v1.input.json", "[]\n")
    with pytest.raises(SystemExit, match=r"v1\.input\.json: released on main and changed"):
        schemas.main(["--base", "main"])


def test_the_check_and_the_hook_recognise_the_same_snapshots() -> None:
    assert schemas.SNAPSHOT.pattern == hook.SNAPSHOT.pattern
    for path in ("schemas/a/v1.json", "schemas/a/v12.output.json", "schemas/a/v1.input.json"):
        assert hook.SNAPSHOT.search(path), path
    for path in ("schemas/index.json", "schemas/a/v1.yaml", "docs/a/v1.json"):
        assert not hook.SNAPSHOT.search(path), path
