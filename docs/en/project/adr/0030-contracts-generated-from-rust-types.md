# ADR 0030: Contracts generated from Rust types, versioned snapshots with a history

Status: Accepted · 2026-11-20 (milestone M0 · Foundations; decided by the maintainer in #291: options 3, (a), (ii) and (B))

## Context

The contracts of Wayloop (ADR 0009, ADR 0028) are hand-written JSON Schema files under
`schemas/`, interpreted by the core over a documented subset (plan of SPEC-0001, D5). The
first command and the first artifact, in the pull request of #256, show the cost: the error
report, the source of an error, the origin of a document and the definition of a status are
written again in every file that needs them, by hand, in two directions (closed inputs, open
outputs). Each copy can drift from the Rust types that the core fills and from the other
copies, and only review finds it. The number of files grows with every command, every
artifact type and every version.

The maintainer asked, on 2026-11-20, for contracts that are maintainable, with every pattern
defined once, built by nesting small types into larger ones, and versioned as Fairmark versions
its method schemas, so that people, systems and agents can rely on them. The maintainer chose
the Rust types as the single source, as in Fairmark (ADR 0003 and ADR 0009 of Fairmark), and
not hand-written JSON fragments.

D5 of the plan of SPEC-0001 rejected types as the source for two reasons: a refactor of a type
would change the contract silently, and the generated shapes would follow the generator rather
than C4. Both are answered below by a check, not by discipline: the committed snapshot is the
contract of record and CI fails when the generated schema differs from it, and the generator
checks the subset and the shape rules of C4. D5 stays right on one point: the core keeps
validating documents with its own interpreter, so that every error is reported in one pass
with the reasons of C3. Serde deserialisation into the types never replaces it.

### Options

1. **Hand-written schemas, shared fragments by `$ref` across files.** Fewer copies, but the
   interpreter allows only local references, tool definitions must inline everything (ADR
   0024), and the Rust types still drift from the files.
2. **Hand-written schemas assembled from fragments by a script.** Removes the copies in JSON;
   the types remain a second source, checked against the files only by tests.
3. **Rust types as the source; snapshots generated, committed and checked.** One definition
   per pattern, shared by the core, the schemas, the tool definitions and the documentation.
   Needs a schema derive in a pure crate and a set of rules that keep the output inside the
   subset. Chosen by the maintainer.

For where the derive lives:

- **(a) `schemars` as a normal dependency of `wayloop-schemas`**, on its allowlist. Simple; the
  derive is compiled into the shipped crate although only the generator calls it.
- **(b) The derive behind a cargo feature** (`cfg_attr` on every type and attribute).
  `scripts/check_core_io.py` resolves the pure crates with `--all-features`, so the feature
  still needs the allowlist or an exception in the check; every contract type carries twice
  the attributes.
- **(c) Schemas implemented in a separate generator crate.** The orphan rule forbids deriving
  for types of another crate; this means hand-written implementations, which is option 1 in
  Rust.

For the history of versions:

- **(i) One committed `schemas/index.json`**, as Fairmark had at first. Fairmark replaced it
  (Fairmark ADR 0017): two pull requests that add two contracts both rewrite it and conflict,
  which serialises parallel lanes.
- **(ii) One manifest per contract folder** and an assembled view, as Fairmark does today.

For changes to a version already on `main`:

- **(A) Byte-immutable**, the current rule (`scripts/check_schemas.py`, conventions page,
  Contract snapshots): every change, additive or not, is a new version.
- **(B) As Fairmark:** earlier versions are byte-immutable; the current version may change
  additively only, as C5 defines additive, and a classifier refuses any other change.

## Decision

Option 3, with (a), (ii) and (B).

### G1. One source: the types of `wayloop-schemas`

- Every contract type lives in `wayloop-schemas` and derives `serde::Serialize`,
  `serde::Deserialize` and `schemars::JsonSchema`. Its doc comments become the descriptions of
  the schema. The generator refuses a type, a member or an enum value without a description.
  A description states the meaning and, for a number, its unit or its origin (from 0, from 1);
  the presence is checked by the generator, the content by the contract-auditor role.
- Each pattern is defined once and nested:
  - `Envelope<D>`: one generic envelope (C1), where `D` is the data type of one command;
    `Diagnostics`, `NotDone`, `Meta` and `DefaultApplied` inside it.
  - `Error`, `ErrorReport` (the first error and `errors`), `Source`, `Reason`: defined once,
    used by the envelope of every command and by `diagnostics.warnings`.
  - One input type and one data type per command (`ValidateInput`, `ValidateData`).
  - Artifact types built from one common type, `Common<Id, Status>`, holding the members every
    artifact has (`kind`, `schema_version`, `id`, `title`, `status`, `language`,
    `superseded_by`), with the identifier type and the status enum of the kind as parameters.
    The constants `kind` and `schema_version` come from the registration of the contract
    (G5), not from each type.
  - Identifier newtypes (`BriefId`, `TaskId`, and so on), numbers with their bound
    (`IssueNumber`, `Line`) and `ProjectPath`: one type per value form, used wherever the
    form appears.
- Composition by `#[serde(flatten)]` is allowed for plain structs only, and a test proves that
  the generated schema merges their members into one object. No contract type uses
  `deny_unknown_fields`, which serde does not support with `flatten`: closedness is decided
  by the direction of the contract (G2).
- After the interpreter has accepted a document, deserialising it into its type must succeed;
  a failure there is an internal error (reason `internal`) and a defect. A test checks every
  golden contract case both ways: the serialised value of a type is accepted by its schema, and
  a document the schema accepts deserialises into the type.

### G2. What the derive cannot express, written once

One module of `wayloop-schemas`, `contract`, holds every rule the derive does not produce, as
`schemars` settings, attributes or transforms. No JSON is edited after generation.

| Need | Where it is written |
| --- | --- |
| Conditional requirements (`superseded_by` required when `status` is `superseded`) | A `#[schemars(transform = ...)]` attribute on the type that calls one helper of `contract::rules`; the helper writes the `allOf` of `if` and `then` of the subset |
| Named formats | One enum `Format` in `wayloop-schemas` listing every named format; each value-form newtype implements `JsonSchema` by hand, once, with `type: string` and its format. The core's format checker matches on the same enum, so a format the generator emits and the core lacks does not compile |
| Integer bounds | On the newtype (`#[schemars(range(min = 1))]` or a non-zero integer type); the bound of plus or minus (2^53 - 1) stays a rule of the interpreter, not of each schema |
| Closed inputs, open outputs | A transform applied by the generator by direction: every object of an input schema gets `additionalProperties: false`; output schemas get none (C5 as amended on 2026-11-20) |
| Absent, not null (C2) | Settings: an `Option` adds no `null` type and no `nullable`; optional members use `skip_serializing_if` and are left out of `required` |
| Enum values with their meaning | A transform turns the `oneOf` of documented constants into `enum`, and appends the meaning of each value to the description of the member, as ADR 0024 requires |
| Any JSON value (`fix`, `value`) | A transform turns the boolean schema `true` into an object schema with its description |
| Integer `format` values such as `uint32` | Removed by a transform; the type and the bounds remain |

A construct that none of these covers is not worked around: either a type changes to use a
covered form, or the subset of the interpreter is extended by a reviewed change of the core
with its tests and the architecture page.

### G3. The generated output stays inside the subset

- Draft 2020-12 only, references local to the file (`#/$defs/...`), no `pattern`, no `null`
  type, no `uniqueItems`, no `default`, no format the core does not implement. Contract types
  therefore use `Vec`, not sets; defaults are applied by the core and reported in
  `meta.defaults_applied` (C2), never written as `default` in a schema.
- The subset has one definition: the loader of the core (`wayloop_core::schema`). The
  generator loads every schema it produces with it and fails on the first refusal; a test of
  the core loads every embedded snapshot, of every version.
- The generator also checks C4 where it is mechanical: at most four levels of objects, and an
  enum value for every choice.
- The test-only validator of the core (the oracle) checks that every generated schema is a
  valid draft 2020-12 schema.

### G4. Where generation runs, and purity

- `schemars` is a normal dependency of `wayloop-schemas` (option (a)), without the
  `preserve_order` feature, added to the allowlist of both pure crates in
  `scripts/check_core_io.py` with every crate it reaches (expected: `schemars`,
  `schemars_derive`, `serde_derive_internals`, `dyn-clone`, `ref-cast`, `ref-cast-impl`; the
  exact list is fixed by the task with the output of `cargo tree`). It performs no I/O.
  Dependency rule (`dependencies.md`): maintainer Graham Esau, licence MIT, scope limited to
  deriving schemas, no I/O; to be confirmed by the security-reviewer role with `cargo deny`.
  Fallback if it is abandoned: the committed snapshots stay the contract, and the trait can be
  implemented by hand or by a fork without changing a file under `schemas/`.
- No `preserve_order`: with cargo feature unification it would turn on `preserve_order` of
  `serde_json` in the core, change the iteration order of every map the core reads, and add
  `indexmap` and `hashbrown` to the allowlists. The core visits members in the byte order of
  their keys; snapshots are written with keys in byte order too. Members of a snapshot are
  therefore listed alphabetically; `required` keeps the order of declaration.
- The generator is a binary of a new workspace crate, `wayloop-schemagen`, not published, at
  the adapter level of the crate layout (it may depend on `wayloop-schemas` and
  `wayloop-core`). It writes nothing: it prints one JSON document with every current schema
  of the registry. The runtime and the command line never generate schemas; they use the
  snapshots embedded with `include_str!`.

### G5. Versioning as Fairmark

- One registry list in `wayloop-schemas`: each contract names its area (`artifacts`,
  `inputs`, `commands`, `envelope`), its name, its current version and its types. The
  generator, the embedded snapshots and the kinds of the core are built from this list, and a
  test checks that the folders under `schemas/` match it.
- Files: `schemas/<area>/<name>/v<N>.json`; for commands also `v<N>.output.json` (the
  envelope with the data of that command, self-contained); for the hand-over also
  `v<N>.input.json` (SPEC-0001). `$id` is the published URL of the file, as today.
- History: `schemas/<area>/<name>/index.json` per contract (option (ii)), with
  `format`, `current` and, per version, the files, their SHA-256 and `introduced_in`
  (`unreleased` until the release stamps it), in the record shape of Fairmark. One function
  assembles the view `schema_history()`; the documentation build publishes it as
  `schemas/index.json` and the release attaches it.
- Changes (option (B)): a version earlier than the current one never changes; the current
  version may change only additively under C5 (a new optional input member, a new output
  member, a new enum value of an output); any other change is a new version `v<N+1>`. The
  current version never goes back, a contract is never removed. Before a version reaches
  `main`, it may change freely.
- An earlier version stays reproducible from its snapshot, not from Rust types: the frozen file
  is authoritative and is embedded for validation of documents and requests of that version
  (SPEC-0001 R16; the command version in the context of the entry point, ADR 0028 as amended).
  Moving from one version to the next is a pure function on JSON values in the core. Old types
  are not kept.
- The generator writes only the current version and refuses to rewrite an earlier one: it
  stops with the name of the contract and asks for a version bump.

### G6. One command, one check

- `uv run just generate` runs `wayloop-schemagen` and `scripts/schema_snapshots.py`, which
  writes the snapshots and the manifests of the current versions. It is deterministic
  (keys in byte order, two-space indentation, a final newline, UTF-8 without escapes) and
  idempotent: a second run changes no byte.
- `scripts/schema_snapshots.py` takes over `scripts/check_schemas.py`, so that one script
  holds every snapshot rule: `--check` fails when a committed snapshot or manifest differs
  from the generated one; `--diff-against <base>` classifies the change of the current version
  (additive or breaking, as the classifier of Fairmark does, with the open-output rule of C5)
  and refuses a rewritten history. The hook `.claude/hooks/protect_contracts.py` keeps the
  same file pattern, tested against the script. Both run in `just check` and in CI.
- Tool definitions (ADR 0024) are generated from the snapshots, not from the types, so that the
  definition of an earlier version is rebuilt from its frozen file. The types are therefore
  their source too, through one step.

### G7. Migration

1. This record, accepted by the maintainer.
2. The generator task (#291 follow-ups below): dependency and allowlist, `contract` module and
   registry, `wayloop-schemagen`, `schema_snapshots.py`, the error report and the envelope
   moved to the derive. The interpreter must accept open output objects before the first
   output snapshot is generated; that change of the core comes from the branch of #256 into
   this task, or lands first on its own.
3. The branch of #256 is rebased: its hand-written `schemas/artifacts/brief/v1.json` and
   `schemas/commands/validate/v1.json`, `v1.output.json` are replaced by the generated ones
   before their first merge, so no hand-written snapshot is ever released.

## Amendments

- 2026-11-20, security review of #301 (G4): the crates reached by `schemars` are those
  expected, confirmed by `cargo tree`. `ref-cast` has a build script that runs
  `rustc --version` and writes into `OUT_DIR`; nothing runs at run time. `dyn-clone` and
  `ref-cast` contain confined `unsafe` (the clone of a trait object, a cast between
  `repr(transparent)` types), reviewed and accepted. The features of `schemars` are declared
  explicitly (`default-features = false`, `derive` and `std`), so that a change of its
  defaults upstream cannot widen them.
- 2026-11-20, implementation of #294 (G4, G5, G6): a crate embeds with `include_str!` only
  files inside its own folder, since `cargo package` packs only that folder. `just generate`
  therefore also copies every snapshot of every version, byte for byte, into
  `crates/wayloop-schemas/snapshots/`, and writes
  `crates/wayloop-schemas/src/contract/embedded.rs`, the list `contract::EMBEDDED` that embeds
  the copies; `--check` fails when a copy or the list differs from what generation writes. The
  snapshots under `schemas/` stay the contract of record. The envelope is the contract
  `envelope/result`: the envelope of the result of every command, at the version of
  `meta.envelope_version`. The generator also prints every part of a contract with the name of
  its first file, so that the file pattern of the script and of the hook is checked against the
  registry. A released file is one that a history of the base records, or that the pattern
  matches; a released version record never changes, except `introduced_in` from `unreleased`
  to a release and, for the current version, the hashes of its files. Rule (B) is classified
  conservatively: besides the cases of C5, any change of a keyword other than `properties`,
  `required`, `items`, `$defs`, `enum` and the annotations is breaking, `allOf` and `$schema`
  included; a new `enum` value is additive in both directions, as in Fairmark.

- 2026-11-20, with the rebase of #256, subject to the approval of its pull request by the
  maintainer (G2): in the input direction the generator closes every
  object that declares its members (`properties`); an object that declares none is a free
  object, a document whose schema is another one (the documents of `validate`, checked by the
  schema of their kind, and the map of their lines, checked by the core), written by hand with
  `value::free_object_schema` and left open. The free objects are declared in one list,
  `FREE_OBJECTS` of the checks of the generator, which refuses any other open object of an
  input, a map derived by `schemars` included. The generator also drops a definition that no
  reference reaches, such as the type of `kind` once a rule makes it a constant. The line of a `source` is written only
  when it is known (SPEC-0004 R2): a breaking change of `envelope/result` v1, which no release
  holds, made under the amendment below.
- 2026-11-20, decision of the maintainer (G5, G6): a version that no release holds may change
  incompatibly until the first release that holds it; from that release it is immutable, and
  rule (B) applies in full. This replaces the last sentence of G5, "Before a version reaches
  `main`, it may change freely". No release holds a version when its record on the base says
  `introduced_in: unreleased` and no release tag `v<semver>` holds its file.
  `schema_snapshots.py --diff-against` allows a breaking change of such a current version and
  still reports it as breaking, so that the pull request shows it; an earlier version never
  changes, released or not. Residual risk: the documentation site publishes `main`, so the
  `$id` of an unreleased version is public, and the schema at that URL may change before the
  first release. A reader that depends on it before a release accepts that risk.

## Consequences

Positive:

- Each pattern (error report, source, envelope, common artifact members, identifier forms) is
  written once; a new command or artifact type is a data type, one registry line and a
  generated file.
- The core, the schemas, the tool definitions and the documentation read one definition; a
  disagreement between a type and a snapshot fails CI.
- The history is per contract: parallel lanes that add different contracts do not touch the
  same file.
- Every published version stays available, identified by its URL and its hash.

Negative:

- A pure crate gains a proc-macro dependency and its crates on the allowlist, compiled into
  the shipped crate though used only by the generator.
- Snapshots list members alphabetically, which reads less naturally than the hand-written
  order.
- A refactor of a contract type can fail CI on a snapshot that only looks unrelated; the
  failure names the contract and the pointer, and the fix is a version bump or a revert.
- Rule (B) relaxes the byte immutability of the current version: a cached copy of the current
  version can differ from a later one by additive changes; the history records each hash.

Follow-ups:

- Amend the plan of SPEC-0001 (D5 and the section on schemas and their interpreter), the
  architecture page (the schema interpreter, the crate layout, the allowlists), the
  conventions page (Contract snapshots, under rule (B)) and the dependency policy, each in the
  task that changes the behaviour it describes.
- The documentation build publishes `schemas/` at the URLs used as `$id`, and the assembled
  history.
