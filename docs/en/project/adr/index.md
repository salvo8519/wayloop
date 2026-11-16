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
