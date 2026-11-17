# SPEC-0019: Benchmark framework

| | |
| --- | --- |
| Status | Draft |
| Milestone | M6 · Benchmark and hardening |
| Owner | @salvo8519 |
| Related | ADR 0021, #121, #122, #123 |
| Commit scope | benchmark |

## Problem
A claim of lower cost and higher quality must be measured, reproducible and published, or it is marketing.

## Scope
**In scope:** Fixed corpus of tasks (greenfield and brownfield, back end and front end); baseline of the same agent without Wayloop; metrics (accepted pull requests, tests passing, rework, human interventions, tokens, duration, cost); repeated runs; published raw data and harness.

**Out of scope:** Comparisons naming other products.

## User stories
- As an evaluator, I want to rerun the benchmark myself, so that I trust the numbers.

## Requirements (EARS)
- **R1**: THE BENCHMARK SHALL run every task of the corpus with Wayloop and with the baseline under the same budget.
- **R2**: THE BENCHMARK SHALL repeat each run a configured number of times and report spread, not only averages.
- **R3**: THE BENCHMARK SHALL record tokens, duration, cost, interventions and outcome of every run.
- **R4**: THE BENCHMARK SHALL publish the harness, the corpus and the raw results.
- **R5**: THE BENCHMARK SHALL measure each phase separately, so that cost is attributed.

## Acceptance criteria
- **AC1**: Given the published harness, when rerun on the same corpus, then results fall within the reported spread. Verification: end-to-end.

## Interfaces
- benchmarks/ folder; results page on the site.

## Open questions
- [ ] Size of the corpus for 1.0: 20 or 50 tasks?
