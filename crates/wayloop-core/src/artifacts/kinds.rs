//! The artifact types and inputs of SPEC-0001, the versions of their schemas and the
//! migrations between them (R1, R16).
//!
//! The types whose schema is built are the contracts of the areas `artifacts` and `inputs` in
//! the registry of `wayloop-schemas` (ADR 0030 G5), and the inputs of [`INPUTS`], each a further
//! file of the contract of an artifact; the schema of each version is its snapshot embedded in
//! that crate. Every input carries its `schema_version`, as an artifact does (SPEC-0001,
//! Inputs). Each type has one migration per step `v<N>` to `v<N+1>`: a pure function on the
//! document, which also sets `schema_version` to the next version. A document of an earlier
//! version is checked against the schema of its own version, then migrated to the current one,
//! which is the version the later checks read.

use core::num::NonZeroU32;

use serde_json::{Map, Value};
use wayloop_schemas::artifact::ArtifactKind;
use wayloop_schemas::contract::{Area, CONTRACTS, Contract, Embedded, HANDOVER, Part, Reread};
use wayloop_schemas::number::{SAFE_INTEGER, integer, is_safe};
use wayloop_schemas::validate::InputKind;

use super::codes::{ARTIFACT_SCHEMA, HANDOVER_INPUT};
use crate::schema::Schema;
use crate::{Check, Code, Error, Pointer, Reason};

/// The spellings of the artifact types, the accepted values of `kind`, in the order of the
/// enum of the contract.
#[must_use]
pub fn kinds() -> Vec<&'static str> {
    ArtifactKind::ALL.iter().map(|k| k.id()).collect()
}

/// The check of the schema of a type or an input, `schema_<kind>`: in `diagnostics.checks`
/// when the schema is built, in `diagnostics.not_done` otherwise. One check per schema.
#[must_use]
pub fn schema_check(kind: &str) -> Check {
    Check::owned(format!("schema_{kind}"))
}

/// An input whose schema is built (SPEC-0001, Inputs): the file `part` of `contract`, of every
/// version of the contract, with the `code` of its schema errors.
#[derive(Debug, Clone)]
pub struct InputSchema {
    /// The value of `input_kind` that selects it.
    pub kind: InputKind,
    /// The contract that holds its schema.
    pub contract: &'static Contract,
    /// The file of the contract that holds its schema.
    pub part: Part,
    /// The code of its schema errors.
    pub code: Code,
}

/// The inputs whose schema is a further file of the contract of an artifact, built after the
/// inputs of the area `inputs`. The hand-over input is a file of the hand-over contract, and its
/// errors have their own code (SPEC-0001 R22).
pub const INPUTS: [InputSchema; 1] = [InputSchema {
    kind: InputKind::HandoverInput,
    contract: &HANDOVER,
    part: Part::Input,
    code: HANDOVER_INPUT,
}];

/// A migration from one version of a document to the next.
pub type Migration = fn(Map<String, Value>) -> Map<String, Value>;

/// The migrations of each built type, by its `kind` or its `input_kind`; a type absent here has
/// one version.
const MIGRATIONS: &[(&str, &[Migration])] = &[];

/// A type whose schema is built: its versions, `v1` first, and the migrations between them,
/// one fewer than the versions.
#[derive(Debug, Clone)]
pub struct Kind {
    /// The value of `kind`, or of `input_kind` for an input.
    pub id: &'static str,
    /// The input kind, for an input; `None` for an artifact.
    pub input: Option<InputKind>,
    /// The code of its schema errors.
    pub code: Code,
    /// The snapshot of the schema of each version, from `v1`.
    pub versions: Vec<Embedded>,
    /// The migration of each step, from `v1` to `v2`.
    pub migrations: &'static [Migration],
    /// Reads a document of the current version into the contract type of the kind.
    pub reread: Reread,
}

