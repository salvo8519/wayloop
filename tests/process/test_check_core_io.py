"""scripts/check_core_io.py: the pure crates depend only on their allowlist and keep their lints."""

from __future__ import annotations

import importlib.util
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location("check_core_io", ROOT / "scripts/check_core_io.py")
assert _spec and _spec.loader
core_io = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(core_io)

needs_cargo = pytest.mark.skipif(shutil.which("cargo") is None, reason="cargo is not installed")


def tree(*lines: str) -> str:
    """Output of `cargo tree --prefix depth --format {p}`, from lines written as "depth name"."""
    out = []
    for line in lines:
        depth, name, *marker = line.split()
        out.append(f"{depth}{name} v1.0.0 {' '.join(marker)}".rstrip())
    return "\n".join(out) + "\n"


# The dependency graph read from `cargo tree`.


def test_a_crate_without_dependencies_has_no_paths() -> None:
    assert core_io.dependency_paths(tree("0 wayloop-core")) == {}


def test_every_crate_is_reached_by_its_shortest_path() -> None:
    text = tree("0 wayloop-core", "1 a", "2 b", "3 tokio", "1 tokio (*)", "2 mio")
    assert core_io.dependency_paths(text) == {
        "a": ["wayloop-core", "a"],
        "b": ["wayloop-core", "a", "b"],
        "tokio": ["wayloop-core", "tokio"],
        "mio": ["wayloop-core", "tokio", "mio"],
    }


def test_a_line_that_is_not_a_package_is_refused() -> None:
    with pytest.raises(ValueError, match="unexpected line"):
        core_io.dependency_paths("0wayloop-core v0.0.0\nwarning: something\n")


# The allowlist.


def test_a_crate_on_the_allowlist_passes() -> None:
    text = tree("0 wayloop-core", "1 wayloop-schemas")
    assert core_io.refused("wayloop-core", text, frozenset({"wayloop-schemas"})) == []


def test_a_crate_off_the_allowlist_is_refused_with_its_path() -> None:
    text = tree("0 wayloop-core", "1 helper", "2 itoa")
    assert core_io.refused("wayloop-core", text, frozenset({"helper"})) == [
        "wayloop-core -> helper -> itoa: itoa is not on the allowlist of wayloop-core"
    ]


def test_a_known_io_crate_is_named_as_such() -> None:
    text = tree("0 wayloop-schemas", "1 tokio")
    assert core_io.refused("wayloop-schemas", text, frozenset()) == [
        (
            "wayloop-schemas -> tokio: tokio is not on the allowlist of wayloop-schemas"
            " (a known I/O crate)"
        )
    ]


def test_names_of_io_crates_are_compared_with_hyphens_and_underscores_alike() -> None:
    text = tree("0 wayloop-schemas", "1 libsqlite3_sys")
    assert core_io.refused("wayloop-schemas", text, frozenset())[0].endswith("(a known I/O crate)")


@pytest.mark.parametrize(
    ("crate", "dependency"),
    [
        ("wayloop-core", "wayloop-runtime"),
        ("wayloop-core", "wayloop-cli"),
        ("wayloop-schemas", "wayloop-core"),
    ],
)
def test_a_dependency_against_the_direction_is_named_as_such(crate: str, dependency: str) -> None:
    text = tree(f"0 {crate}", f"1 {dependency}")
    assert core_io.refused(crate, text, frozenset())[0].endswith(
        "(against the dependency direction, ADR 0006)"
    )


def test_the_allowlists_are_those_reviewed() -> None:
    assert {
        "wayloop-schemas": frozenset(),
        "wayloop-core": frozenset({"wayloop-schemas"}),
    } == core_io.PURE_CRATES


# The lint guards: clippy.toml and the forbid attribute at the crate root.


def crate_dir(tmp_path: Path, clippy: str | None, lib: str) -> Path:
    path = tmp_path / "crate"
    (path / "src").mkdir(parents=True)
    (path / "src" / "lib.rs").write_text(lib, encoding="utf-8")
    if clippy is not None:
        (path / "clippy.toml").write_text(clippy, encoding="utf-8")
    return path


def test_the_lint_guards_of_the_pure_crates_are_present() -> None:
    for crate in core_io.PURE_CRATES:
        assert core_io.missing_guards(ROOT / "crates" / crate) == []


def test_a_missing_clippy_toml_is_reported(tmp_path: Path) -> None:
    lib = (ROOT / "crates/wayloop-core/src/lib.rs").read_text(encoding="utf-8")
    assert core_io.missing_guards(crate_dir(tmp_path, None, lib)) == ["clippy.toml is missing"]


def test_an_entry_removed_from_clippy_toml_is_reported(tmp_path: Path) -> None:
    clippy = (ROOT / "crates/wayloop-core/clippy.toml").read_text(encoding="utf-8")
    lib = (ROOT / "crates/wayloop-core/src/lib.rs").read_text(encoding="utf-8")
    clippy = "\n".join(line for line in clippy.splitlines() if '"std::thread::spawn"' not in line)
    clippy = "\n".join(
        line for line in clippy.splitlines() if '"std::collections::HashMap"' not in line
    )
    assert core_io.missing_guards(crate_dir(tmp_path, clippy, lib)) == [
        "clippy.toml: disallowed-methods lacks std::thread::spawn",
        "clippy.toml: disallowed-types lacks std::collections::HashMap",
    ]


