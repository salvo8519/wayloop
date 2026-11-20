"""Contract snapshots: generated from the types, their history kept per contract (ADR 0030 G5, G6).

`wayloop-schemagen` prints the snapshots of the current version of every contract of the
registry (`wayloop_schemas::contract::CONTRACTS`), with their text, written only by
`contract::snapshot`. This script writes those bytes unchanged and keeps every other snapshot
rule:

    schemas/<area>/<name>/v<N>.json         the schema of version N of a contract
    schemas/<area>/<name>/v<N>.output.json  the output of a command: its envelope
    schemas/<area>/<name>/v<N>.input.json   the input of a document handed to the engine
    schemas/<area>/<name>/index.json        the history: `format`, `current` and, per version,
                                            the files, their SHA-256 and `introduced_in`

Every snapshot of every version is also copied, byte for byte, into
`crates/wayloop-schemas/snapshots/`, and `crates/wayloop-schemas/src/contract/embedded.rs`
embeds the copies with `include_str!`: a crate embeds only files inside its own folder, so that
`cargo package` works (ADR 0030, amendment of 2026-11-20).

Rules (ADR 0030 G5, rule B): a version earlier than the current one never changes; the current
version may change only additively under ADR 0009 C5; any other change is a new version
v<N+1>; the current version never goes back, versions run from 1 without a gap, and a contract
is never removed. A version that is not yet on the base branch may change freely.

    python scripts/schema_snapshots.py                      # write snapshots, histories, copies
    python scripts/schema_snapshots.py --check              # fail when a file differs
    python scripts/schema_snapshots.py --diff-against BASE  # classify changes against BASE

`--diff-against` compares with the merge base of BASE and HEAD (the tip of BASE without one):
it refuses a rewritten history (an earlier version changed, a file deleted or replaced, a
version or a contract removed, a current version gone back) and a breaking change of the
current version.

Breaking for an input: a member removed, a member newly required, a value removed from an
`enum`, any other keyword changed (type, format, bounds, `$ref`, `additionalProperties`,
`allOf`, ...). Breaking for an output: a member removed or no longer required, a value removed
from an `enum`, any other keyword changed (`additionalProperties` added closes an open output).
The same rules apply inside `items` and `$defs`; a definition removed is breaking. Additive: a
new optional input member, a new output member, a new `enum` value (clients accept values they
do not know), a new definition, and any change of `description`, `title` or `$comment`.

Standard library only: the hook .claude/hooks/protect_contracts.py imports SNAPSHOT from here.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import stat
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import _git

ROOT = Path(__file__).resolve().parents[1]

# The hook .claude/hooks/protect_contracts.py imports this pattern.
SNAPSHOT = re.compile(r"(^|/)schemas/(.+/)?v(?P<version>\d+)(\.output|\.input)?\.json$")

SCHEMAS = "schemas"
# Copies embedded by wayloop-schemas, and the generated list that embeds them.
COPIES = "crates/wayloop-schemas/snapshots"
EMBEDDED_RS = "crates/wayloop-schemas/src/contract/embedded.rs"
MANIFEST = "index.json"
FORMAT = 1
UNRELEASED = "unreleased"
GENERATOR = ("cargo", "run", "--quiet", "--locked", "--package", "wayloop-schemagen")
# Keys a classification ignores (documentation), and those it compares member by member.
ANNOTATIONS = frozenset({"$comment", "$id", "description", "title"})
STRUCTURE = frozenset({"$defs", "enum", "items", "properties", "required"})


@dataclass(frozen=True)
class File:
    """One generated file of the current version of a contract."""

    part: str
    direction: str
    path: str
    text: str


@dataclass(frozen=True)
class Contract:
    """A contract of the registry, with the files of its current version."""

    area: str
    name: str
    version: int
    files: tuple[File, ...]

    @property
    def folder(self) -> str:
        return f"{self.area}/{self.name}"


def parse(document: dict[str, Any]) -> list[Contract]:
    """The contracts of the document printed by wayloop-schemagen.

    The generator also prints every part of a contract with the name of its file at version 1:
    the registry is the source of the spellings, and a part whose files SNAPSHOT does not
    recognise would escape the hook, so it stops the script here.
    """
    for part in document["parts"]:
        if not SNAPSHOT.search(f"{SCHEMAS}/area/name/{part['first_file']}"):
            sys.exit(
                f"schema_snapshots: part {part['part']}: {part['first_file']} is not recognised "
                "by SNAPSHOT; extend SNAPSHOT in scripts/schema_snapshots.py and the hook "
                "fallback in .claude/hooks/protect_contracts.py"
            )
    return [
        Contract(
            area=c["area"],
            name=c["name"],
            version=int(c["version"]),
            files=tuple(
                File(part=f["part"], direction=f["direction"], path=f["path"], text=f["text"])
                for f in c["files"]
            ),
        )
        for c in document["contracts"]
    ]


def generated(root: Path = ROOT) -> list[Contract]:
    """The contracts printed by wayloop-schemagen, built from the workspace at `root`."""
    out = subprocess.run(
        GENERATOR, cwd=root, capture_output=True, text=True, encoding="utf-8", check=False
    )
    if out.returncode != 0:
        sys.exit(f"schema_snapshots: wayloop-schemagen failed:\n{out.stderr.strip()}")
    return parse(json.loads(out.stdout))


def keys(part: str) -> tuple[str, str]:
    """The keys of the file and of its hash in a version record (the record shape of Fairmark)."""
    return ("file", "sha256") if part == "schema" else (f"{part}_file", f"{part}_sha256")


def files_of(record: dict[str, Any]) -> list[tuple[str, str]]:
    """(file, hash) of every file of a version record."""
    return [
        (path, record.get(key.removesuffix("file") + "sha256", ""))
        for key, path in sorted(record.items())
        if key == "file" or key.endswith("_file")
    ]


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def manifest_path(folder: str) -> str:
    return f"{SCHEMAS}/{folder}/{MANIFEST}"


def read_manifest(root: Path, folder: str) -> dict[str, Any] | None:
    path = root / manifest_path(folder)
    if not path.is_file():
        return None
    manifest: dict[str, Any] = json.loads(path.read_text(encoding="utf-8"))
    return manifest


def manifest_text(manifest: dict[str, Any]) -> str:
    """Keys in byte order, versions in numeric order (v10 after v9), two-space indentation."""
    versions = manifest.get("versions", {})
    ordered = {
        **{k: v for k, v in sorted(manifest.items()) if k != "versions"},
        "versions": {
            k: dict(sorted(versions[k].items())) for k in sorted(versions, key=_version_key)
        },
    }
    return json.dumps(ordered, indent=2, ensure_ascii=False) + "\n"


def _version_key(version: str) -> tuple[int, str]:
    return (int(version), version) if version.isdigit() else (0, version)


EMBEDDED_HEAD = """\
//! Generated by `scripts/schema_snapshots.py` (`uv run just generate`): do not edit.
//!
//! The text of every committed snapshot under `schemas/`, of every version. Each is embedded
//! from its copy in `crates/wayloop-schemas/snapshots/`, inside this crate, so that `cargo
//! package` works (ADR 0030, amendment of 2026-11-20); `just check` fails when a copy differs
//! from its source.