/// The types whose schema is built: the artifacts of the registry, the inputs of the area
/// `inputs` (whose schema errors have the code `artifact_schema`, SPEC-0001 R1), then the
/// inputs of [`INPUTS`], each with the embedded snapshot of every version and the contract
/// type of its schema; an internal error when a snapshot is not embedded, the contract
/// registers no file of that part, or a contract of the area `inputs` names no input kind.
pub(crate) fn built() -> Result<Vec<Kind>, Box<Error>> {
    let artifacts = CONTRACTS
        .iter()
        .filter(|contract| contract.area == Area::Artifacts)
        .map(|contract| kind(contract, Part::Schema, None, ARTIFACT_SCHEMA));
    let registered = CONTRACTS
        .iter()
        .filter(|contract| contract.area == Area::Inputs)
        .map(|contract| {
            let input = InputKind::from_id(contract.name).ok_or_else(|| {
                let folder = contract.folder();
                boxed(format!("The contract {folder} names no input kind."))
            })?;
            kind(contract, Part::Schema, Some(input), ARTIFACT_SCHEMA)
        });
    let files = INPUTS
        .into_iter()
        .map(|input| kind(input.contract, input.part, Some(input.kind), input.code));
    artifacts.chain(registered).chain(files).collect()
}

/// The type whose schema is the file `part` of `contract`, an input when `input` is given.
fn kind(
    contract: &Contract,
    part: Part,
    input: Option<InputKind>,
    code: Code,
) -> Result<Kind, Box<Error>> {
    let id = input.map_or(contract.name, InputKind::id);
    let versions = (1..=contract.version.get())
        .filter_map(NonZeroU32::new)
        .map(|version| {
            contract.embedded(part, version).ok_or_else(|| {
                let path = contract.path(part, version);
                boxed(format!("The schema {path} is not embedded."))
            })
        })
        .collect::<Result<_, _>>()?;
    let migrations = MIGRATIONS
        .iter()
        .find(|(kind, _)| *kind == id)
        .map_or(&[][..], |(_, migrations)| *migrations);
    let reread = contract
        .files
        .iter()
        .find(|file| file.part == part)
        .map(|file| file.registered.reread)
        .ok_or_else(|| {
            boxed(format!(
                "The contract {} registers no file {part}.",
                contract.folder()
            ))
        })?;
    Ok(Kind {
        id,
        input,
        code,
        versions,
        migrations,
        reread,
    })
}

/// A type with its schemas loaded.
#[derive(Debug, Clone)]
pub struct Loaded {
    pub(super) id: &'static str,
    pub(super) input: Option<InputKind>,
    code: Code,
    versions: Vec<Schema>,
    migrations: &'static [Migration],
    pub(super) reread: Reread,
}

/// An error of the engine itself, boxed: `what` failed.
fn boxed(what: String) -> Box<Error> {
    Box::new(Error::internal(what))
}

/// The schema of `snapshot`, parsed and loaded, or the internal error that says why not.
pub(crate) fn load(snapshot: Embedded) -> Result<Schema, Box<Error>> {
    let value: Value = serde_json::from_str(snapshot.text)
        .map_err(|e| boxed(format!("The schema {} is not JSON: {e}.", snapshot.path)))?;
    Schema::load(&value).map_err(|e| {
        boxed(format!(
            "The schema {} cannot be loaded: {e}.",
            snapshot.path
        ))
    })
}

/// Loads the schemas of `kinds`; an internal error when one cannot be loaded or a type has not
/// one migration per step.
pub(crate) fn load_all(kinds: &[Kind]) -> Result<Vec<Loaded>, Box<Error>> {
    kinds
        .iter()
        .map(|kind| {
            if kind.versions.is_empty() || kind.migrations.len() != kind.versions.len() - 1 {
                return Err(boxed(format!(
                    "The type {} has {} versions and {} migrations.",
                    kind.id,
                    kind.versions.len(),
                    kind.migrations.len()
                )));
            }
            let versions = kind
                .versions
                .iter()
                .map(|snapshot| load(*snapshot))
                .collect::<Result<_, _>>()?;
            Ok(Loaded {
                id: kind.id,
                input: kind.input,
                code: kind.code.clone(),
                versions,
                migrations: kind.migrations,
                reread: kind.reread,
            })
        })
        .collect()
}

