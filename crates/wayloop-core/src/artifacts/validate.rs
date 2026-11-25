//! Validation of a set of artifacts and inputs (SPEC-0001; plan, Entry point and validation
//! function).
//!
//! [`validate`] is a pure, total function: it never panics, never stops at the first error,
//! and gives the same [`Validation`] for the same input. It is reached only through the entry
//! point of the crate (ADR 0028). Errors come in the order of the documents, then of the
//! stages (kind, version and schema; identifier), then within a stage in the order of the
//! schema pass: the members of an object in the byte order of their keys, absent members
//! after present ones. Each error of a document carries the index of the document. A check
//! this version does not run is listed in [`Validation::not_done`], never skipped silently.
//!
//! A document the interpreter has accepted, migrated to the current version of its type, is
//! read into its contract type (ADR 0030 G1); a failure there is a defect of the engine.

use std::collections::BTreeSet;

use serde_json::{Map, Value};
use wayloop_schemas::artifact::ArtifactKind;
use wayloop_schemas::contract::{Part, VALIDATE};
use wayloop_schemas::validate::{ArtifactFolder, DocumentSummary, InputKind, Origin, ValidateData};
use wayloop_schemas::{Diagnostics, ErrorReport, NotDone, NotDoneReason, Outcome};

use super::identifiers::{self, Indexed};
use super::input::{self, Body, Entry, Input, Place};
use super::kinds::{self, Checked, Kind, Loaded};
use super::{checks, codes};
use crate::command::COMMAND_INPUT;
use crate::{Check, Error, Pointer, Reason};

/// What validating a set gave.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Validation {
    /// One summary per document, in the order of the input.
    pub(crate) documents: Vec<DocumentSummary>,
    /// Every error found, in one pass.
    pub(crate) errors: Vec<Error>,
    /// Warnings: the set is not refused for them.
    pub(crate) warnings: Vec<Error>,
    /// The checks not run, each once, in the order of their identifiers.
    pub(crate) not_done: Vec<NotDone>,
    /// The checks run.
    pub(crate) checks: Vec<Check>,
    /// Failures of the engine itself, with the reason `internal`; when there is one, the
    /// result is a failure, whatever the errors.
    pub(crate) failures: Vec<Error>,
}

/// Validates `input` with the types `kinds`, loading their schemas first.
pub(crate) fn validate(input: &Input, kinds: &[Kind]) -> Validation {
    match kinds::load_all(kinds) {
        Ok(loaded) => validate_with(input, &loaded),
        Err(failure) => Validation {
            failures: vec![*failure],
            ..Validation::default()
        },
    }
}

/// Validates `input` with the types `kinds`.
pub(crate) fn validate_with(input: &Input, kinds: &[Loaded]) -> Validation {
    let mut run = vec![checks::DOCUMENT_KIND, checks::SCHEMA_VERSION];
    run.extend(kinds.iter().map(|k| kinds::schema_check(k.id)));
    run.extend([
        checks::IDENTIFIER_FORMAT,
        checks::IDENTIFIER_DUPLICATE,
        checks::IDENTIFIER_LOCATION,
    ]);
    let mut validation = Validation {
        checks: run,
        ..Validation::default()
    };
    let mut not_done: BTreeSet<NotDone> = checks::NOT_BUILT
        .iter()
        .map(|c| NotDone::new(c.clone(), NotDoneReason::NotBuilt))
        .collect();
    // The index of the identifier stage: every artifact of a known kind whose `id` raised no
    // error, whatever its other errors, so that duplicates are found in the same pass (ADR 0009
    // C3).
    let mut index = Vec::new();
    let id = identifiers::id_pointer();
    // The position counts in u64, the type of `document`, so it needs no conversion.
    for (document, entry) in (0_u64..).zip(&input.documents) {
        let first = validation.errors.len();
        let summary = check_entry(entry, kinds, &mut validation, &mut not_done);
        let found = validation.errors.get_mut(first..).unwrap_or_default();
        let id_refused = found.iter().any(|error| *error.pointer() == id);
        for error in found {
            *error = error.clone().with_document(document);
        }
        if let (Some(_), Some(carried), false) = (summary.kind, &summary.id, id_refused) {
            index.push(Indexed {
                document,
                id: carried.clone(),
                entry,
            });
        }
        validation.documents.push(summary);
    }
    let duplicates = identifiers::duplicates(&index);
    let warnings = identifiers::locations(&index);
    validation.errors.extend(duplicates);
    // By document, then by stage: the stable sort keeps the order of the stages within each
    // document (plan of SPEC-0001, Entry point and validation function).
    validation.errors.sort_by_key(Error::document);
    validation.warnings.extend(warnings);
    match &input.locations {
        Some(folders) => {
            validation.checks.push(checks::ARTIFACT_LOCATIONS);
            if folders.is_empty() {
                validation.warnings.push(no_location());
            }
        }
        None => {
            not_done.insert(NotDone::new(
                checks::ARTIFACT_LOCATIONS,
                NotDoneReason::InputAbsent,
            ));
        }
    }
    validation.not_done = not_done.into_iter().collect();
    validation
}

