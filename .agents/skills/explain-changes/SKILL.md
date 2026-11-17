---
name: explain-changes
description: Explains a commit range, a pull request, a branch or the working tree in the reader's language - what changed, why, and the impact on the engine, contracts, documentation, process and risks - citing files and requirements. Use when the maintainer or a contributor asks to understand or review changes, or before confirming a merge.
---

Talk to the maintainer in their preferred language (`CLAUDE.local.md`); everything written to the repository or to GitHub is in English, without icons.

The explanation is written in the reader's language: the language of the request, or the
maintainer's preference in `CLAUDE.local.md`. Commit subjects, code identifiers, paths and
command names are quoted in their original English. The skill reads; it changes no file.

## Produces
A structured explanation in the conversation: summary, changes by area, impact, and the
questions or decisions for the reader. Posted to GitHub only when asked, and then in English.

## Steps
1. **Scope, deterministically.** Identify what to explain and collect it:
   - pull request: `gh pr view <n> --json title,body,commits,files,labels` and `gh pr diff <n>`;
   - branch: `git log --oneline origin/main..<branch>` and `git diff origin/main...<branch> --stat`;
   - commit range: `git log --format='%h %s%n%b' <from>..<to>` and `git diff <from> <to> --stat`;
   - working tree: `git status` and `git diff --stat`, then `git diff --staged --stat`.
   If the scope is ambiguous, ask which one is meant before reading further.
2. **Facts.** Read the diff file by file. Read the linked issue, the specification it cites
   (`specs/NNNN-*/spec.md`) and the ADRs it touches. Run
   `uv run python scripts/docs_sync.py impact --base <base>` for the documentation impact and,
   for a pull request, `gh pr checks <n>` for the state of its checks.
3. **Group by area**: engine (`crates/wayloop-core/`), adapters (`crates/wayloop-cli/`, SDKs),
   schemas and contracts, specifications and ADRs, documentation and translations, process
   (`scripts/`, `.github/`, `roadmap/roadmap.yaml`, `sprints/`), agents and skills
   (`.agents/`), tests.
4. **Write the explanation**:
   - a summary of one paragraph: what the change does and why;
   - per area: what changed and why, citing files and, where they exist, requirements
     (`SPEC-NNNN Rk`) and ADR numbers;
   - impact: contracts (additive or breaking), behaviour of the engine, the ledger,
     documentation and Italian translations to update, release note, roadmap task, tier of
     review and who merges it (AGENTS.md, Merging);
   - risks and what is not covered by tests;
   - questions or decisions for the reader, each answerable in one line.
5. **Separate facts from judgement.** State what the diff shows; mark inferences as such.
   Never claim a check passed unless its output was read.
6. **Post** only when asked: an English comment with `gh pr comment <n> --body-file <file>`.

## Stop and ask
- The scope is ambiguous (several pull requests, an unclear range).
- The change contradicts its specification, an ADR or `AGENTS.md`: point it out as a finding
  and ask whether to hand it to `pr-triage` or the author; do not fix it here.
- Posting to GitHub needs an explicit request.

## Report
- Done: the scope explained and the sources read.
- Not done or not verified: files skimmed rather than read, checks not available, behaviour
  not run.
- Risks: breaking contracts, missing tests or translations, decisions left open.
