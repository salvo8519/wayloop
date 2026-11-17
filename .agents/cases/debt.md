# Behavioural case: debt

## Input
The code holds three comments: `// shortcut: linear scan of events, revisit above 10k events` in `crates/wayloop-core/src/ledger.rs` with an open issue citing it; `# shortcut: no retry on rate limit, revisit when M3 starts` in a script, with no issue, and M3 has started; `// shortcut: hard-coded path` with no trigger. A concept page also mentions the word "shortcut:" in prose.

## Expected
Three rows grouped by file, with line, limit and trigger; the concept page is not listed. States: tracked, due, and no-trigger. The totals line reads `3 markers, 1 tracked, 2 untracked, 1 due, 1 with no trigger.` Task drafts for the due and the no-trigger markers are handed to `roadmap-update`; the maintainer is asked for the missing trigger; no issue is created and no code is changed.
