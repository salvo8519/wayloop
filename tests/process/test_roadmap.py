"""Rules of the roadmap file (scripts/roadmap.py), offline."""

from __future__ import annotations

import copy
import importlib.util
from pathlib import Path
from typing import Any

import pytest

ROOT = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location("roadmap", ROOT / "scripts/roadmap.py")
assert _spec and _spec.loader
roadmap = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(roadmap)


def data() -> dict[str, Any]:
    return copy.deepcopy(roadmap.load())


def test_committed_roadmap_is_consistent() -> None:
    assert roadmap.problems_of(data()) == []


def test_every_task_needs_a_unique_issue() -> None:
    d = data()
    first = d["milestones"][0]["epics"][0]["tasks"]
    first[1]["issue"] = first[0]["issue"]
    assert any("appears twice" in p for p in roadmap.problems_of(d))
    del first[0]["issue"]
    assert any("needs an issue" in p for p in roadmap.problems_of(d))


def test_specs_must_exist_and_be_referenced() -> None:
    d = data()
    task = d["milestones"][0]["epics"][0]["tasks"][0]
    task["spec"] = "9999"
    assert any("spec 9999 does not exist" in p for p in roadmap.problems_of(d))
    for m in d["milestones"]:
        for e in m["epics"]:
            for t in e["tasks"]:
                if t.get("spec") == "0008":
                    del t["spec"]
    assert any("spec 0008 is not referenced" in p for p in roadmap.problems_of(d))


def test_unknown_status_and_keys_are_refused() -> None:
    d = data()
    task = d["milestones"][0]["epics"][0]["tasks"][0]
    task["status"] = "almost"
    task["owner"] = "someone"
    problems = roadmap.problems_of(d)
    assert any("status must be" in p for p in problems)
    assert any("unknown keys" in p for p in problems)


def test_a_pull_request_must_close_a_planned_issue() -> None:
    d = data()
    task = d["milestones"][0]["epics"][0]["tasks"][0]
    planned = task["issue"]
    task["status"] = "open"
    assert any("mark its task done" in p for p in roadmap.problems_of(d, f"Closes #{planned}."))
    task["status"] = "done"
    problems = roadmap.problems_of(d, f"Summary.\n\nCloses #{planned}.")
    assert not [p for p in problems if "closes" in p]
    assert any("#99999" in p for p in roadmap.problems_of(d, "Closes #99999"))


def test_dropped_work_stays_visible_and_is_not_counted() -> None:
    d = data()
    task = d["milestones"][0]["epics"][0]["tasks"][0]
    task["status"] = "dropped"
    block = roadmap.page_block(d)
    assert "| Dropped |" in block
    assert not [p for p in roadmap.problems_of(d) if "status" in p]


def test_the_page_includes_the_generated_tables() -> None:
    """The tables are generated at build time; a change to the plan needs no page change."""
    d = data()
    d["milestones"][0]["gate"] = "A different gate."
    assert not [p for p in roadmap.problems_of(d) if "roadmap.md" in p]
    assert roadmap.INCLUDE in roadmap.PAGE.read_text(encoding="utf-8")


def test_a_page_without_the_include_is_refused(monkeypatch, tmp_path) -> None:  # type: ignore[no-untyped-def]
    page = tmp_path / "roadmap.md"
    page.write_text("# Roadmap\n\nTables pasted by hand.\n", encoding="utf-8")
    monkeypatch.setattr(roadmap, "PAGE", page)
    assert any("must include the generated tables" in p for p in roadmap.problems_of(data()))


def test_write_page_produces_the_tables(monkeypatch, tmp_path) -> None:  # type: ignore[no-untyped-def]
    target = tmp_path / ".generated" / "ROADMAP.md"
    monkeypatch.setattr(roadmap, "GENERATED_PAGE", target)
    d = data()
    roadmap.write_page(d)
    assert target.read_text(encoding="utf-8") == roadmap.page_block(d) + "\n"


