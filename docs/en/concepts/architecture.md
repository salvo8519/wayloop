# Architecture

One engine decides; thin adapters translate. The engine never calls a model, and agents never
write the state by hand. The rule is recorded in
[ADR 0001](../project/adr/0001-rust-engine-thin-adapters.md).

## What exists

| Component | Path | What it does today |
| --- | --- | --- |
| Schemas | `crates/wayloop-schemas` | Rust library with no I/O, home of the contract types that mirror the JSON schemas: the error report ([below](#errors-are-values)), the envelope with its exit codes ([below](#the-envelope)), the brief and the input and data of `validate`, with one type per value form (identifiers, project paths, language tags, numbers from 1). Each derives its JSON Schema inside the subset of the core ([below](#schemas-derived-from-the-contract-types)). |
| Core | `crates/wayloop-core` | Rust library with no I/O. It fixes the vocabulary of the issue cycle: the nine steps in order, their stable identifiers, and which step is an approval state. It re-exports the error of every refusal and the envelope, and computes the hash of an input from its canonical JSON ([below](#the-envelope)). It reads the JSON schemas of the contracts with its own interpreter ([below](#the-schema-interpreter)). Its entry point runs the command `validate`, which checks briefs against their schema ([reference](../reference/validate.md)). |
| Runtime | `crates/wayloop-runtime` | Rust library that runs the work the core decides ([below](#synchronous-core-asynchronous-runtime)). It holds no code yet. |
| Command line | `crates/wayloop-cli` | The `wayloop` binary, an adapter only: it parses arguments, calls the core and prints JSON. Commands: `version` and `steps` ([reference](../reference/index.md)), and `validate`, which prints the envelope of the entry point ([reference](../reference/validate.md)). |
| Schema generator | `crates/wayloop-schemagen` | A tool of the repository, not published: it prints the snapshots of the contracts, which `uv run just generate` writes under `schemas/` ([below](#snapshots-of-the-contracts)). |
| Agent content | `.agents/` | Roles and skills in the open Agent Skills format, with generated mirrors for Claude Code. They are used to build Wayloop itself. |
| Process | `roadmap/`, `specs/`, `sprints/`, `scripts/` | The plan as code, synchronised with the GitHub issues; specifications with numbered requirements; sprint lanes of disjoint paths; the checks run by CI. |

Wayloop has no release yet. The components of the engine, and the order in which they are
built, are in the [roadmap](../project/roadmap.md).

## Crate layout

The engine is a Cargo workspace of four crates. Each one has a single role, and dependencies
point in one direction only: towards the pure crates. A fifth crate of the workspace,
`wayloop-schemagen`, is a tool at the level of the adapters, never published.

```text
wayloop-cli        adapter: translates a channel, decides nothing
     |
     v
wayloop-runtime    I/O, no decisions
     |
     v
wayloop-core       decisions, no I/O
     |
     v
wayloop-schemas    contract types, no I/O
```

| Crate | Role | May depend on | Performs I/O |
| --- | --- | --- | --- |
| `wayloop-schemas` | Contract types of the JSON schemas under `schemas/` | no Wayloop crate | No |
| `wayloop-core` | All decision logic: artifact model, state machine, ledger rules | `wayloop-schemas` | No |
| `wayloop-runtime` | Queue, parallel workers, tracker calls, single writer to the ledger | `wayloop-core`, `wayloop-schemas` | Yes |
| `wayloop-cli` and the other adapters | Translation between a channel and the engine | any of the above | Yes |
| `wayloop-schemagen` | Prints the snapshots of the contracts; not published | `wayloop-core`, `wayloop-schemas` | Yes, standard output only |

- **One entry point.** The core exposes one function to every channel: command, canonical
  input and context in, envelope out
  ([ADR 0028](../project/adr/0028-one-contract-on-every-channel.md)). It is
  `wayloop_core::run`, at the root of `wayloop-core`; adapters call it and only translate its
  input and its envelope, `Envelope<Value>`, whose data is the data type of the command written
  as JSON in one place. `wayloop_core::run_json` takes the input as JSON text: text that is not
  JSON, holds a number beyond a double or repeats a key in one object is refused with the reason
  `syntax`, `range` or `duplicate` before the command runs. The context carries the settings of
  the channel: the dry run, written in `meta.dry_run`, and the requested version of the command;
  without one the current version runs and is listed in `meta.defaults_applied`, and a version
  the engine does not run is refused (ADR 0028, amended on 2026-11-20).
  `wayloop_core::usage_error` and `wayloop_core::input_unreadable` build the envelopes of a
  channel that was not used as documented and of an input it could not read, so that the
  command line prints an envelope for every outcome.
- **Allowlisted dependencies.** Each pure crate may depend only on the crates of its reviewed
  allowlist. Both may use `serde` with its derive macros and `serde_json`, which perform no
  I/O, with the crates they reach (`serde_core`, `serde_derive`, `proc-macro2`, `quote`,
  `syn`, `unicode-ident`, `itoa`, `zmij`, `memchr`), and `schemars`, which derives the JSON
  Schema of the contract types and performs no I/O, without its `preserve_order` feature, with
  the crates it reaches (`schemars_derive`, `serde_derive_internals`, `dyn-clone`, `ref-cast`,
  `ref-cast-impl`) ([ADR 0030](../project/adr/0030-contracts-generated-from-rust-types.md));
  `wayloop-core` may also use `wayloop-schemas`, and `wayloop-schemas` no Wayloop crate.
  `scripts/check_core_io.py` resolves each pure crate on its own with `cargo tree` (normal
  dependencies, every target, all its features, the lock file left unchanged) and fails on
  every crate reached outside the allowlist, printing the path that reaches it and naming
  known I/O crates. The allowlists also enforce the direction of the table above: a pure crate
  never depends on the runtime or an adapter, and the schemas never depend on the core. A
  feature that another crate of the workspace enables on a shared dependency is not blamed on
  a pure crate. Development dependencies (tests only) and build dependencies (run at build
  time) are not part of the shipped crate and are skipped. Extending an allowlist is a
  reviewed change to the script.
- **No I/O in the source.** The compiler refuses it. The `clippy.toml` of the workspace, the
  only one, lists the parts of the standard library that perform I/O or read ambient state:
  files (`std::fs`), the network (`std::net`), processes (`std::process`), the environment
  (`std::env`), the standard streams, threads, clock reads (`SystemTime::now`, `Instant::now`)
  and hash maps seeded at random (`HashMap`, `HashSet`; `BTreeMap` and `BTreeSet` are used
  instead). The Clippy lints `disallowed_methods` and `disallowed_types` apply the lists to
  every crate and every target by default, tests included. A crate that performs I/O, the
  runtime, an adapter or the schema generator, opts out explicitly with an `allow` of the two
  lints at the root of each of its targets that needs it. Each pure crate also forbids at its root `print_stdout`,
  `print_stderr`, `dbg_macro`, `disallowed_methods` and `disallowed_types`: `forbid`, unlike
  `deny`, cannot be lifted by an `allow` further in. The lints follow name resolution, so an
  alias or a nested import does not escape them. `cargo clippy` applies them in `just check`
  and in CI. `scripts/check_core_io.py` fails when the attribute or an entry of the
  `clippy.toml` is removed, when a crate has a `clippy.toml` of its own, which Clippy would
  read instead of the workspace one, and when a manifest allows the two lints.
- **No panic.** Every crate of the workspace refuses the Clippy lints that let code panic or
  stop the process: `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`,
  `unreachable`, `indexing_slicing`, `string_slice`, `get_unwrap`, `panic_in_result_fn`, `exit`
  and `mem_forget`, on top of the `all` and `pedantic` groups. They are set in the workspace
  manifest and fail `cargo clippy -D warnings` in `just check` and in CI. Tests may unwrap,
  expect, panic and index: the `clippy.toml` of the workspace allows it in test code only. A
  failure is an error value; the command line turns it into an exit code, even when it cannot
  write its output.

## Synchronous core, asynchronous runtime

The split between the core and the runtime around it is the subject of
[ADR 0006](../project/adr/0006-synchronous-core-asynchronous-runtime.md). The core is
synchronous and pure: given a state and an event, it returns the next state and the work to
do. All concurrency lives in the runtime.

- **Queue.** Ready work is derived from the state (dependencies satisfied, lane free), not kept
  in an external broker. There is nothing to install and nothing to lose.
- **Parallel workers.** Issues of different lanes run in parallel, each in its own git
  worktree. Lanes declare disjoint paths, checked before the sprint starts, so parallel agents
  never edit the same files. A configurable limit caps the number of agents running at once.
- **Single writer.** Workers send their results to one writer that validates them and appends
  them to the ledger in order. Parallel execution never makes the state non-deterministic.
- **Batches.** Tracker calls are grouped to respect rate limits. Independent, non-interactive
  steps (council opinions, documentation of a module, reviews) can go through the batch APIs
  of model providers where they exist, which are cheaper and slower.

This repository already works this way by hand: sprint lanes declare disjoint paths, checked by
`scripts/sprint.py`, and every issue is worked in its own git worktree.

## Contracts

Every input and output crossing the engine boundary is JSON with a versioned schema. An
adapter that needs a new behaviour asks the engine for it; it never implements it. The
envelope shared by every command, with its error codes and exit codes, is the subject of
[ADR 0009](../project/adr/0009-contract-guidelines.md).

### Errors are values

The engine never panics on bad input: it returns `wayloop_core::Error`. The type is a contract
type: it is defined in `wayloop-schemas` and re-exported by the core. It carries as data every
field of a refusal that an agent acts on (ADR 0009, C3), and where the faulty value is stored:

| Field | Meaning |
| --- | --- |
| `code` | Stable identifier of the refusal; a new meaning gets a new code, never a renamed one (C8) |
| `message` | What went wrong, for a person |
| `pointer` | RFC 6901 pointer from the root of the input sent; empty for the whole input; a value of the type `Pointer`, checked when it is parsed from text, or built one escaped token at a time (`join`, `join_index`) |
| `pointers` | Pointers to the other parts of the input involved, such as both sides of a conflict; optional |
| `reason` | One value of the documented enum: `syntax`, `missing`, `unknown_field`, `duplicate`, `type`, `enum`, `conflict`, `requires`, `range`, `length`, `state`, `stale`, and `internal` for a failure of the engine itself, which is not a refusal of the input |
| `expected` | The values accepted at `pointer`, never a pattern; optional, and `[]` when no value is accepted |
| `fix` | The closest field for a misspelt key, or a corrected value; optional |
| `source` | Where the value is stored: `{file, line}` for a repository file, with `line` only when it is known, `{issue}` for a tracker issue, `{milestone}` for a tracker milestone, with lines and numbers from 1; absent for a document sent directly |
| `document` | Index, from 0, of the document of the input the error belongs to, whose `pointer` is then relative to that document; absent for an error of the input itself |

An optional field that is not given is left out of the JSON form; a list given empty is
written `[]`, because absent means not given and `[]` means none (ADR 0009, C2). The error
report, `wayloop_core::ErrorReport`, lists every error found in one pass in `errors`, in the
order found, and repeats the first one at the top level, so `errors[0]` always equals the top
level. `Error` and `ErrorReport` implement `Display` and `std::error::Error`. A code is a constant of the type
`Code`, so that one place lists every code of the engine; the codes themselves come with the
specification of each command.

### The envelope

Every command returns one envelope (ADR 0009, C1), a contract type of `wayloop-schemas`
re-exported as `wayloop_core::Envelope<D>`, generic over `D`, the data type of the command; the
diagnostics, the meta and the error report inside it are defined once for every command
([ADR 0030](../project/adr/0030-contracts-generated-from-rust-types.md), G1):

| Field | Meaning |
| --- | --- |
| `command` | Identifier of the command |
| `data` | The result; present only when the command succeeded |
| `error` | The error report ([above](#errors-are-values)); present only when the command was refused or failed, and then `data` is absent |
| `diagnostics` | `warnings` in the shape of an error; `not_done`, one `{check, reason}` for each check not run, with the reason `not_built` (the part of the engine that runs it is not built yet) or `input_absent` (the input it needs was not given); `checks`, the checks run. Each list is always written, `[]` when empty |
| `meta` | `engine_version`, `envelope_version` (1), `command_version`, `input_hash`, `dry_run` and `defaults_applied`: one `{pointer, value, reason, message}` for each default the engine applied, where `reason` is an identifier and `message`, for people, is optional |

Every identifier in the envelope (a check, a reason, an error code) is a stable `snake_case`
value, documented with the command that writes it; clients accept values they do not know
(ADR 0009, C5).

`Envelope::exit_code()` gives the exit code of ADR 0009 C7: 0 for a result with `data`, 1 for a
refused input, 2 for a channel not used as documented (code `usage`), 3 for an input that could
not be read (code `input_unreadable`), 5 for an internal error of the engine; it returns `Outcome::exit_code()`, the one
place where an outcome is mapped to an exit code. An internal error is written in `error` as a refusal is; its errors have
the reason `internal`, which tells the two apart in the JSON. The envelope holds nothing that depends on the machine or the
moment: no clock time, host name or absolute path.

An outcome is built only through constructors that refuse, by returning `None`, what the
contract does not define: `Outcome::success` refuses data written as null (null means not
given, ADR 0009 C2) or with no JSON form, `Outcome::refused` a report with an error of reason
`internal`, `Outcome::failed` a report with an error of any other reason, `Outcome::unusable`
and `Outcome::environment` a report whose errors are not all of the codes `usage` and
`input_unreadable`. `Outcome::internal` always builds a failure, for a caller that must give
an outcome when no other can be built. Every envelope the
engine can build is therefore one its reader accepts. The reader refuses the same cases, and a
document with both `data` and `error`, or with neither. The schema requires `error` when
`data` is absent; the subset of the interpreter has no negation, so only the type refuses both.
The schema of the envelope of any data is that of `Envelope<Value>`, committed as the snapshots
of the contract `envelope/result`; the output schema of a command is the envelope with the data
type of that command. The tests of the core load every snapshot with the interpreter and the
oracle.

`meta.input_hash` is the SHA-256 of the input serialised as canonical JSON (RFC 8785), in
lower-case hexadecimal, so the same input gives the same hash on every channel and platform:

- **Canonical JSON** (`wayloop_core::canonical`). Object members are sorted by the UTF-16 code
  units of their keys, never by the order of the map; strings are escaped as ECMAScript
  `JSON.stringify` escapes them; numbers are written as the IEEE 754 double they denote, in the
  ECMAScript form, so `4.50` is written `4.5` and `1E30` is written `1e+30`. An integer beyond
  2^53 in magnitude is written as the nearest double, as RFC 8785 requires, so two such integers
  can share a hash. `4364.0` and `4364` denote the same double, so they hash the same. NaN and the infinities cannot be held in a JSON value, so the function is
  total. The writer keeps its own stack, so a deeply nested input cannot exhaust the call stack.
  It is checked against the examples and the number samples of RFC 8785. A JSON value cannot
  hold duplicate keys, which RFC 8785 refuses: `serde_json` keeps the last of two equal keys, so
  whoever parses JSON text must refuse them first, with the reason `duplicate`.
- **Number parsing.** The workspace declares `serde_json` once, with the `float_roundtrip`
  feature, which adds no crate: every number is read as the nearest double, as other JSON
  parsers read it. The default parser of `serde_json` can be one unit in the last place off, and
  the same input would then hash differently on two channels. Tests of the core fail if
  `float_roundtrip` is dropped or if `arbitrary_precision`, which keeps the text of a number
  instead of its double, is enabled by any crate.
- **SHA-256** (`wayloop_core::sha256`). Written by hand from FIPS 180-4, so that the core needs
  no hashing crate (SPEC-0001 plan, D3). It is checked against the published test vectors and,
  on random inputs, against a hashing crate used in tests only (`sha2`, a development
  dependency, which is never shipped and is not on the allowlist of the core). The hash
  identifies content and protects no secret, so it makes no attempt at constant time.

### The schema interpreter

The schemas of the contracts are JSON Schema (draft 2020-12), derived from the contract types
(see the next section). The core reads them with a small interpreter of a documented subset,
`wayloop_core::schema`, so that every error of a document is reported in one pass with its
pointer and its reason, and the core needs no validator crate (plan of SPEC-0001, D5). A schema that uses a keyword outside the
subset, or a keyword out of its place, is refused when it is loaded: no rule of a schema is
silently ignored.

| Keyword | Meaning | Reason of the error |
| --- | --- | --- |
| `type` | `boolean`, `integer`, `string`, `array` or `object`; an integer is a finite number without a fraction, so `2.0` is one | `type`, and nothing else is checked on that value |
| `enum`, `const` | Accepted values: strings, integers, booleans or `null`, compared by value | `enum`, with the accepted values in `expected` |
| `minLength`, `maxLength` | Bounds of a string, in Unicode code points | `length` |
| `minItems`, `maxItems` | Bounds of an array | `length` |
| `minimum`, `maximum` | Inclusive integer bounds, beside `type: integer` | `range` |
| `format` | A named format of the core, on strings | `syntax`, with a corrected value in `fix` when one can be derived |
| `items` | Schema of every element of an array | |
| `properties` | Schema of each declared member; with `additionalProperties: false` any other member is refused, without it the object is open and other members are accepted unchecked, as in output schemas (ADR 0009 C5, amended on 2026-11-20) | `unknown_field`, with the closest declared key in `fix` |
| `required` | Declared members that must be present | `missing` |
| `if` and `then`, `allOf` | Conditional requirements: when the value matches the condition of `if`, the members of the `required` of `then` must be present; `allOf` lists such pairs | `requires`, with the members the condition tested in `pointers` |
| `$ref`, `$defs` | A reference to a definition of the same file, whose name has no `/`, `~` or `%` | |
| `$schema`, `$id`, `$comment`, `title`, `description` | Annotations, at the root for `$schema` and `$id` | |

The condition of an `if` uses `properties`, `required`, `enum`, `const` and `contains`. There is
no `pattern`: the core holds no regular expression engine, and a named format gives a precise
`fix`. The named formats are `date`, `utc-date-time` (with the suffix `Z`), `language-tag`
(language, script, region and variants of BCP 47), `tracker-handle`, `project-path` (a path
relative to the root of the project in one spelling: segments joined by `/`, none empty, `.`
or `..`; no `\` and no drive letter), the identifiers of each
artifact type (`brief-id`, `council-id`, `gate-id`, `epic-id`, `story-id`, `spec-id`,
`change-id`, `task-id`, `milestone-id`, `handover-id`) and the identifiers of parts
(`requirement-id`, `criterion-id`, `decision-id`, `question-id`), with the canonical forms of
SPEC-0001. The documentation of the module lists each form and each fix. The list of named
formats is one enum, `wayloop_schemas::Format`, and the format checker of the core matches on
it exhaustively: a format added to the enum does not compile until the core checks it.

- **Null.** A member of an object set to `null` is not given, exactly as an absent member
  (ADR 0009 C2): accepted when optional, `missing` when required, never an unknown field, and
  absent for a condition. A `null` element of an array is a value like any other and fails
  `type`, since no type of the subset is `null`.
- **Order of the errors.** Errors come in document order: the errors of a value before the
  errors inside it, the members of an object in the byte order of their keys, the elements of
  an array by index, and the absent members of an object after its present ones. A member
  missing under both `required` and a `then` is reported once, as `missing`.
- **Closest key.** The fix of an unknown key is a declared key not already present, at an
  optimal string alignment distance (the restricted Damerau-Levenshtein distance) of at most a
  third of the length of the unknown key; ties go to the first key in byte order. A key longer
  than 64 characters gets no fix.
- **Integers.** Every integer the interpreter checks lies within plus or minus (2^53 - 1), the
  range in which an IEEE 754 double holds every integer exactly (RFC 7493, I-JSON); a value
  beyond is refused with the reason `range`, whatever its schema says. Integers in a schema
  are held to the same bound. Numbers are compared by value, so `4364.0` equals `4364`. A
  number in a part of a document that no schema describes is not checked. Numbers are read as
  IEEE 754 doubles, as RFC 8785 reads them, so digits beyond that precision are not seen:
  `1.0000000000000001` is the integer 1.
- **Bounds.** A schema is at most 32 levels of subschemas deep, counted through references, and
  a reference that reaches its own definition is refused. The interpreter descends into a
  document only where its schema does, so its recursion is bounded whatever the document.
- **Oracle.** The tests check every schema and document they use with a JSON Schema validator
  crate, a development dependency that is never shipped; it must agree with the interpreter on
  the verdict. The named formats are checked in the oracle by regular expressions written from
  SPEC-0001, independently of the parsers of the core. Two rules of Wayloop are not JSON
  Schema, so the oracle sees the document without its members set to null, and the integer
  bound is added to its verdict; the bound is checked on every number, which matches the
  interpreter only for documents that their schema describes fully, as the documents of the
  tests are.

### Schemas derived from the contract types

Every contract type of `wayloop-schemas` derives its JSON Schema with `schemars`, and its doc
comments become the descriptions
([ADR 0030](../project/adr/0030-contracts-generated-from-rust-types.md)).
`wayloop_schemas::contract::schema_for` returns the schema of a type for one direction, input
or output, and writes no file; `contract::TYPES` lists every contract type whose schema is
derived. Every contract type also reads back what it writes: a document its schema accepts
deserialises into the type, which writes the same JSON again (`contract::reread`). A closed
enum is written with the macro `closed_enum!` of `wayloop_schemas::ident`, one spelling per
value read by `id()`, `from_id()` and `Display`; two values with the same spelling do not
compile. A closed enum of a contract (`Reason`, `NotDoneReason`) is written with
`contract_enum!`, so that its serialised form and the values of its schema read the same
spelling; `Format`, whose value `contract::rules::formatted` writes through `id()`, and
`IssueStep`, in no contract yet, use `closed_enum!`; `ExitCode`, a number by contract (ADR 0009
C7), has no spelling. A static identifier (`Code`, `Check`,
`DefaultReason`) is written with `static_id!`: constants of the engine borrow their text, a
value read from a document owns it. What the derive does not write is written once, in the
module `contract`:

| Need | How it is written |
| --- | --- |
| A conditional requirement | A function of the type, named in `#[schemars(transform = ...)]`, calls `contract::rules::require_if_present`, `require_if_absent` or `require_if_equals`, which append an `if` and `then` pair to `allOf`; `require_if_absent` tests the member against `const: null`, which holds when it is absent or null, in the interpreter and in draft 2020-12 alike |
| A named format | A value form implements `JsonSchema` by hand with `contract::rules::formatted`, from a value of `Format` |
| Absent, not null | An optional member, out of `required`, loses the `null` type the derive adds to an `Option` |
| An enum value with its meaning | A `oneOf` of documented constants becomes an `enum`, and each value is appended to the description as a line ``- `value`: meaning`` |
| Any JSON value | The boolean schema `true` becomes an object schema with a description |
| Integer formats | A `format` such as `uint32` is removed; the type and the bounds remain |
| Closed inputs, open outputs | In an input every object that declares its members and has no `additionalProperties` gets `additionalProperties: false`; an object that declares none is a free object, written by hand for a document of another schema, and stays open; an output has none |

The lines of a doc comment are joined into paragraphs, since they wrap at the width of the
code. The generator then checks the rules of the contracts that are not rules of the subset:
a description on the root, every definition and every member (one of blanks only is none),
the meaning of every enum value in its description, no `additionalProperties` in an output,
and at most four levels of objects (ADR 0009 C4). The subset itself has one definition, the
loader of the interpreter (ADR 0030 G3): `wayloop_core::schema::contract_schema` runs the
checks of the generator and then `Schema::load` on a registered type, and fails with
`ContractFault::Rule` or `ContractFault::Subset`. A `null` type, the keywords `pattern`,
`default` and `uniqueItems`, a format that is not named, a boolean schema or a map in an input
are refused there, at their place.

A type whose serialised form the derive cannot describe in the subset has a private form type
for its schema and for reading: `Source`, whose forms are one object with optional members
and `file` and `line` given together, which the type reads as exactly one form, and
`ErrorReport`, the members of its first error merged with `errors`, read only when `errors`
starts with those members, and `Envelope<D>`, with `data` and `error` both optional in its
schema, read as exactly one of them. A generic type has no schema until its parameter is
given, so `contract::TYPES` registers the envelope as `Envelope<Value>`, under the name of its
schema, `Envelope`. `contract::snapshot` writes the text of a snapshot: the schema with its
`$id`, keys in byte order, two-space indentation and a final newline; it writes no file. The
tests of the core run `contract_schema` on every type of
`contract::TYPES` in both directions, check each schema against the meta-schema of draft
2020-12 with the oracle, and check that the serialised values of the types are accepted and
read back into the same JSON.

### Snapshots of the contracts

The schemas of the contracts are committed under `schemas/` as snapshots, generated from the
types ([ADR 0030](../project/adr/0030-contracts-generated-from-rust-types.md), G5 and G6).
One registry, `wayloop_schemas::contract::CONTRACTS`, lists every contract with its area
(`artifacts`, `inputs`, `commands`, `envelope`), its name, its current version and the type and
direction of each of its files; today it lists the envelope, `envelope/result`, at the version
of `meta.envelope_version`. The binary `wayloop-schemagen` runs `contract_schema` on every file
of every current version, writes its text with `contract::snapshot` and prints them all as one
JSON document; it writes no file. `scripts/schema_snapshots.py` writes those bytes unchanged at
`schemas/<area>/<name>/v<N>.json` (`v<N>.output.json` for the output of a command,
`v<N>.input.json` for the input of a hand-over), with the history of each contract in
`schemas/<area>/<name>/index.json`.

A version earlier than the current one never changes, and the current version, once released,
changes only additively (ADR 0009 C5): the rules are in the
[conventions](../project/conventions.md#contract-snapshots). An earlier version is reproduced
from its committed file, not from the types, which describe only the current one.

`wayloop-schemas` embeds the text of every snapshot of every version, `contract::EMBEDDED`.
Since `cargo package` packs only the folder of a crate, the snapshots are embedded from
byte-for-byte copies inside it, in `crates/wayloop-schemas/snapshots/`, listed in the
generated file `src/contract/embedded.rs`; the check fails when a copy differs from its
source. The tests of `wayloop-schemas` check that the embedded current versions are the
snapshots of the types, and the tests of the core load every embedded snapshot with the
interpreter and the oracle.
