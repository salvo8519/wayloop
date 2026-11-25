"""The naming conventions in scripts/conventions.py (docs/en/project/conventions.md)."""

import json
import re
import subprocess
import sys
from typing import Any

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
    assert waived and all(p.startswith("branch: ") for p in waived)
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
    ("sprint/S1/42-x", None, None),
    ("feat/42-Upper", None, None),
    ("feat/042-x", None, None),
    ("feat/42-one-two-three-four-five-six", None, None),
    ("feat/9007199254740992-x", None, None),
    ("feat/1-" + "x" * 94, None, None),
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


def test_bots_follow_the_naming_rules_and_skip_only_the_issue_link() -> None:
    # D3 of #267: no exemption by author for the title and the branch.
    title = "build(deps): bump serde from 1.0.1 to 1.0.2"
    assert (
        conventions.check_pull_request(title, "dependabot/cargo/x", "", [], "dependabot[bot]") == []
    )
    capital = "build(deps): Bump serde from 1.0.1 to 1.0.2 in the cargo-minor group across 1 dir"
    problems = conventions.check_pull_request(
        capital, "dependabot/cargo/x", "", [], "dependabot[bot]"
    )
    assert any("from 'a' to 'z'" in p for p in problems)
    assert any("limit is 72" in p for p in problems)
    assert not any("link the issue" in p for p in problems)


def test_commit_messages_written_by_git_are_exempt_by_their_form() -> None:
    for header in ("Merge branch 'main'", 'Revert "feat(core): add x"', "fixup! x", "squash! x"):
        assert conventions.check_commit_message(header + "\n") == [], header
    assert conventions.check_commit_message("amend! feat(core): add x\n")


def test_a_breaking_commit_needs_its_paragraph_or_footer() -> None:
    signed = "Signed-off-by: A <a@b.c>\n"
    header = "feat(core)!: drop the v1 envelope\n\n"
    assert any("BREAKING CHANGE" in p for p in conventions.check_commit_message(header + signed))
    footer = header + "Drops it.\n\nBREAKING CHANGE: clients read v2.\n" + signed
    assert conventions.check_commit_message(footer) == []


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


NAMING = ROOT / "crates" / "wayloop-core" / "src" / "naming"


def test_the_limits_and_the_generated_headers_of_the_core_are_those_of_the_conventions() -> None:
    # Deliberate copies (crates/wayloop-core/src/naming/): the core embeds no script.
    commit_rs = (NAMING / "commit.rs").read_text(encoding="utf-8")
    branch_rs = (NAMING / "branch.rs").read_text(encoding="utf-8")
    header = re.search(r"const HEADER_MAX_CHARS: usize = (\d+);", commit_rs)
    branch = re.search(r"const BRANCH_MAX_CHARS: usize = (\d+);", branch_rs)
    words = re.search(r"const SLUG_MAX_WORDS: usize = (\d+);", branch_rs)
    generated = re.search(r"const GENERATED: \[&str; (\d+)\] = \[(.*?)\];", commit_rs)
    breaking = re.search(r'const BREAKING_CHANGE: &str = "([^"]*)";', commit_rs)
    revert = re.search(r'const REVERT: &str = "([^"]*)";', commit_rs)
    assert header and branch and words and generated and breaking and revert
    assert int(header.group(1)) == conventions.MAX_HEADER
    assert int(branch.group(1)) == conventions.BRANCH_MAX_CHARS
    assert int(words.group(1)) == conventions.SLUG_MAX_WORDS
    starts = tuple(
        json.loads(f'"{m}"') for m in re.findall(r'"((?:[^"\\]|\\.)*)"', generated.group(2))
    )
    assert len(starts) == int(generated.group(1))
    assert starts == conventions.GENERATED
    assert breaking.group(1) == conventions.BREAKING_CHANGE_TEXT
    assert revert.group(1) == conventions.REVERT


def test_the_white_space_of_the_script_is_that_of_rust() -> None:
    # `char::is_whitespace` is Unicode White_Space; `str.isspace` adds U+001C to U+001F.
    isspace = {chr(c) for c in range(sys.maxunicode + 1) if chr(c).isspace()}
    assert set(conventions.WHITE_SPACE) == isspace - {"\x1c", "\x1d", "\x1e", "\x1f"}


