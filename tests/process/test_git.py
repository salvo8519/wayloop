"""git access shared by the scripts and the hooks (scripts/_git.py)."""

from __future__ import annotations

import subprocess
from pathlib import Path

import _git
import pytest
from conftest import git


def write(root: Path, *paths: str) -> None:
    for path in paths:
        (root / path).parent.mkdir(parents=True, exist_ok=True)
        (root / path).write_text("x\n", encoding="utf-8")


@pytest.fixture
def repo(git_repo: Path) -> Path:
    """`main` with three files; the branch `feat/1-x` renames one, the work tree adds more."""
    write(git_repo, "a.yml", "b.md", "specs/0001-x/tasks.md")
    git(git_repo, "add", ".")
    git(git_repo, "commit", "-q", "-m", "base")
    git(git_repo, "switch", "-q", "-c", "feat/1-x")
    git(git_repo, "mv", "b.md", "c.md")
    git(git_repo, "commit", "-q", "-m", "rename")
    return git_repo


def test_ls_files_reads_pathspecs_and_untracked_files(repo: Path) -> None:
    write(repo, "new.yaml", "ignored.yml")
    (repo / ".gitignore").write_text("ignored.yml\n", encoding="utf-8")
    assert _git.ls_files("*.yml", "*.yaml", cwd=repo) == ["a.yml"]
    assert _git.ls_files("specs/*/tasks.md", cwd=repo) == ["specs/0001-x/tasks.md"]
    assert _git.ls_files(others=True, cwd=repo) == [
        ".gitignore",
        "a.yml",
        "c.md",
        "new.yaml",
        "specs/0001-x/tasks.md",
    ]


def test_changed_on_a_branch_counts_both_sides_of_a_rename(repo: Path) -> None:
    assert _git.changed("main", cwd=repo) == ["b.md", "c.md"]


def test_changed_falls_back_to_the_base_without_a_merge_base(repo: Path) -> None:
    git(repo, "checkout", "-q", "--orphan", "other", "main")
    git(repo, "commit", "-q", "-m", "the tree of main, without its history")
    assert _git.changed("main", cwd=repo) == []


def test_changed_in_the_index_and_the_work_tree(repo: Path) -> None:
    (repo / "a.yml").write_text("y\n", encoding="utf-8")
    write(repo, "é.md")
    write(repo, "staged.md")
    git(repo, "add", "staged.md")
    assert _git.changed(staged=True, cwd=repo) == ["staged.md"]
    assert _git.changed(cwd=repo) == ["a.yml", "staged.md"]
    assert _git.changed(untracked=True, cwd=repo) == ["a.yml", "staged.md", "é.md"]


def test_output_and_lines_outside_a_repository(tmp_path: Path) -> None:
    assert _git.output("rev-parse", "HEAD", cwd=tmp_path) == ""
    assert _git.run("rev-parse", "HEAD", cwd=tmp_path).returncode != 0
    with pytest.raises(subprocess.CalledProcessError):
        _git.lines("rev-parse", "HEAD", cwd=tmp_path)
    with pytest.raises(subprocess.CalledProcessError):
        _git.changed(cwd=tmp_path)


@pytest.mark.parametrize("base", ["--output=written-by-git", "-p", ""])
def test_a_base_read_as_an_option_is_refused(repo: Path, base: str) -> None:
    with pytest.raises(ValueError, match="not a revision"):
        _git.changed(base, cwd=repo)
    assert not (repo / "written-by-git").exists()


def test_tags_and_the_paths_a_revision_holds(repo: Path) -> None:
    git(repo, "tag", "v0.2.0", "main")
    git(repo, "tag", "v0.1.0", "main")
    git(repo, "tag", "other")
    assert _git.tags("v*", cwd=repo) == ["v0.1.0", "v0.2.0"]
    assert _git.holds("v0.1.0", "b.md", cwd=repo)
    assert not _git.holds("v0.1.0", "c.md", cwd=repo)
    assert not _git.holds("nowhere", "b.md", cwd=repo)
    with pytest.raises(ValueError, match="not a revision"):
        _git.holds("--output=written-by-git", "b.md", cwd=repo)
