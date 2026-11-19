"""The naming conventions in scripts/conventions.py (docs/en/project/conventions.md)."""

import importlib.util
from pathlib import Path

import pytest

_spec = importlib.util.spec_from_file_location(
    "conventions", Path(__file__).resolve().parents[2] / "scripts" / "conventions.py"
)
assert _spec and _spec.loader
conventions = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(conventions)


@pytest.mark.parametrize(
    "header",
    [
        "feat(state): add the transition table",
        "fix(ledger): reject a broken hash chain [SPEC-0003 R4, R5]",
        "feat(cli)!: rename the next command",
        "revert: feat(state): add the transition table",
        "feat(tracker): read sub-issues into the cache [SPEC-0005 R1]",
    ],
)
def test_valid_headers(header: str) -> None:
    assert conventions.check_header(header) == []


@pytest.mark.parametrize(
    "header",
    [
        "Added guards",
        "feat: add something",
        "feature(state): add something",
        "feat(money): add something",
        "feat(state): Add something",
        "feat(state): add something.",
        "feat(state): " + "x" * 70,
    ],
)
def test_invalid_headers(header: str) -> None:
    assert conventions.check_header(header)


def test_pull_request_needs_matching_issue() -> None:
    ok = conventions.check_pull_request(
        "feat(state): add guards", "feat/42-guards", "Closes #42", [], "someone"
    )
    assert ok == []
    mismatch = conventions.check_pull_request(
        "feat(state): add guards", "feat/42-guards", "Closes #41", [], "someone"
    )
    assert any("#42" in p for p in mismatch)
    waived = conventions.check_pull_request(
        "docs(docs): fix a typo", "docs/typo", "", ["no-issue"], "someone"
    )
    assert any("branch" in p for p in waived) and not any("body" in p for p in waived)


def test_bots_only_need_a_valid_type_and_scope() -> None:
    title = (
        "build(deps): Bump serde from 1.0.1 to 1.0.2 in the cargo-minor group across 1 directory"
    )
    assert (
        conventions.check_pull_request(title, "dependabot/cargo/x", "", [], "dependabot[bot]") == []
    )


def test_commit_message_needs_sign_off() -> None:
    msg = "feat(core): add explain step\n\nBody.\n"
    assert any("Signed-off-by" in p for p in conventions.check_commit_message(msg))
    assert conventions.check_commit_message(msg + "\nSigned-off-by: A <a@example.org>\n") == []


def test_subject_without_a_full_stop() -> None:
    assert any("full stop" in p for p in conventions.check_header("feat(state): add guards."))


def test_commit_message_layout_and_sign_off() -> None:
    ok = "feat(state): add guards\n\nBody.\n\nSigned-off-by: A <a@b.c>\n"
    assert conventions.check_commit_message(ok) == []
    packed = "feat(state): add guards\nBody.\n\nSigned-off-by: A <a@b.c>\n"
    assert any("second line must be blank" in p for p in conventions.check_commit_message(packed))
    unsigned = "feat(state): add guards\n\nBody.\n"
    assert any("Signed-off-by" in p for p in conventions.check_commit_message(unsigned))


def test_breaking_title_needs_the_label_and_the_paragraph() -> None:
    title = "fix(core)!: rename a step"
    body = "Closes #42\n\nBREAKING CHANGE: the step is renamed."
    ok = conventions.check_pull_request(title, "fix/42-step", body, ["breaking-change"], "a")
    assert ok == []
    unlabelled = conventions.check_pull_request(title, "fix/42-step", body, [], "a")
    assert any("breaking-change" in p for p in unlabelled)
    silent = conventions.check_pull_request(
        title, "fix/42-step", "Closes #42", ["breaking-change"], "a"
    )
    assert any("BREAKING CHANGE:" in p for p in silent)


def test_breaking_label_needs_the_marker_in_the_title() -> None:
    body = "Closes #42\n\nBREAKING CHANGE: the step is renamed."
    problems = conventions.check_pull_request(
        "fix(core): rename a step", "fix/42-step", body, ["breaking-change"], "a"
    )
    assert any("!" in p for p in problems)