# ---- sync, offline: GitHub issues are given as data


def github_view(d: dict[str, Any]) -> dict[int, dict[str, Any]]:
    """Issues as GitHub would report them if it agreed with the roadmap `d`."""
    issues: dict[int, dict[str, Any]] = {}
    for m, _e, t in roadmap.tasks(d):
        status = t.get("status", "open")
        issues[t["issue"]] = {
            "number": t["issue"],
            "title": t["title"],
            "state": "open" if status == "open" else "closed",
            "state_reason": "not_planned" if status == "dropped" else "completed",
            "milestone": {"title": m["title"]},
        }
    return issues


def test_sync_agrees_with_a_matching_github() -> None:
    d = data()
    drift, untriaged = roadmap.differences(copy.deepcopy(d), github_view(d))
    assert drift == [] and untriaged == []


def test_new_issue_is_pending_while_its_pull_request_is_open() -> None:
    main = data()
    branch = copy.deepcopy(main)
    epic = branch["milestones"][0]["epics"][0]
    epic["tasks"].append({"issue": 99999, "title": "New task", "status": "open"})
    issues = github_view(branch)
    drift, _ = roadmap.differences(copy.deepcopy(main), issues)
    assert [n for n, _ in drift] == [99999]
    assert roadmap.pending_in(drift, {7: (branch, set())}, issues) == {99999: 7}
    # Without the pull request the difference stands.
    assert roadmap.pending_in(drift, {}, issues) == {}


def test_a_pull_request_that_does_not_resolve_leaves_the_difference() -> None:
    main = data()
    issues = github_view(main)
    task = main["milestones"][0]["epics"][0]["tasks"][0]
    issues[task["issue"]]["state"] = "open" if task.get("status") != "open" else "closed"
    drift, _ = roadmap.differences(copy.deepcopy(main), issues)
    assert [n for n, _ in drift] == [task["issue"]]
    unrelated = copy.deepcopy(main)
    assert roadmap.pending_in(drift, {8: (unrelated, set())}, issues) == {}


def test_issue_closed_by_an_open_pull_request_is_pending() -> None:
    # The pull request adds the task as done (check --pr-body requires it); the issue is still
    # open on GitHub until the merge closes it.
    main = data()
    branch = copy.deepcopy(main)
    branch["milestones"][0]["epics"][0]["tasks"].append(
        {"issue": 99998, "title": "Closed by the pull request", "status": "done"}
    )
    issues = github_view(branch)
    issues[99998]["state"] = "open"
    drift, _ = roadmap.differences(copy.deepcopy(main), issues)
    assert [n for n, _ in drift] == [99998]
    assert roadmap.pending_in(drift, {9: (branch, {99998})}, issues) == {99998: 9}
    # A pull request that does not close the issue leaves the difference.
    assert roadmap.pending_in(drift, {9: (branch, set())}, issues) == {}


def test_closed_by_reads_the_closing_keywords() -> None:
    body = "Closes #12\nfixes #7\n  Resolves #3\nSee #99\nRelated: closes #5 later"
    assert roadmap.closed_by(body) == {12, 7, 3}


def test_releases_must_be_semver_and_not_decrease() -> None:
    d = data()
    d["milestones"][0]["release"] = "next"
    assert any("release X.Y.Z" in p for p in roadmap.problems_of(d))
    d = data()
    d["milestones"][0]["release"], d["milestones"][1]["release"] = (
        d["milestones"][1]["release"],
        d["milestones"][0]["release"],
    )
    assert any("must not decrease" in p for p in roadmap.problems_of(d))


def test_the_file_must_be_in_canonical_form(monkeypatch, tmp_path) -> None:  # type: ignore[no-untyped-def]
    copy_of = tmp_path / "roadmap.yaml"
    copy_of.write_text(roadmap.ROADMAP.read_text(encoding="utf-8") + "\n# note\n", encoding="utf-8")
    monkeypatch.setattr(roadmap, "ROADMAP", copy_of)
    assert any("canonical form" in p for p in roadmap.problems_of(data()))