# SPEC-0001 R11 in the engine and in the script (D2 of #267): the faults of the script, by the
# reason the engine gives each.
REASONS = {
    "form": "syntax",
    "start": "syntax",
    "full_stop": "syntax",
    "type": "enum",
    "scope": "enum",
    "no_scope": "missing",
    "length": "length",
    "words": "length",
    "requires": "requires",
}
CASES: dict[str, Any] = json.loads(
    (ROOT / "tests" / "process" / "naming_cases.json").read_text(encoding="utf-8")
)


def commit_reasons(header: str, body: str | None) -> list[str]:
    faults = conventions.commit_faults(header, body, conventions.TYPES, conventions.SCOPES)
    return [REASONS[f] for f in faults]


def branch_reasons(branch: str) -> list[str]:
    return [REASONS[f] for f in conventions.branch_faults(branch, conventions.TYPES)]


@pytest.mark.parametrize("case", CASES["commits"], ids=lambda c: c["header"][:40])
def test_each_shared_commit_case_gives_the_reasons_of_the_engine(case: dict[str, Any]) -> None:
    # The same cases are read by crates/wayloop-runtime/src/project/tests.rs.
    assert commit_reasons(case["header"], case.get("body")) == case["reasons"]


@pytest.mark.parametrize("case", CASES["branches"], ids=lambda c: c["branch"][:40])
def test_each_shared_branch_case_gives_the_reasons_of_the_engine(case: dict[str, Any]) -> None:
    assert branch_reasons(case["branch"]) == case["reasons"]


def _git(*args: str) -> str:
    return subprocess.run(
        ["git", *args], cwd=ROOT, capture_output=True, text=True, check=True
    ).stdout


def test_the_history_of_main_and_the_remote_branches_get_one_verdict_from_both() -> None:
    # Every commit of origin/main and every remote branch, through the script and through the
    # `wayloop validate` command of the engine: the two give the same reasons. A name both
    # refuse, such as an old branch without an issue, is history and stays refused.
    messages = [m for m in _git("log", "-z", "--format=%B", "origin/main").split("\0") if m]
    branches = [
        ref.removeprefix("origin/")
        for ref in _git(
            "for-each-ref", "--format=%(refname:short)", "refs/remotes/origin"
        ).splitlines()
        if ref not in ("origin", "origin/HEAD")
    ]
    assert messages and branches
    commits = []
    for message in messages:
        header, _, rest = message.partition("\n")
        body = rest.removeprefix("\n")
        commits.append((header, body or None))
    documents: list[dict[str, Any]] = [
        {
            "format": "document",
            "input_kind": "commit",
            "document": {"schema_version": 1, "header": h} | ({"body": b} if b else {}),
        }
        for h, b in commits
    ]
    documents += [
        {
            "format": "document",
            "input_kind": "pull_request",
            "document": {
                "schema_version": 1,
                "title": "feat(core): add x",
                "body": "",
                "branch": branch,
                "labels": [],
            },
        }
        for branch in branches
    ]
    run = subprocess.run(
        ["cargo", "run", "-q", "-p", "wayloop-cli", "--", "validate", "--input", "-"],
        cwd=ROOT,
        input=json.dumps({"documents": documents}),
        capture_output=True,
        text=True,
        check=False,
    )
    # Exit code 0 accepts the set and 1 refuses it; any other is a failure of the run.
    assert run.returncode in (0, 1), run.stderr
    envelope = json.loads(run.stdout)
    error = envelope.get("error") or {}
    assert error.get("reason") != "internal", envelope
    found: list[list[str]] = [[] for _ in documents]
    for item in error.get("errors", [error] if error else []):
        found[item["document"]].append(item["reason"])
    expected = [commit_reasons(h, b) for h, b in commits] + [branch_reasons(b) for b in branches]
    names = [h for h, _ in commits] + branches
    disagreements = [(n, f, e) for n, f, e in zip(names, found, expected, strict=True) if f != e]
    assert disagreements == []