/// The warning of a project where no folder of a location exists, at the empty `locations`.
fn no_location() -> Error {
    let folders: Vec<&str> = ArtifactFolder::ALL.iter().map(|f| f.path()).collect();
    let message = format!(
        "No folder of stored artifacts exists in the project ({}); the command may not have \
         run at the root of the project.",
        folders.join(", ")
    );
    let pointer = Pointer::root().join("locations");
    Error::new(
        codes::ARTIFACT_LOCATION_MISSING,
        message,
        pointer,
        Reason::Missing,
    )
}

fn not_built(not_done: &mut BTreeSet<NotDone>, check: Check) {
    not_done.insert(NotDone::new(check, NotDoneReason::NotBuilt));
}

/// The failure of the engine when the type `id`, an artifact type or an input kind named by
/// `what`, has no schema loaded: every type of SPEC-0001 has its schema built (`kinds::built`),
/// so one without is a defect of the engine, never a check left out (decided by the maintainer
/// on 2026-11-25).
fn no_schema(what: &str, id: &str) -> Error {
    let check = kinds::schema_check(id);
    Error::internal(format!(
        "The {what} {id} has no schema loaded, so the check {} cannot run.",
        check.id()
    ))
}

/// Checks one entry, adding what it found to `validation`, and gives its summary.
fn check_entry(
    entry: &Entry,
    kinds: &[Loaded],
    validation: &mut Validation,
    not_done: &mut BTreeSet<NotDone>,
) -> DocumentSummary {
    let origin = entry.given.clone();
    let located = |error: Error| entry.locate(error);
    match &entry.body {
        Body::Unreadable { line, message } => {
            let text = format!("The file could not be parsed: {message}");
            let mut error = Error::new(
                codes::ARTIFACT_SCHEMA,
                text,
                Pointer::root(),
                Reason::Syntax,
            );
            // The parser gives the line itself; the input refuses an unreadable entry without
            // a file origin.
            if let Some(Place::File(file)) = &entry.origin {
                error = error.with_location(crate::Source::File {
                    file: file.clone(),
                    line: Some(*line),
                });
            }
            validation.errors.push(error);
            summary(None, None, origin)
        }
        Body::Markdown => {
            not_built(not_done, checks::MARKDOWN);
            summary(None, None, origin)
        }
        Body::Document {
            document,
            input_kind: Some(input_kind),
        } => {
            if let Some(loaded) = kinds.iter().find(|k| k.input == Some(*input_kind)) {
                if let Some(read) = checked(loaded, document, validation, located) {
                    return summary(Some(&read), Some(*input_kind), origin);
                }
            } else {
                let failure = no_schema("input kind", input_kind.id());
                validation.failures.push(failure);
            }
            summary(Some(document), Some(*input_kind), origin)
        }
        Body::Document {
            document,
            input_kind: None,
        } => {
            let at = Pointer::root().join("kind");
            let kind = match document.get("kind").filter(|v| !v.is_null()) {
                None => {
                    let message = "The required field `kind` is missing.".to_owned();
                    validation.errors.push(located(Error::new(
                        codes::ARTIFACT_SCHEMA,
                        message,
                        at,
                        Reason::Missing,
                    )));
                    return summary(Some(document), None, origin);
                }
                Some(Value::String(kind)) => kind,
                Some(_) => {
                    let message = "Expected a string, the type of the artifact.".to_owned();
                    validation.errors.push(located(Error::new(
                        codes::ARTIFACT_SCHEMA,
                        message,
                        at,
                        Reason::Type,
                    )));
                    return summary(Some(document), None, origin);
                }
            };
            let artifact = kinds.iter().find(|k| k.input.is_none() && k.id == kind);
            if let Some(loaded) = artifact {
                if let Some(read) = checked(loaded, document, validation, located) {
                    return summary(Some(&read), None, origin);
                }
            } else if ArtifactKind::from_id(kind).is_some() {
                validation.failures.push(no_schema("artifact type", kind));
            } else {
                let message = format!("The type `{kind}` is not an artifact type.");
                validation.errors.push(located(
                    Error::new(codes::ARTIFACT_SCHEMA, message, at, Reason::Enum)
                        .with_expected(kinds::kinds()),
                ));
            }
            summary(Some(document), None, origin)
        }
    }
}

