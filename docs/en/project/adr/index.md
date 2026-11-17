# Architecture decisions

Each decision is recorded once, with its context, the options considered and its
consequences. A decision is changed only by a new record that supersedes it.

| Record | Decision | Status |
| --- | --- | --- |
| [0001](0001-rust-engine-thin-adapters.md) | One engine in Rust, thin adapters | Accepted |
| [0002](0002-tracker-as-database.md) | The issue tracker is the database of the plan | Accepted |
| [0003](0003-deterministic-ledger.md) | An append-only, hash-chained ledger written by the engine | Accepted |
| [0004](0004-no-embeddings.md) | No embeddings in the product | Accepted |
| [0005](0005-english-source-bilingual-docs.md) | English source, documentation in English and Italian | Accepted |
| [0006](0006-synchronous-core-asynchronous-runtime.md) | Synchronous pure core, asynchronous runtime | Proposed |
| [0007](0007-files-as-source-sqlite-as-index.md) | Files as the source, SQLite as a derived index | Proposed |
| [0008](0008-identifiers-and-traceability.md) | Identifiers and traceability between artifacts | Proposed |
| [0009](0009-contract-guidelines.md) | Contract guidelines: envelope, errors and versioning | Proposed |
| [0010](0010-configuration.md) | One configuration file with layered precedence | Proposed |
| [0011](0011-executors-behind-an-interface.md) | Executors behind an interface; no model calls in the core | Proposed |
| [0012](0012-gates-on-the-tracker.md) | Approval gates are tracker states, not chat prompts | Proposed |
| [0013](0013-worktree-per-issue.md) | One worktree per issue; lanes as the unit of parallelism | Proposed |
| [0014](0014-specifications-by-capability.md) | Living specifications by capability with change deltas | Proposed |
| [0015](0015-index-design.md) | Index: tree-sitter symbols and SQLite full-text search | Proposed |
| [0016](0016-council-protocol.md) | Expert council: independent first round and structured report | Proposed |
| [0017](0017-idempotent-issue-generation.md) | Generated issues are idempotent by stable identifiers | Proposed |
| [0018](0018-language-bindings.md) | Bindings through one JSON boundary | Proposed |
| [0019](0019-distribution-and-versioning.md) | Distribution channels and versioning | Proposed |
| [0020](0020-plugin-and-skill-packaging.md) | Plugins and skills generated from one source | Proposed |
| [0021](0021-benchmark-methodology.md) | Benchmark methodology | Proposed |
| [0022](0022-threat-model.md) | Threat model: content is data, executors have least privilege | Proposed |
| [0023](0023-stability-policy.md) | Stability policy for 1.0 | Proposed |
| [0024](0024-tool-definitions-for-agents.md) | Tool definitions for agents: MCP and function calling from one source | Proposed |
| [0025](0025-api-executor.md) | API executor for steps that do not edit code | Proposed |
| [0026](0026-process-policy-in-the-core.md) | Process policy as registries, rules in the core and a ledger of results | Proposed |
| [0027](0027-skills-agents-hooks-standard.md) | One standard and one source for skills, agents and hooks | Proposed |