def test_a_milestone_without_epics_is_written_as_an_empty_list() -> None:
    d = data()
    d["milestones"][-1]["epics"] = []
    text = roadmap.dump(d)
    assert "    epics: []" in text
    assert roadmap.yaml.safe_load(text)["milestones"][-1]["epics"] == []


def test_unknown_keys_of_the_file_milestones_and_epics_are_refused() -> None:
    d = data()
    d["notes"] = "x"
    assert any("top-level keys must be labels and milestones" in p for p in roadmap.problems_of(d))
    d = data()
    d["milestones"][0]["owner"] = "x"
    d["milestones"][0]["epics"][0]["owner"] = "x"
    problems = roadmap.problems_of(d)
    assert sum("unknown keys ['owner']" in p for p in problems) == 2


def test_a_file_that_is_not_a_mapping_is_refused(monkeypatch, tmp_path) -> None:  # type: ignore[no-untyped-def]
    bad = tmp_path / "roadmap.yaml"
    bad.write_text("- a list\n", encoding="utf-8")
    monkeypatch.setattr(roadmap, "ROADMAP", bad)
    with pytest.raises(SystemExit, match="not a mapping"):
        roadmap.load()


def test_a_failed_github_call_stops_with_its_error(monkeypatch) -> None:  # type: ignore[no-untyped-def]
    class Done:
        returncode, stdout, stderr = 1, "", "HTTP 404"

    monkeypatch.setattr(roadmap.subprocess, "run", lambda *a, **k: Done())
    with pytest.raises(SystemExit, match="failed: HTTP 404"):
        roadmap.gh("repos/x/y")


def test_new_issue_needs_a_known_milestone_on_both_sides(monkeypatch) -> None:  # type: ignore[no-untyped-def]
    import argparse

    args = argparse.Namespace(milestone="M9", epic="E", title="T", labels="", spec=None, body=None)
    with pytest.raises(SystemExit, match="no milestone starts with 'M9'"):
        roadmap.new_issue(args)
    monkeypatch.setattr(roadmap, "github_milestones", dict)
    args.milestone = "M0"
    with pytest.raises(SystemExit, match="does not exist on GitHub"):
        roadmap.new_issue(args)


def test_sync_refuses_a_milestone_missing_on_github(monkeypatch) -> None:  # type: ignore[no-untyped-def]
    d = data()
    m, _e, t = roadmap.tasks(d)[0]
    monkeypatch.setattr(roadmap, "load", lambda: d)
    monkeypatch.setattr(
        roadmap, "github_issues", lambda: {t["issue"]: {"state": "open", "milestone": None}}
    )
    monkeypatch.setattr(roadmap, "differences", lambda *a: ([], []))
    monkeypatch.setattr(roadmap, "github_milestones", dict)
    with pytest.raises(SystemExit, match=f"milestone '{m['title']}' does not exist on GitHub"):
        roadmap.sync(write=False, apply_milestones=True)


@pytest.mark.parametrize(
    "title",
    ["feat: replay the ledger", "core(state): add guards", "[FEAT] guards", "Add guards."],
)
def test_titles_state_the_outcome_without_a_type_prefix(title: str) -> None:
    d = data()
    d["milestones"][0]["epics"][0]["tasks"][0]["title"] = title
    assert any("title" in p and title in p for p in roadmap.problems_of(d))


def test_titles_of_tasks_and_epics_are_unique() -> None:
    d = data()
    first = d["milestones"][0]["epics"][0]
    first["tasks"][1]["title"] = first["tasks"][0]["title"].upper()
    assert any("same title" in p for p in roadmap.problems_of(d))
    d = data()
    d["milestones"][1]["epics"][0]["title"] = d["milestones"][0]["epics"][0]["title"]
    assert any("same title" in p for p in roadmap.problems_of(d))