def test_entries_may_be_plain_paths(tmp_path: Path) -> None:
    methods = ", ".join(f'"{path}"' for path in sorted(core_io.REQUIRED_METHODS))
    types = ", ".join(f'"{path}"' for path in sorted(core_io.REQUIRED_TYPES))
    clippy = f"disallowed-methods = [{methods}]\ndisallowed-types = [{types}]\n"
    lib = (ROOT / "crates/wayloop-core/src/lib.rs").read_text(encoding="utf-8")
    assert core_io.missing_guards(crate_dir(tmp_path, clippy, lib)) == []


def test_an_invalid_clippy_toml_is_reported(tmp_path: Path) -> None:
    lib = (ROOT / "crates/wayloop-core/src/lib.rs").read_text(encoding="utf-8")
    found = core_io.missing_guards(crate_dir(tmp_path, "disallowed-methods = [", lib))
    assert len(found) == 1 and found[0].startswith("clippy.toml is not valid TOML")


@pytest.mark.parametrize(
    ("lib", "missing"),
    [
        ("//! No attribute.\n", core_io.REQUIRED_LINTS),
        (
            "#![forbid(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)]\n",
            ("clippy::disallowed_methods", "clippy::disallowed_types"),
        ),
        # deny can be lifted by an inner allow; forbid cannot.
        ("#![deny(" + ", ".join(core_io.REQUIRED_LINTS) + ")]\n", core_io.REQUIRED_LINTS),
        # A commented attribute is no attribute.
        ("// #![forbid(" + ", ".join(core_io.REQUIRED_LINTS) + ")]\n", core_io.REQUIRED_LINTS),
    ],
)
def test_a_lint_missing_from_the_forbid_attribute_is_reported(
    tmp_path: Path, lib: str, missing: tuple[str, ...]
) -> None:
    clippy = (ROOT / "crates/wayloop-core/clippy.toml").read_text(encoding="utf-8")
    assert core_io.missing_guards(crate_dir(tmp_path, clippy, lib)) == [
        f"src/lib.rs: #![forbid(...)] lacks {lint}" for lint in missing
    ]


def test_the_forbid_attribute_may_span_lines_and_be_split(tmp_path: Path) -> None:
    clippy = (ROOT / "crates/wayloop-core/clippy.toml").read_text(encoding="utf-8")
    lib = (
        "//! Pure.\n#![forbid(\n    clippy::print_stdout,\n    clippy::print_stderr,\n)]\n"
        "#![forbid(clippy::dbg_macro, clippy::disallowed_methods, clippy::disallowed_types)]\n"
    )
    assert core_io.missing_guards(crate_dir(tmp_path, clippy, lib)) == []


# Real resolution by cargo, on a small workspace of path crates (no network).


def workspace(tmp_path: Path, manifests: dict[str, str]) -> Path:
    """A workspace of path crates; each manifest is the text after the [package] table."""
    (tmp_path / "Cargo.toml").write_text(
        '[workspace]\nresolver = "3"\nmembers = ['
        + ", ".join(f'"crates/{name}"' for name in manifests)
        + "]\n",
        encoding="utf-8",
    )
    for name, body in manifests.items():
        (tmp_path / "crates" / name / "src").mkdir(parents=True)
        (tmp_path / "crates" / name / "src" / "lib.rs").write_text("//! Test.\n", encoding="utf-8")
        (tmp_path / "crates" / name / "Cargo.toml").write_text(
            f'[package]\nname = "{name}"\nversion = "0.1.0"\nedition = "2024"\n{body}',
            encoding="utf-8",
        )
    subprocess.run(["cargo", "generate-lockfile", "--offline", "--quiet"], cwd=tmp_path, check=True)
    return tmp_path


def resolved(root: Path, crate: str = "wayloop-core") -> list[str]:
    problems: list[str] = core_io.refused(crate, core_io.cargo_tree(crate, root), frozenset())
    return problems


@needs_cargo
def test_cargo_resolves_a_renamed_dependency_by_its_package_name(tmp_path: Path) -> None:
    root = workspace(
        tmp_path,
        {
            "wayloop-core": '[dependencies]\nrt = { package = "tokio", path = "../tokio" }\n',
            "tokio": "",
        },
    )
    assert resolved(root) == [
        "wayloop-core -> tokio: tokio is not on the allowlist of wayloop-core (a known I/O crate)"
    ]


@needs_cargo
def test_cargo_resolves_a_target_specific_dependency(tmp_path: Path) -> None:
    body = "[target.'cfg(windows)'.dependencies]\nmio = { path = \"../mio\" }\n"
    root = workspace(tmp_path, {"wayloop-core": body, "mio": ""})
    assert resolved(root) == [
        "wayloop-core -> mio: mio is not on the allowlist of wayloop-core (a known I/O crate)"
    ]


