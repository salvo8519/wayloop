"""scripts/check_schemas.py: released schema snapshots never change (SPEC-0001 R1, R16, AC3)."""

from __future__ import annotations

import subprocess
from pathlib import Path

import _git
import check_schemas as schemas
import protect_contracts as hook
import pytest
from conftest import git


def write(root: Path, path: str, text: str = "{}\n") -> None:
    target = root / path
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(text, encoding="utf-8")


def problems(root: Path, base: str = "main") -> list[str]:
    found: list[str] = schemas.problems(root, base)
    return found


@pytest.fixture
def repo(git_repo: Path) -> Path:
    """A repository whose `main` holds released snapshots; the work happens on a branch."""
    write(git_repo, "README.md", "x\n")
    for path in (
        "schemas/commands/next/v1.json",
        "schemas/commands/next/v1.output.json",
        "schemas/handover/v1.input.json",
        "schemas/index.json",
    ):
        write(git_repo, path)
    git(git_repo, "add", ".")
    git(git_repo, "commit", "-q", "-m", "base")
    git(git_repo, "checkout", "-q", "-b", "feat/1-x")
    return git_repo


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


def test_a_snapshot_added_on_a_lower_branch_stays_editable_against_main(repo: Path) -> None:
    write(repo, "schemas/commands/next/v2.json")
    git(repo, "add", ".")
    git(repo, "commit", "-q", "-m", "lower branch")
    git(repo, "checkout", "-q", "-b", "feat/2-upper")
    write(repo, "schemas/commands/next/v2.json", '{"type": "object"}\n')
    git(repo, "commit", "-q", "-am", "upper branch")
    assert problems(repo, "main") == []
    assert problems(repo, "feat/1-x") == [
        "schemas/commands/next/v2.json: released on feat/1-x and changed"
    ], "the base decides what is released: CI passes origin/main"


def test_a_snapshot_released_on_the_base_after_the_fork_is_not_deleted(repo: Path) -> None:
    git(repo, "checkout", "-q", "main")
    write(repo, "schemas/commands/next/v2.json")
    git(repo, "add", ".")
    git(repo, "commit", "-q", "-m", "released later")
    git(repo, "checkout", "-q", "feat/1-x")
    assert problems(repo) == []


def test_without_a_merge_base_the_tip_of_the_base_is_released(repo: Path) -> None:
    git(repo, "checkout", "-q", "--orphan", "other")
    git(repo, "rm", "-q", "-r", "--cached", ".")
    git(repo, "add", ".")
    git(repo, "commit", "-q", "-m", "unrelated history")
    write(repo, "schemas/handover/v1.input.json", "[]\n")
    assert problems(repo) == ["schemas/handover/v1.input.json: released on main and changed"]


def test_a_snapshot_with_a_non_ascii_path_is_protected(repo: Path) -> None:
    write(repo, "schemas/\u00e9/v1.json")
    git(repo, "add", ".")
    git(repo, "commit", "-q", "-m", "accented")
    git(repo, "branch", "-q", "-f", "main")
    write(repo, "schemas/\u00e9/v1.json", "[]\n")
    assert problems(repo) == ["schemas/\u00e9/v1.json: released on main and changed"]


def test_a_released_snapshot_replaced_by_a_symlink_fails(repo: Path) -> None:
    path = "schemas/commands/next/v1.json"
    (repo / "copy.json").write_bytes((repo / path).read_bytes())
    (repo / path).unlink()
    (repo / path).symlink_to(repo / "copy.json")
    assert problems(repo) == [f"{path}: released on main and replaced by a symlink"]


def test_a_released_snapshot_with_a_changed_mode_fails(repo: Path) -> None:
    path = repo / "schemas/commands/next/v1.json"
    path.chmod(0o755)
    assert problems(repo) == [
        "schemas/commands/next/v1.json: released on main and its mode changed (100644 to 100755)"
    ]


def test_an_incomplete_answer_of_git_is_a_clear_error(
    repo: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    real = _git.run

    def short(*args: str, cwd: Path | str) -> subprocess.CompletedProcess[str]:
        result: subprocess.CompletedProcess[str] = real(*args, cwd=cwd)
        if args[0] == "hash-object":
            result.stdout = ""
        return result

    monkeypatch.setattr(_git, "run", short)
    with pytest.raises(SystemExit, match="git hash-object returned 0 ids for 3 files"):
        problems(repo)


def test_the_pattern_of_snapshots_shared_by_the_check_and_the_hook() -> None:
    # The hook imports SNAPSHOT from check_schemas, so both recognise the same files.
    found = schemas.SNAPSHOT.search("schemas/a/v12.output.json")
    assert found is not None and found["version"] == "12"
    for path in ("schemas/a/v1.json", "schemas/a/v12.output.json", "schemas/a/v1.input.json"):
        assert schemas.SNAPSHOT.search(path), path
    for path in ("schemas/index.json", "schemas/a/v1.yaml", "docs/a/v1.json"):
        assert not schemas.SNAPSHOT.search(path), path
    assert hook.reason_to_block("docs/a/v1.json", "main") is None