use super::Embedded;

/// Every committed snapshot, of every version, in the byte order of their paths.
"""


def embedded_rs(paths: list[str]) -> str:
    """The Rust list that embeds the copy of every snapshot, in the byte order of the paths,
    formatted by rustfmt as `cargo fmt` formats the crate."""
    entries = "".join(
        f'Embedded {{ path: "{p}", text: include_str!("../../snapshots/{p}") }},' for p in paths
    )
    source = f"{EMBEDDED_HEAD}pub const EMBEDDED: &[Embedded] = &[{entries}];\n"
    import tomllib  # Python 3.11 and later; the hook, which imports this file, never calls it

    manifest = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    edition = manifest["workspace"]["package"]["edition"]
    out = subprocess.run(
        ["rustfmt", "--edition", edition, "--emit", "stdout"],
        input=source,
        cwd=ROOT,
        capture_output=True,
        text=True,
        encoding="utf-8",
        check=False,
    )
    if out.returncode != 0:
        sys.exit(f"schema_snapshots: rustfmt failed:\n{out.stderr.strip()}")
    return out.stdout


def expected(root: Path, contracts: list[Contract]) -> tuple[dict[str, bytes], list[str]]:
    """Every file that generation writes, by path from `root`, and the reasons it refuses to.

    The snapshots and the history of each current version come from `contracts`; the copies
    and the Rust list cover every version recorded in the histories.
    """
    out: dict[str, bytes] = {}
    refused: list[str] = []
    recorded: list[str] = []
    for contract in contracts:
        old = read_manifest(root, contract.folder) or {}
        versions: dict[str, Any] = dict(old.get("versions", {}))
        current = int(old.get("current", 0))
        if contract.version < current:
            refused.append(
                f"{contract.folder}: the current version would go back from v{current} to "
                f"v{contract.version}, and an earlier version would be rewritten"
            )
            continue
        if contract.version > current + 1:
            refused.append(
                f"{contract.folder}: v{contract.version} would follow v{current}, leaving a gap"
            )
            continue
        key = str(contract.version)
        record: dict[str, Any] = {
            "introduced_in": versions.get(key, {}).get("introduced_in", UNRELEASED)
        }
        unknown = [f for f in contract.files if not SNAPSHOT.search(f"{SCHEMAS}/{f.path}")]
        if unknown:
            refused += [
                f"{SCHEMAS}/{f.path}: a file name SNAPSHOT does not recognise; extend SNAPSHOT in "
                "scripts/schema_snapshots.py and the hook fallback"
                for f in unknown
            ]
            continue
        for file in contract.files:
            data = file.text.encode("utf-8")
            out[f"{SCHEMAS}/{file.path}"] = data
            file_key, hash_key = keys(file.part)
            record[file_key] = file.path
            record[hash_key] = sha256(data)
        versions[key] = record
        manifest = {"current": key, "format": FORMAT, "versions": versions}
        out[manifest_path(contract.folder)] = manifest_text(manifest).encode("utf-8")
        recorded += [path for record in versions.values() for path, _ in files_of(record)]
    for path in sorted(set(recorded)):
        source = f"{SCHEMAS}/{path}"
        if source in out:
            out[f"{COPIES}/{path}"] = out[source]
        elif (root / source).is_file():
            out[f"{COPIES}/{path}"] = (root / source).read_bytes()
        else:
            refused.append(f"{source}: recorded in its history and missing")
    out[EMBEDDED_RS] = embedded_rs(sorted(set(recorded))).encode("utf-8")
    return out, refused


def _files_under(root: Path, folder: str) -> list[str]:
    base = root / folder
    if not base.exists():
        return []
    return sorted(
        p.relative_to(root).as_posix() for p in base.rglob("*") if p.is_file() or p.is_symlink()
    )


def strays(root: Path, contracts: list[Contract], out: dict[str, bytes]) -> list[str]:
    """Files under `schemas/` and among the copies that generation neither writes nor records,
    and folders of contracts the registry does not list."""
    folders = {c.folder for c in contracts}
    found: list[str] = []
    committed = _files_under(root, SCHEMAS)
    for path in committed:
        folder = path.removeprefix(f"{SCHEMAS}/").rpartition("/")[0]
        if folder not in folders:
            found.append(f"{path}: not in a folder of a contract of the registry")
    known = set(out)
    for contract in contracts:
        for record in (read_manifest(root, contract.folder) or {}).get("versions", {}).values():
            known |= {f"{SCHEMAS}/{path}" for path, _ in files_of(record)}
    found += [
        f"{path}: neither generated nor recorded in a history"
        for path in committed
        if path not in known and path.removeprefix(f"{SCHEMAS}/").rpartition("/")[0] in folders
    ]
    found += [
        f"{path}: a copy of no snapshot" for path in _files_under(root, COPIES) if path not in out
    ]
    return found


def recorded_problems(root: Path, contracts: list[Contract]) -> list[str]:
    """Every file recorded in a history is a regular file with its recorded hash, and the
    versions of a history run from 1 to `current` without a gap."""
    found: list[str] = []
    for contract in contracts:
        manifest = read_manifest(root, contract.folder)
        if manifest is None:
            continue
        versions = manifest.get("versions", {})
        expected_keys = [str(v) for v in range(1, int(manifest.get("current", 0)) + 1)]
        if sorted(versions, key=_version_key) != expected_keys:
            found.append(
                f"{manifest_path(contract.folder)}: versions {', '.join(versions)} do not run "
                f"from 1 to {manifest.get('current')}"
            )
        for version, record in versions.items():
            found += [
                f"{manifest_path(contract.folder)}: v{version} has the unknown key {key}"
                for key in sorted(record)
                if key != "introduced_in"
                and key != "file"
                and not key.endswith("_file")
                and not (_is_hash(key) and key.removesuffix("sha256") + "file" in record)
            ]
            for path, digest in files_of(record):
                file = root / SCHEMAS / path
                if file.is_symlink() or not file.is_file():
                    found.append(f"{SCHEMAS}/{path}: recorded in its history, not a regular file")
                elif sha256(file.read_bytes()) != digest:
                    found.append(f"{SCHEMAS}/{path}: its hash differs from its history")
    return found


def check(root: Path, contracts: list[Contract]) -> list[str]:
    """Every difference between the committed files and the generated ones."""
    out, refused = expected(root, contracts)
    found = list(refused)
    for path, data in sorted(out.items()):
        file = root / path
        if not file.is_file():
            found.append(f"{path}: missing")
        elif file.read_bytes() != data:
            found.append(f"{path}: differs from the generated file")
    return found + strays(root, contracts, out) + recorded_problems(root, contracts)


def write(root: Path, contracts: list[Contract]) -> list[str]:
    """Writes every generated file and removes copies of no snapshot; nothing is written when
    generation refuses. Returns the refusals and the stray files under `schemas/`."""
    out, refused = expected(root, contracts)
    if refused:
        return refused
    for path, data in out.items():
        file = root / path
        file.parent.mkdir(parents=True, exist_ok=True)
        if not file.is_file() or file.read_bytes() != data:
            file.write_bytes(data)
    for path in _files_under(root, COPIES):
        if path not in out:
            (root / path).unlink()
    return strays(root, contracts, out)


# ---- the history against a base


def released(root: Path, base: str) -> dict[str, tuple[str, str]]:
    """Files under `schemas/` at the merge base of `base` and HEAD, mapped to their mode and
    blob id.

    The merge base keeps a branch behind `base` from reporting as deleted a snapshot released
    after it forked. Without a merge base (unrelated history, a shallow clone) the tip of
    `base` is used.
    """
    not_a_commit = f"schema_snapshots: base '{base}' is not a commit; fetch it or pass another."
    try:
        _git.revision(base)
    except ValueError:
        sys.exit(not_a_commit)
    if _git.run("rev-parse", "--verify", "--quiet", f"{base}^{{commit}}", cwd=root).returncode != 0:
        sys.exit(not_a_commit)
    fork = _git.run("merge-base", "--end-of-options", base, "HEAD", cwd=root)
    tree = fork.stdout.strip() if fork.returncode == 0 else base
    listing = _git.run("ls-tree", "-r", "-z", "--end-of-options", tree, "--", SCHEMAS, cwd=root)
    if listing.returncode != 0:
        sys.exit(f"schema_snapshots: git ls-tree failed: {listing.stderr.strip()}")
    entries: dict[str, tuple[str, str]] = {}
    for entry in listing.stdout.split("\0"):
        if not entry:
            continue
        meta, _, path = entry.partition("\t")
        mode, _, blob = meta.split()
        entries[path] = (mode, blob)
    return entries


def _blob(root: Path, blob: str) -> bytes:
    out = subprocess.run(
        ["git", "cat-file", "blob", blob], cwd=root, capture_output=True, check=False
    )
    if out.returncode != 0:
        sys.exit(f"schema_snapshots: git cat-file failed: {out.stderr.decode().strip()}")
    return out.stdout


def _mode(file: Path) -> str | None:
    """The git mode of a file in the working tree, or None when it is missing or a folder."""
    try:
        info = file.lstat()
    except FileNotFoundError:
        return None
    if stat.S_ISLNK(info.st_mode):
        return "120000"
    if stat.S_ISREG(info.st_mode):
        return "100755" if info.st_mode & stat.S_IXUSR else "100644"
    return None


def breaking(where: str, old: Any, new: Any, output: bool) -> list[str]:
    """The breaking differences from `old` to `new`, two (sub)schemas, each at its pointer."""
    if not isinstance(old, dict) or not isinstance(new, dict):
        return [] if old == new else [f"{where or '/'}: changed"]
    found: list[str] = []
    old_members, new_members = old.get("properties", {}), new.get("properties", {})
    for member in sorted(old_members.keys() - new_members.keys()):
        found.append(f"{where}/properties/{member}: member removed")
    for member in sorted(old_members.keys() & new_members.keys()):
        found += breaking(
            f"{where}/properties/{member}", old_members[member], new_members[member], output
        )
    old_required, new_required = set(old.get("required", [])), set(new.get("required", []))
    if output:
        found += [
            f"{where}/properties/{m}: may now be absent"
            for m in sorted((old_required - new_required) & new_members.keys())
        ]
    else:
        found += [
            f"{where}/properties/{m}: now required" for m in sorted(new_required - old_required)
        ]
    if "items" in old or "items" in new:
        found += breaking(f"{where}/items", old.get("items"), new.get("items"), output)
    old_defs, new_defs = old.get("$defs", {}), new.get("$defs", {})
    for name in sorted(old_defs):
        if name in new_defs:
            found += breaking(f"{where}/$defs/{name}", old_defs[name], new_defs[name], output)
        else:
            found.append(f"{where}/$defs/{name}: definition removed")
    old_values, new_values = old.get("enum"), new.get("enum")
    if old_values != new_values:
        if not isinstance(old_values, list) or not isinstance(new_values, list):
            found.append(f"{where or '/'}: `enum` changed")
        else:
            kept = {json.dumps(v) for v in new_values}
            found += [
                f"{where or '/'}: value {json.dumps(v)} removed"
                for v in old_values
                if json.dumps(v) not in kept
            ]
    for key in sorted((old.keys() | new.keys()) - ANNOTATIONS - STRUCTURE):
        if old.get(key) != new.get(key):
            found.append(f"{where or '/'}: `{key}` changed")
    return found


def _is_hash(key: str) -> bool:
    return key == "sha256" or key.endswith("_sha256")


def _record_kept(record: dict[str, Any], current: bool) -> dict[str, Any]:
    """The part of a version record that never changes once on the base branch."""
    return {
        k: v for k, v in record.items() if k != "introduced_in" and not (current and _is_hash(k))
    }


def diff_against(root: Path, base: str, contracts: list[Contract]) -> tuple[list[str], list[str]]:
    """(history rewritten, breaking changes of a current version) against `base`."""
    from roadmap import SEMVER  # the project environment only: the hook never calls this

    entries = released(root, base)
    rewritten: list[str] = []
    found: list[str] = []
    currents: dict[str, tuple[int, int]] = {}
    # Every file a history of the base records, with its contract and version: released
    # whatever its name, so that a part the pattern misses is protected too.
    recorded: dict[str, tuple[str, int]] = {}
    for path in sorted(p for p in entries if p.endswith(f"/{MANIFEST}")):
        folder = path.removeprefix(f"{SCHEMAS}/").removesuffix(f"/{MANIFEST}")
        old: dict[str, Any] = json.loads(_blob(root, entries[path][1]))
        new = read_manifest(root, folder)
        if new is None:
            rewritten.append(f"{folder}: the contract was removed")
            continue
        currents[folder] = (int(old["current"]), int(new["current"]))
        if int(new["current"]) < int(old["current"]):
            rewritten.append(
                f"{folder}: the current version went back from v{old['current']} to "
                f"v{new['current']}"
            )
        for version, record in old.get("versions", {}).items():
            now = new.get("versions", {}).get(version)
            if now is None:
                rewritten.append(f"{folder}: v{version} removed from its history")
                continue
            recorded |= {f"{SCHEMAS}/{p}": (folder, int(version)) for p, _ in files_of(record)}
            before, after = record.get("introduced_in"), now.get("introduced_in")
            if after != before and not (before == UNRELEASED and SEMVER.match(str(after))):
                rewritten.append(f"{folder}: introduced_in of v{version} changed to {after}")
            # The whole record stays, but for introduced_in above and, of a version current on
            # both sides, the hashes of its files, which an additive change moves.
            current = version == str(old["current"]) == str(new["current"])
            if _record_kept(record, current) != _record_kept(now, current):
                rewritten.append(f"{folder}: the record of v{version} changed in its history")
    directions = {f"{SCHEMAS}/{f.path}": f.direction for c in contracts for f in c.files}
    for path, (mode, blob) in sorted(entries.items()):
        match = SNAPSHOT.search(path)
        if path in recorded:
            folder, version = recorded[path]
        elif match is not None:
            folder, version = (
                path.removeprefix(f"{SCHEMAS}/").rpartition("/")[0],
                int(match["version"]),
            )
        else:
            continue
        local = _mode(root / path)
        if local is None:
            rewritten.append(f"{path}: released on {base} and deleted")
            continue
        if local == "120000":
            rewritten.append(f"{path}: released on {base} and replaced by a symlink")
            continue
        if local != mode:
            rewritten.append(f"{path}: released on {base} and its mode changed ({mode} to {local})")
            continue
        before_bytes, after_bytes = _blob(root, blob), (root / path).read_bytes()
        if before_bytes == after_bytes:
            continue
        if currents.get(folder) != (version, version) or path not in directions:
            rewritten.append(f"{path}: released on {base} as an earlier version, and changed")
            continue
        found += [
            f"{path}#{problem}"
            for problem in breaking(
                "",
                json.loads(before_bytes),
                json.loads(after_bytes),
                output=directions[path] == "output",
            )
        ]
    return rewritten, found


def schema_history(root: Path = ROOT) -> dict[str, Any]:
    """The history of every contract, assembled from the manifests under `schemas/`: the view
    the documentation build publishes as `schemas/index.json`."""
    contracts: dict[str, Any] = {}
    for path in sorted((root / SCHEMAS).glob(f"*/*/{MANIFEST}")):
        folder = path.parent.relative_to(root / SCHEMAS).as_posix()
        manifest = json.loads(path.read_text(encoding="utf-8"))
        contracts[folder] = {k: v for k, v in manifest.items() if k != "format"}
    return {"contracts": contracts, "format": FORMAT}


def _report(title: str, lines: list[str]) -> str:
    return title + ":\n  " + "\n  ".join(lines)


def main(argv: list[str] | None = None, root: Path = ROOT) -> None:
    parser = argparse.ArgumentParser(description=(__doc__ or "").split("\n", 1)[0])
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--check", action="store_true", help="fail when a file differs")
    group.add_argument("--diff-against", metavar="BASE", help="classify changes against BASE")
    args = parser.parse_args(argv)
    contracts = generated(root)
    if args.diff_against:
        rewritten, found = diff_against(root, args.diff_against, contracts)
        problems = []
        if rewritten:
            problems.append(
                _report(
                    "Schema history rewritten: earlier versions and contracts never change",
                    rewritten,
                )
            )
        if found:
            problems.append(
                _report(
                    "Breaking changes of a current version: write the next version v<N+1> "
                    "(ADR 0030 G5, ADR 0009 C5)",
                    found,
                )
            )
        if problems:
            sys.exit("\n".join(problems))
        print(f"Schema snapshots: changes against {args.diff_against} are additive or none.")
        return
    if args.check:
        found = check(root, contracts)
        if found:
            sys.exit(
                _report("Schema snapshots differ from the types", found)
                + "\nRun: uv run just generate"
            )
        print(f"Schema snapshots up to date: every contract of the registry ({len(contracts)}).")
        return
    found = write(root, contracts)
    if found:
        sys.exit(_report("Schema snapshots not written, or stray files", found))
    print(f"Schema snapshots written: every contract of the registry ({len(contracts)}).")


if __name__ == "__main__":
    main()
