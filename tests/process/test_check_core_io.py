"""scripts/check_core_io.py: wayloop-core has no I/O dependency and no I/O in its source."""

from __future__ import annotations

import importlib.util
import shutil
from pathlib import Path
from typing import Any

import pytest

ROOT = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location("check_core_io", ROOT / "scripts/check_core_io.py")
assert _spec and _spec.loader
core_io = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(core_io)


def metadata(edges: dict[str, list[tuple[str, str | None]]]) -> dict[str, Any]:
    """Cargo metadata for a graph given as {crate: [(dependency, kind), ...]}.

    The kind is None for a normal dependency, "dev" or "build" otherwise, as cargo writes it.
    """
    names = set(edges) | {dep for deps in edges.values() for dep, _ in deps}
    pid = {name: f"registry+https://example.org#{name}@1.0.0" for name in names}
    return {
        "packages": [{"name": name, "id": pid[name]} for name in sorted(names)],
        "workspace_members": [pid["wayloop-core"]],
        "resolve": {
            "nodes": [
                {
                    "id": pid[name],
                    "deps": [
                        {
                            "name": dep.replace("-", "_"),
                            "pkg": pid[dep],
                            "dep_kinds": [{"kind": kind, "target": None}],
                        }
                        for dep, kind in edges.get(name, [])
                    ],
                }
                for name in sorted(names)
            ]
        },
    }


def test_a_core_without_dependencies_passes() -> None:
    assert core_io.io_dependencies(metadata({"wayloop-core": []})) == []


def test_a_pure_dependency_passes() -> None:
    graph = {"wayloop-core": [("serde", None)], "serde": [("serde-derive", None)]}
    assert core_io.io_dependencies(metadata(graph)) == []


def test_a_direct_io_dependency_is_refused() -> None:
    graph = {"wayloop-core": [("tokio", None)]}
    assert core_io.io_dependencies(metadata(graph)) == [["wayloop-core", "tokio"]]


def test_a_transitive_io_dependency_is_refused_with_its_path() -> None:
    graph = {"wayloop-core": [("helper", None)], "helper": [("rusqlite", None)]}
    assert core_io.io_dependencies(metadata(graph)) == [["wayloop-core", "helper", "rusqlite"]]


def test_names_are_compared_with_hyphens_and_underscores_alike() -> None:
    graph = {"wayloop-core": [("libsqlite3_sys", None)]}
    assert core_io.io_dependencies(metadata(graph)) == [["wayloop-core", "libsqlite3_sys"]]


def test_a_dev_dependency_is_not_shipped_and_passes() -> None:
    graph = {"wayloop-core": [("tempfile", "dev")]}
    assert core_io.io_dependencies(metadata(graph)) == []


def test_a_build_dependency_runs_at_build_time_and_passes() -> None:
    graph = {"wayloop-core": [("cc", "build")], "cc": [("tempfile", None)]}
    assert core_io.io_dependencies(metadata(graph)) == []


def test_a_crate_reached_twice_is_reported_once() -> None:
    graph = {
        "wayloop-core": [("a", None), ("b", None)],
        "a": [("reqwest", None)],
        "b": [("reqwest", None)],
    }
    assert core_io.io_dependencies(metadata(graph)) == [["wayloop-core", "a", "reqwest"]]


def source(tmp_path: Path, text: str) -> Path:
    src = tmp_path / "src"
    src.mkdir()
    (src / "lib.rs").write_text(text, encoding="utf-8")
    return src


@pytest.mark.parametrize(
    "line",
    [
        "use std::fs;",
        "let text = std::fs::read_to_string(path);",
        "use std::{collections::BTreeMap, net::TcpStream};",
        'let _ = ::std::process::Command::new("git");',
        'let home = std::env::var("HOME");',
        "let out = std::io::stdout();",
        'println!("{}", x);',
        'eprintln!("{}", x);',
    ],
)
def test_io_in_the_source_is_refused_with_its_line(tmp_path: Path, line: str) -> None:
    src = source(tmp_path, f"//! Pure.\n\npub fn f() {{\n    {line}\n}}\n")
    found = core_io.io_in_source(src)
    assert [(path.name, number) for path, number, _ in found] == [("lib.rs", 4)]


def test_a_grouped_import_over_several_lines_is_refused(tmp_path: Path) -> None:
    src = source(tmp_path, "use std::{\n    collections::BTreeMap,\n    fs::File,\n};\n")
    assert [number for _, number, _ in core_io.io_in_source(src)] == [1]


def test_comments_and_compile_time_macros_pass(tmp_path: Path) -> None:
    text = (
        "//! Performs no I/O: no std::fs, no println!.\n"
        "/// Version, read at compile time with env!.\n"
        'pub const VERSION: &str = env!("CARGO_PKG_VERSION");\n'
        "use std::io::Write; // a trait, writing to a buffer the caller owns\n"
        "use std::fmt::Write as _;\n"
    )
    assert core_io.io_in_source(source(tmp_path, text)) == []


def test_main_refuses_an_io_dependency(monkeypatch: pytest.MonkeyPatch) -> None:
    graph = {"wayloop-core": [("helper", None)], "helper": [("tokio", None)]}
    monkeypatch.setattr(core_io, "cargo_metadata", lambda: metadata(graph))
    with pytest.raises(SystemExit, match="wayloop-core -> helper -> tokio"):
        core_io.main()


def test_main_refuses_io_in_the_source(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(core_io, "cargo_metadata", lambda: metadata({"wayloop-core": []}))
    monkeypatch.setattr(core_io, "CORE_SRC", source(tmp_path, "use std::fs;\n"))
    monkeypatch.setattr(core_io, "ROOT", tmp_path)
    with pytest.raises(SystemExit, match=r"src/lib.rs:1: use std::fs;"):
        core_io.main()


@pytest.mark.skipif(shutil.which("cargo") is None, reason="cargo is not installed")
def test_the_current_tree_passes(capsys: pytest.CaptureFixture[str]) -> None:
    core_io.main()
    assert "no I/O" in capsys.readouterr().out