def test_a_title_may_start_with_a_command_name() -> None:
    d = data()
    d["milestones"][0]["epics"][0]["tasks"][0]["title"] = "wayloop init creates a structure"
    assert [p for p in roadmap.problems_of(d) if "canonical form" not in p] == []


# ------------------------------------------------------------------------------------------
# links: how GitHub names and links the issues of the plan


def github_mirror(d: dict) -> tuple[dict, dict, dict, set]:  # type: ignore[type-arg]
    """A GitHub that agrees with the roadmap: issues, sub-issues, blockers and labels."""
    issues, children = {}, {}
    for m in d["milestones"]:
        for e in m["epics"]:
            issues[e["issue"]] = {
                "title": e["title"],
                "milestone": {"title": m["title"]},
                "labels": [{"name": "epic"}],
                "body": "Epic.",
            }
            children[e["issue"]] = set()
            for t in e["tasks"]:
                issues[t["issue"]] = {
                    "title": t["title"],
                    "milestone": {"title": m["title"]},
                    "labels": [],
                    "body": f"Part of #{e['issue']}.\n\n## Outcome\nX.",
                }
                children[e["issue"]].add(t["issue"])
    return issues, children, {}, {lbl["name"] for lbl in d["labels"]}


def linked_data() -> dict:  # type: ignore[type-arg]
    d = data()
    for k, m in enumerate(d["milestones"]):
        for j, e in enumerate(m["epics"]):
            e.setdefault("issue", 9000 + 10 * k + j)
    return d


def test_links_agree_when_github_mirrors_the_roadmap() -> None:
    d = linked_data()
    assert roadmap.link_problems(d, *github_mirror(d)) == []


@pytest.mark.parametrize(
    ("break_it", "message"),
    [
        (lambda i, c, b, lab, t, e: i[t].update(title="Other"), "title on GitHub 'Other'"),
        (lambda i, c, b, lab, t, e: i[t].update(milestone=None), "not in milestone"),
        (lambda i, c, b, lab, t, e: i[e].update(labels=[]), "missing the label 'epic'"),
        (lambda i, c, b, lab, t, e: c[e].discard(t), "not a sub-issue of its epic"),
        (
            lambda i, c, b, lab, t, e: i[t].update(body="No parent line."),
            "must start with 'Part of #",
        ),
        (
            lambda i, c, b, lab, t, e: i[t].update(body=f"Part of #{e}.\n\n## Depends on\n- #3\n"),
            "not blocked by #3",
        ),
        (
            lambda i, c, b, lab, t, e: lab.discard("epic"),
            "label 'epic' is in the roadmap but not on GitHub",
        ),
        (lambda i, c, b, lab, t, e: i.pop(t), "does not exist on GitHub"),
    ],
)
def test_each_link_rule_is_reported(break_it, message: str) -> None:  # type: ignore[no-untyped-def]
    d = linked_data()
    if not any(lbl["name"] == "epic" for lbl in d["labels"]):
        d["labels"].append({"name": "epic", "color": "000000"})
    issues, children, blockers, labels = github_mirror(d)
    epic = d["milestones"][0]["epics"][0]
    break_it(issues, children, blockers, labels, epic["tasks"][0]["issue"], epic["issue"])
    assert any(message in p for p in roadmap.link_problems(d, issues, children, blockers, labels))


def test_a_native_dependency_satisfies_the_depends_on_section() -> None:
    d = linked_data()
    issues, children, blockers, labels = github_mirror(d)
    epic = d["milestones"][0]["epics"][0]
    task = epic["tasks"][0]["issue"]
    issues[task]["body"] = f"Part of #{epic['issue']}.\n\n## Depends on\n- #3 (the ledger)\n- #4\n"
    blockers[task] = {3, 4}
    assert roadmap.link_problems(d, issues, children, blockers, labels) == []
    assert roadmap.depends_on("## Depends on\n- #3 (x)\n- #4\n\n## Next\n- #5") == [3, 4]