@needs_cargo
def test_cargo_resolves_an_optional_dependency_behind_a_non_default_feature(
    tmp_path: Path,
) -> None:
    body = (
        '[features]\nnet = ["dep:reqwest"]\n'
        '[dependencies]\nreqwest = { path = "../reqwest", optional = true }\n'
    )
    root = workspace(tmp_path, {"wayloop-core": body, "reqwest": ""})
    assert resolved(root) == [
        (
            "wayloop-core -> reqwest: reqwest is not on the allowlist of wayloop-core"
            " (a known I/O crate)"
        )
    ]


@needs_cargo
def test_a_feature_enabled_by_another_member_is_not_blamed_on_the_pure_crate(
    tmp_path: Path,
) -> None:
    root = workspace(
        tmp_path,
        {
            "wayloop-core": '[dependencies]\nhelper = { path = "../helper" }\n',
            "wayloop-cli": '[dependencies]\nhelper = { path = "../helper", features = ["rt"] }\n',
            "helper": (
                '[features]\nrt = ["dep:tokio"]\n'
                '[dependencies]\ntokio = { path = "../tokio", optional = true }\n'
            ),
            "tokio": "",
        },
    )
    text = core_io.cargo_tree("wayloop-core", root)
    assert core_io.refused("wayloop-core", text, frozenset({"helper"})) == []


@needs_cargo
def test_development_and_build_dependencies_are_skipped(tmp_path: Path) -> None:
    body = (
        '[dev-dependencies]\ntempfile = { path = "../tempfile" }\n'
        '[build-dependencies]\ncc = { path = "../cc" }\n'
    )
    root = workspace(
        tmp_path,
        {
            "wayloop-core": body,
            "tempfile": "",
            "cc": '[dependencies]\nnix = { path = "../nix" }\n',
            "nix": "",
        },
    )
    assert resolved(root) == []


@needs_cargo
def test_a_pure_crate_depending_on_the_runtime_is_refused(tmp_path: Path) -> None:
    root = workspace(
        tmp_path,
        {
            "wayloop-core": '[dependencies]\nwayloop-runtime = { path = "../wayloop-runtime" }\n',
            "wayloop-runtime": "",
        },
    )
    assert resolved(root) == [
        (
            "wayloop-core -> wayloop-runtime:"
            " wayloop-runtime is not on the allowlist of wayloop-core"
            " (against the dependency direction, ADR 0006)"
        )
    ]


@needs_cargo
def test_cargo_tree_never_rewrites_the_lock_file(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # CI sets CARGO_TERM_COLOR=always; the error must still be plain text.
    monkeypatch.setenv("CARGO_TERM_COLOR", "always")
    root = workspace(tmp_path, {"wayloop-core": "", "helper": ""})
    manifest = root / "crates/wayloop-core/Cargo.toml"
    manifest.write_text(
        manifest.read_text(encoding="utf-8") + '[dependencies]\nhelper = { path = "../helper" }\n',
        encoding="utf-8",
    )
    with pytest.raises(core_io.CargoError, match=r"^error: .*lock file.*--locked"):
        core_io.cargo_tree("wayloop-core", root)


# The whole check.


def pure_workspace(tmp_path: Path, core_body: str = "") -> Path:
    root = workspace(
        tmp_path,
        {
            "wayloop-schemas": "",
            "wayloop-core": '[dependencies]\nwayloop-schemas = { path = "../wayloop-schemas" }\n'
            + core_body,
            "tokio": "",
        },
    )
    for crate in core_io.PURE_CRATES:
        for name in ("clippy.toml", "src/lib.rs"):
            shutil.copy(ROOT / "crates" / crate / name, root / "crates" / crate / name)
    return root


@needs_cargo
def test_main_passes_a_clean_workspace(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    core_io.main(pure_workspace(tmp_path))
    assert "only their allowlisted dependencies" in capsys.readouterr().out


@needs_cargo
def test_main_refuses_a_dependency_and_a_missing_guard(tmp_path: Path) -> None:
    root = pure_workspace(tmp_path, 'tokio = { path = "../tokio" }\n')
    (root / "crates/wayloop-schemas/clippy.toml").unlink()
    with pytest.raises(SystemExit) as refused:
        core_io.main(root)
    message = str(refused.value)
    assert "wayloop-core -> tokio: tokio is not on the allowlist of wayloop-core" in message
    assert "crates/wayloop-schemas: clippy.toml is missing" in message


@needs_cargo
def test_main_refuses_a_pure_crate_missing_from_the_workspace(tmp_path: Path) -> None:
    root = workspace(tmp_path, {"wayloop-core": ""})
    with pytest.raises(SystemExit, match="wayloop-schemas: cargo tree failed"):
        core_io.main(root)


@needs_cargo
def test_the_current_tree_passes(capsys: pytest.CaptureFixture[str]) -> None:
    core_io.main()
    assert "only their allowlisted dependencies" in capsys.readouterr().out
