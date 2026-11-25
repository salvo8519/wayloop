"""The naming conventions in scripts/conventions.py (docs/en/project/conventions.md)."""

import re

import conventions
import pytest
from _labels import AREA_PREFIX
from conftest import ROOT

import roadmap


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


BRANCHES = [
    # (branch, issue, sprint)
    ("feat/42-ledger-hash-chain", 42, None),
    ("fix/61-replay-order", 61, None),
    ("docs/48-mcp-guide", 48, None),
    ("sprint/S01/42-ledger-hash", 42, "S01"),
    ("sprint/S12/7-x", 7, "S12"),
    ("main", None, None),
    ("dependabot/cargo/serde-1.0.200", None, None),
    ("release/v0.1.0", None, None),
    ("revert-57-feat/42-x", None, None),
    ("feat/ledger", None, None),
    ("sprint/01/42-x", None, None),
    ("feat/42-Upper", None, None),
]


@pytest.mark.parametrize(("branch", "issue", "sprint"), BRANCHES)
def test_branch_issue_and_sprint_follow_both_forms(
    branch: str, issue: int | None, sprint: str | None
) -> None:
    assert conventions.branch_issue(branch) == issue
    assert conventions.sprint_of(branch) == sprint


def test_a_sprint_branch_is_a_valid_pull_request_branch() -> None:
    problems = conventions.check_pull_request(
        "feat(ledger): chain events by hash", "sprint/S01/42-ledger-hash", "Closes #42", [], "a"
    )
    assert problems == []
    mismatch = conventions.check_pull_request(
        "feat(ledger): chain events by hash", "sprint/S01/42-ledger-hash", "Closes #41", [], "a"
    )
    assert any("#42" in p for p in mismatch)


def test_closing_keywords_match_github_anywhere_in_the_body() -> None:
    body = (
        "Closes #12\nfixes #7\n  Resolves #3\nSee #99\nRelated: closes #5 later\n"
        "Close #20, closed #21, fix #22, fixed #23, resolve #24, resolved #25.\n"
        "Fixes: #26\nprefixes #27\nRefs #28\nCloses#29"
    )
    assert conventions.closed_issues(body) == {12, 7, 3, 5, 20, 21, 22, 23, 24, 25, 26}
    assert conventions.linked_issues(body) == {12, 7, 3, 5, 20, 21, 22, 23, 24, 25, 26, 28}
    assert conventions.linked_issues("Refs: #42\nref #43") == {42, 43}
    assert conventions.closed_issues("") == set()


def test_a_fix_keyword_links_the_branch_issue() -> None:
    assert (
        conventions.check_pull_request(
            "fix(ci): repair the gate", "fix/12-gate", "Some text. Fix #12", [], "a"
        )
        == []
    )


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


# The default policy of the core (SPEC-0001 R21, SPEC-0021 R1) is a deliberate copy of the
# policy of this repository, since the crate embeds no file outside itself: these tests keep it
# equal to its sources.
POLICY_RS = (ROOT / "crates" / "wayloop-core" / "src" / "policy.rs").read_text(encoding="utf-8")


def default_policy_list(name: str) -> tuple[str, ...]:
    """The names of the constant `name` of crates/wayloop-core/src/policy.rs, in order."""
    match = re.search(rf"const {name}: \[&str; (\d+)\] = \[(.*?)\];", POLICY_RS, re.DOTALL)
    assert match, name
    names = tuple(re.findall(r'"([^"]*)"', match.group(2)))
    assert len(names) == int(match.group(1)), name
    return names


def test_the_default_policy_of_the_core_has_the_types_and_scopes_of_the_conventions() -> None:
    assert default_policy_list("TYPES") == conventions.TYPES
    assert default_policy_list("SCOPES") == conventions.SCOPES


def test_the_default_policy_of_the_core_has_the_areas_of_the_roadmap_labels() -> None:
    labels = [label["name"] for label in roadmap.load()["labels"]]
    areas = tuple(name.removeprefix(AREA_PREFIX) for name in labels if name.startswith(AREA_PREFIX))
    assert default_policy_list("AREAS") == areas


def test_the_header_limit_of_the_core_is_that_of_the_conventions() -> None:
    # A deliberate copy (crates/wayloop-core/src/naming/commit.rs): the core embeds no script.
    commit_rs = (ROOT / "crates" / "wayloop-core" / "src" / "naming" / "commit.rs").read_text(
        encoding="utf-8"
    )
    match = re.search(r"const HEADER_MAX_CHARS: usize = (\d+);", commit_rs)
    assert match
    assert int(match.group(1)) == conventions.MAX_HEADER
