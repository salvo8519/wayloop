# Specs: spec-driven development

Every change to what the Wayloop engine does or exposes starts as a specification. Code
follows the specification, tests follow its acceptance criteria, and reviewers check the code
against both.

```
specs/
  constitution.md          non-negotiable principles (read first)
  _templates/              spec.md · plan.md · tasks.md
  NNNN-short-name/
    spec.md                WHAT and WHY: requirements and acceptance criteria
    plan.md                HOW: design, types, contracts, risks
    tasks.md               ordered checklist; [P] = can run in parallel
```

## Lifecycle

| Status | Meaning | Who moves it |
| --- | --- | --- |
| `Draft` | Being written; open questions allowed | anyone |
| `Surveyed` | Written by the survey from existing code (SPEC-0011 R4); waits for approval | engine; the maintainer approves it |
| `Approved` | Requirements frozen; implementation may start | maintainer |
| `Implementing` | Tasks in progress; pull requests cite the specification | engine, when the first task that cites the specification is claimed |
| `Done` | Every acceptance criterion verified by a test; documentation published | maintainer |
| `Superseded` | Replaced by a newer specification (linked) | maintainer |

## Rules

1. **No engine code without an `Approved` specification.** Refactors, documentation and
   tooling do not need one.
2. **Requirements use EARS phrasing** ("WHEN ..., THE ENGINE SHALL ...") and are numbered
   (`R1`, `R2`, ...) so that tests and pull requests can cite them.
3. **Every requirement has at least one acceptance criterion**, and every acceptance criterion
   becomes a test, written before the code where it pays.
4. **Every task names its verification technique**: TDD, BDD scenario, end-to-end, property,
   contract test, or none for documentation and tooling.
5. **Pull request titles and bodies cite the specification**:
   `feat(ledger): chain events by hash [SPEC-0002 R3]`.
6. Changing an `Approved` specification requires a pull request that edits it first.
7. **Every specification is referenced by a task of the roadmap** (`roadmap/roadmap.yaml`);
   CI checks it.

## Agent workflow

Write `spec.md` with the `spec-writer` role, the maintainer approves it, then `plan.md` and
`tasks.md`, then implementation task by task, then review by risk tier, then the pull request.
