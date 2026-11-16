# State in the tracker

Wayloop has no state of its own. The plan, the progress, the decisions and the questions live
in the issue tracker; specifications live in the repository; only session checkpoints stay on
the local machine.

| What | Where it lives | Why |
| --- | --- | --- |
| Roadmap, epics, stories, tasks | Issues, sub-issues, dependencies, issue types, milestones | Visible to the team and to any tool; it survives the removal of Wayloop |
| Progress, questions, decisions, gate outcomes | Fields and comments of the issues | Gates become asynchronous: a decision can be taken from a phone |
| Living specifications and change deltas | The repository, linked from the issue | They change with the code and are reviewed in pull requests |
| Session checkpoints | The local machine, never committed | Only to resume interrupted work |
| Tracker cache | The local machine, rebuildable | Speed and determinism; never a source of truth |

## Where the design is under strain, and how it holds

- **Speed.** Remote APIs are slow and rate-limited. The engine works on a local cache updated
  incrementally, which can be deleted and rebuilt at any time.
- **Free text.** People edit issue bodies too. The engine uses native fields (sub-issues,
  "blocked by" dependencies, issue types) and a structured block validated by a schema;
  anything that does not validate becomes a question, never a silent error.
- **History of the plan.** Each proposed change is a comment carrying the difference and a gate
  label; the approval stays in the issue timeline.
- **Concurrency.** An issue is claimed with an assignee and a label, and written only if it has
  not changed since the last read.
- **GitLab.** On the free tier, "blocks" relations and epics are not available. The GitLab
  adapter falls back on the structured block. GitHub is the first backend.
- **Chains of events.** A merge restarts reconciliation through a workflow. Events produced by
  the bot only create proposals, are idempotent and are grouped, so the loop never feeds itself.

## The removal test

The design is right only if a project stays readable after Wayloop is removed: issues,
specifications and hand-over comments must tell the whole story on their own.