/// Checks `document` against the schemas of its type `loaded`, adding its errors, located by
/// `located`, or the failure of the engine to `validation`; gives the document read into its
/// contract type when it is valid.
fn checked(
    loaded: &Loaded,
    document: &Map<String, Value>,
    validation: &mut Validation,
    located: impl Fn(Error) -> Error,
) -> Option<Map<String, Value>> {
    match loaded.check(document) {
        Checked::Valid(current) => match read_document(loaded, current) {
            Ok(read) => return Some(read),
            Err(failure) => validation.failures.push(*failure),
        },
        Checked::Refused(errors) => validation.errors.extend(errors.into_iter().map(located)),
        Checked::Failed(failure) => validation.failures.push(*failure),
    }
    None
}

/// The document `current`, an artifact or an input accepted by the current schema of `loaded`,
/// read into its contract type and written again: the form the later checks read. A failure is
/// a defect of the engine (ADR 0030 G1).
fn read_document(
    loaded: &Loaded,
    current: Map<String, Value>,
) -> Result<Map<String, Value>, Box<Error>> {
    let kind = loaded.id;
    match (loaded.reread)(&Value::Object(current)) {
        Ok(Value::Object(read)) => Ok(read),
        Ok(_) => Err(Box::new(Error::internal(format!(
            "A {kind} is not written as an object."
        )))),
        Err(e) => Err(Box::new(Error::internal(format!(
            "A {kind} the schema accepts does not read into its type: {e}."
        )))),
    }
}

fn text(document: Option<&Map<String, Value>>, key: &str) -> Option<String> {
    document?.get(key)?.as_str().map(str::to_owned)
}

fn summary(
    document: Option<&Map<String, Value>>,
    input_kind: Option<InputKind>,
    origin: Option<Origin>,
) -> DocumentSummary {
    // An input has no `kind`, `id` or `status`: members of those names in it are refused, never
    // summarised.
    let artifact = if input_kind.is_some() { None } else { document };
    DocumentSummary {
        kind: text(artifact, "kind").and_then(|k| ArtifactKind::from_id(&k)),
        input_kind,
        id: text(artifact, "id"),
        status: text(artifact, "status"),
        origin,
    }
}

/// The outcome of `validate` on the canonical `input`, with its diagnostics: the input is
/// checked against the input schema of the command and its rules, then the set is validated.
/// No check of this version applies a default.
pub(crate) fn run(input: &Value) -> (Outcome<ValidateData>, Diagnostics) {
    match kinds::built() {
        Ok(kinds) => run_with(input, &kinds),
        Err(failure) => alone(failed(*failure)),
    }
}

/// An outcome with no diagnostics.
fn alone(outcome: Outcome<ValidateData>) -> (Outcome<ValidateData>, Diagnostics) {
    (outcome, Diagnostics::default())
}

/// The failure of `error`, an internal error.
fn failed(error: Error) -> Outcome<ValidateData> {
    Outcome::failed(ErrorReport::single(error)).unwrap_or_else(not_an_outcome)
}

/// The refusal of `report`, every error a refusal of the input.
fn refused(report: ErrorReport) -> Outcome<ValidateData> {
    Outcome::refused(report).unwrap_or_else(not_an_outcome)
}

/// The failure given when a report has no outcome of the contract, which the reasons and codes
/// of the core never write.
fn not_an_outcome() -> Outcome<ValidateData> {
    Outcome::internal("The engine wrote a report that mixes refusals and failures.")
}

/// [`run`] with the types `kinds`.
pub(crate) fn run_with(input: &Value, kinds: &[Kind]) -> (Outcome<ValidateData>, Diagnostics) {
    let Some(snapshot) = VALIDATE.embedded(Part::Schema, VALIDATE.version) else {
        let path = VALIDATE.path(Part::Schema, VALIDATE.version);
        return alone(failed(Error::internal(format!(
            "The schema {path} is not embedded."
        ))));
    };
    let schema = match kinds::load(snapshot) {
        Ok(schema) => schema,
        Err(failure) => return alone(failed(*failure)),
    };
    if let Some(report) = ErrorReport::from_errors(schema.validate(input, COMMAND_INPUT)) {
        return alone(refused(report));
    }
    if let Some(report) = ErrorReport::from_errors(input::rules(input)) {
        return alone(refused(report));
    }
    let input = match input::read(input) {
        Ok(input) => input,
        Err(failure) => return alone(failed(*failure)),
    };
    let mut validation = validate(&input, kinds);
    if let Some(report) = ErrorReport::from_errors(core::mem::take(&mut validation.failures)) {
        return alone(Outcome::failed(report).unwrap_or_else(not_an_outcome));
    }
    let diagnostics = Diagnostics::new(validation.warnings, validation.not_done, validation.checks);
    let outcome = match ErrorReport::from_errors(validation.errors) {
        Some(report) => refused(report),
        None => Outcome::success(ValidateData {
            documents: validation.documents,
        })
        .unwrap_or_else(not_an_outcome),
    };
    (outcome, diagnostics)
}
