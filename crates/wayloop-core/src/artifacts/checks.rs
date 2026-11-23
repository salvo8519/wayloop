//! Identifiers of the checks of `validate`, written in `diagnostics.checks` when they ran and
//! in `diagnostics.not_done` when they did not (ADR 0009 C1, amended on 2026-11-20). Each is a
//! stable `snake_case` value, documented in `docs/en/reference/validate.md`.

use crate::Check;

/// Each document has a known `kind`, or is an input with `input_kind`.
pub const DOCUMENT_KIND: Check = Check::new("document_kind");
/// The `schema_version` of each document is known to its type (SPEC-0001 R16).
pub const SCHEMA_VERSION: Check = Check::new("schema_version");
/// The `id` of each document is in the canonical form of its type (R14).
pub const IDENTIFIER_FORMAT: Check = Check::new("identifier_format");

/// Some folder of a location of the artifact model exists in the project the documents were
/// discovered in (#320); run when `locations` is given, `input_absent` otherwise.
pub const ARTIFACT_LOCATIONS: Check = Check::new("artifact_locations");

// The schema of each type is its own check, `schema_<kind>` (`kinds::schema_check`): in
// `checks` when the schema is built, in `not_done` otherwise. The checks run on every set are
// `document_kind`, `schema_version`, `schema_<kind>` of each built type, and
// `identifier_format`, in that order, then `artifact_locations` when `locations` is given.

/// A living specification sent as Markdown is normalised into its document model (R12).
pub const MARKDOWN: Check = Check::new("markdown");
/// No two artifacts of the set carry one identifier (R14).
pub const IDENTIFIER_DUPLICATE: Check = Check::new("identifier_duplicate");
/// Links between artifacts, the hierarchy and the citations of ADRs (R3, R4, R9, R15).
pub const LINKS: Check = Check::new("links");
/// Parts of artifacts and operations of change deltas (R18, R20).
pub const PARTS: Check = Check::new("parts");
/// Transitions and initial statuses against the default branch (R13, R19).
pub const STATUSES: Check = Check::new("statuses");
/// Titles, branch names, commit headers and pull request links (R7, R8, R10, R11).
pub const NAMING: Check = Check::new("naming");

/// The checks of the whole set that this version of the engine does not run.
pub const NOT_BUILT: [Check; 5] = [IDENTIFIER_DUPLICATE, LINKS, PARTS, STATUSES, NAMING];