/// What checking one document of a built type gave.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Checked {
    /// The document is valid; it is given in the current version of its type.
    Valid(Map<String, Value>),
    /// The document is refused, with every error found.
    Refused(Vec<Error>),
    /// The engine failed: a migration gave a document that the current schema refuses.
    Failed(Error),
}

impl Loaded {
    /// The errors of `value` against the schema of the current version, with the code of the
    /// type.
    fn against_current(&self, value: &Value) -> Vec<Error> {
        self.versions
            .last()
            .map(|s| s.validate(value, self.code.clone()))
            .unwrap_or_default()
    }

    /// The known versions, from 1.
    fn known(&self) -> impl Iterator<Item = u64> {
        (1..).take(self.versions.len())
    }

    /// Checks `document`, of this type, against the schema of its `schema_version` and
    /// migrates it to the current version (R16), an artifact or an input alike; a schema error
    /// has the code of the type, `artifact_schema` except for the hand-over input.
    pub(crate) fn check(&self, document: &Map<String, Value>) -> Checked {
        let code = &self.code;
        let at = Pointer::root().join("schema_version");
        let value = Value::Object(document.clone());
        let given = document.get("schema_version").filter(|v| !v.is_null());
        let Some(version) = given.and_then(integer) else {
            // Missing or not an integer: the schema of the current version reports it, with
            // every other error of the document, in one pass (ADR 0009 C3).
            return Checked::Refused(in_stage_order(self.against_current(&value)));
        };
        if !is_safe(version) {
            let message = format!(
                "The version {version} is beyond the accepted range of integers, \
                 plus or minus {SAFE_INTEGER}."
            );
            return Checked::Refused(vec![Error::new(code.clone(), message, at, Reason::Range)]);
        }
        let found = usize::try_from(version)
            .ok()
            .and_then(|v| v.checked_sub(1))
            .and_then(|i| self.versions.get(i).map(|schema| (i, schema)));
        let Some((index, schema)) = found else {
            let known: Vec<u64> = self.known().collect();
            let message = format!(
                "The type {} has no schema version {version}; the known versions are {}.",
                self.id,
                known
                    .iter()
                    .map(u64::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            let error = Error::new(
                super::codes::SCHEMA_VERSION_UNKNOWN,
                message,
                at,
                Reason::Enum,
            )
            .with_expected(known);
            return Checked::Refused(vec![error]);
        };
        let errors = schema.validate(&value, code.clone());
        if !errors.is_empty() {
            return Checked::Refused(in_stage_order(errors));
        }
        let mut current = document.clone();
        for migration in self.migrations.iter().skip(index) {
            current = migration(current);
        }
        // The current schema checks the migrated document: a migration that breaks it is a
        // defect of the engine, never a refusal of the input.
        let latest = self.against_current(&Value::Object(current.clone()));
        if let Some(first) = latest.into_iter().next() {
            return Checked::Failed(Error::internal(format!(
                "The migration of a {} from version {version} gives a document that the \
                 current schema refuses: {first}",
                self.id
            )));
        }
        Checked::Valid(current)
    }
}

/// The errors of one document in the order of the stages of the plan of SPEC-0001: the errors
/// of the schema stage first, in the order of the schema pass, then those of the identifier
/// stage. A format error of `/id` belongs to the identifier stage.
fn in_stage_order(errors: Vec<Error>) -> Vec<Error> {
    let (identifiers, schema): (Vec<Error>, Vec<Error>) = errors
        .into_iter()
        .map(identifier)
        .partition(|e| *e.code() == super::codes::IDENTIFIER_FORMAT);
    schema.into_iter().chain(identifiers).collect()
}

/// `error`, with the code `identifier_format` when it is a format error of `/id` (SPEC-0001
/// R14): one fault, one error, with its own code.
fn identifier(error: Error) -> Error {
    if error.reason() != Reason::Syntax || error.pointer().as_str() != "/id" {
        return error;
    }
    let mut renamed = Error::new(
        super::codes::IDENTIFIER_FORMAT,
        error.message(),
        error.pointer().clone(),
        error.reason(),
    );
    if let Some(fix) = error.fix() {
        renamed = renamed.with_fix(fix.clone());
    }
    renamed
}
