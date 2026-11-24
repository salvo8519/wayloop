//! The registry of the contracts (ADR 0030 G5): every contract with its area, its name, its
//! current version and the types of its files. The generator (`wayloop-schemagen`), the
//! snapshots embedded in this crate and, later, the kinds of the core are built from
//! [`CONTRACTS`]; `scripts/schema_snapshots.py` refuses a folder under `schemas/` that it does
//! not list.

use core::num::NonZeroU32;

use super::{Direction, ENVELOPE, Registered, VALIDATE_OUTPUT, reread, schema_for};
use crate::ENVELOPE_VERSION;
use crate::artifact::{
    ArtifactKind, Brief, Change, CouncilReport, Epic, Gate, Handover, HandoverInput, Milestone,
    Specification, Story, Task,
};
use crate::input::{Commit, PullRequest};
use crate::validate::{InputKind, ValidateInput};

crate::closed_enum! {
    /// The area of a contract, the first folder of its path under `schemas/`.
    pub enum Area {
        /// A document of the artifact model, read and written by the engine.
        Artifacts = "artifacts",
        /// A document a person or an agent hands to the engine, such as a hand-over.
        Inputs = "inputs",
        /// The input of a command and its envelope with the data of the command.
        Commands = "commands",
        /// The envelope of every command, with data of any shape (ADR 0009 C1).
        Envelope = "envelope",
    }
}

crate::closed_enum! {
    /// One file of a version of a contract. The spelling names the file: `v<N>.json` for the
    /// schema, `v<N>.<spelling>.json` for the others.
    pub enum Part {
        /// The schema of the contract: the input of a command, an artifact, the envelope.
        Schema = "schema",
        /// The output of a command: the envelope with the data of that command.
        Output = "output",
        /// The input of a document handed to the engine, such as a hand-over (SPEC-0001).
        Input = "input",
    }
}

impl Part {
    /// The name of the file of this part for `version`, for example `v1.output.json`.
    #[must_use]
    pub fn file_name(self, version: NonZeroU32) -> String {
        match self {
            Part::Schema => format!("v{version}.json"),
            other => format!("v{version}.{other}.json"),
        }
    }
}

/// One file of a contract: its part, the direction of its schema and the type that generates
/// it.
#[derive(Debug, Clone, Copy)]
pub struct ContractFile {
    /// Which file of the version.
    pub part: Part,
    /// Whether the engine reads the document (closed objects) or writes it (open objects).
    pub direction: Direction,
    /// The contract type whose schema the file holds.
    pub registered: Registered,
}

/// A contract: where its snapshots live, its current version and the types of its files.
#[derive(Debug, Clone, Copy)]
pub struct Contract {
    /// The first folder of its path under `schemas/`.
    pub area: Area,
    /// The second folder: lower-case letters, digits and underscores, from a letter.
    pub name: &'static str,
    /// The version the generator writes; earlier versions are frozen files (ADR 0030 G5).
    pub version: NonZeroU32,
    /// The files of every version, at most one per part.
    pub files: &'static [ContractFile],
}

impl Contract {
    /// The folder of the contract under `schemas/`, for example `envelope/result`.
    #[must_use]
    pub fn folder(&self) -> String {
        format!("{}/{}", self.area, self.name)
    }

    /// The path under `schemas/` of the file of `part` for `version`, for example
    /// `envelope/result/v1.json`; the `$id` of the snapshot is this path after
    /// [`PUBLISHED_AT`](super::PUBLISHED_AT).
    #[must_use]
    pub fn path(&self, part: Part, version: NonZeroU32) -> String {
        format!("{}/{}", self.folder(), part.file_name(version))
    }

    /// The embedded snapshot of the file `part` of `version`; `None` when it is not embedded.
    #[must_use]
    pub fn embedded(&self, part: Part, version: NonZeroU32) -> Option<Embedded> {
        let path = self.path(part, version);
        super::EMBEDDED.iter().find(|e| e.path == path).copied()
    }
}

/// The contract of a document the engine reads, an artifact type or an input: its schema is an
/// input (closed). `$kinds::$t` is the value that selects the schema, the `kind` of an artifact
/// (`ArtifactKind`) or the `input_kind` of the entry of an input (`InputKind`); its spelling is
/// the name of the contract, and the current version is the `schema_version` a current document
/// of the type carries. `$t` names both that value and the contract type, which share their
/// name. Each further file, `$part => $other`, holds a document the engine also reads, such as
/// the hand-over input, so it is closed too.
macro_rules! document {
    ($area:ident, $kinds:ident :: $t:ident, $version:expr $(, $part:ident => $other:ty)*) => {
        Contract {
            area: Area::$area,
            name: $kinds::$t.id(),
            version: $version,
            files: &[
                ContractFile {
                    part: Part::Schema,
                    direction: Direction::Input,
                    registered: registered!($t),
                },
                $(ContractFile {
                    part: Part::$part,
                    direction: Direction::Input,
                    registered: registered!($other),
                },)*
            ],
        }
    };
}

/// The brief (SPEC-0001, Brief).
pub const BRIEF: Contract = document!(Artifacts, ArtifactKind::Brief, NonZeroU32::MIN);

/// The council report (SPEC-0001, Council report).
pub const COUNCIL_REPORT: Contract = document!(Artifacts, ArtifactKind::CouncilReport, NonZeroU32::MIN);

/// The living specification (SPEC-0001, Living specification), in normalised form.
pub const SPECIFICATION: Contract = document!(Artifacts, ArtifactKind::Specification, NonZeroU32::MIN);

/// The change delta (SPEC-0001, Change delta).
pub const CHANGE: Contract = document!(Artifacts, ArtifactKind::Change, NonZeroU32::MIN);

/// The gate issue (SPEC-0001, Gate issue).
pub const GATE: Contract = document!(Artifacts, ArtifactKind::Gate, NonZeroU32::MIN);

/// The milestone (SPEC-0001, Milestone).
pub const MILESTONE: Contract = document!(Artifacts, ArtifactKind::Milestone, NonZeroU32::MIN);

/// The epic (SPEC-0001, Epic).
pub const EPIC: Contract = document!(Artifacts, ArtifactKind::Epic, NonZeroU32::MIN);

/// The user story (SPEC-0001, User story).
pub const STORY: Contract = document!(Artifacts, ArtifactKind::Story, NonZeroU32::MIN);

/// The task (SPEC-0001, Task).
pub const TASK: Contract = document!(Artifacts, ArtifactKind::Task, NonZeroU32::MIN);

/// The pull request input (SPEC-0001, Inputs).
pub const PULL_REQUEST: Contract = document!(Inputs, InputKind::PullRequest, NonZeroU32::MIN);

/// The commit input (SPEC-0001, Inputs).
pub const COMMIT: Contract = document!(Inputs, InputKind::Commit, NonZeroU32::MIN);

/// The hand-over (SPEC-0001, Hand-over record): the record, `v<N>.json`, and the input an
/// agent submits, `v<N>.input.json`, of one version (R22).
pub const HANDOVER: Contract =
    document!(Artifacts, ArtifactKind::Handover, NonZeroU32::MIN, Input => HandoverInput);

/// The command `validate` (SPEC-0001): its canonical input and its envelope with its data.
pub const VALIDATE: Contract = Contract {
    area: Area::Commands,
    name: "validate",
    version: NonZeroU32::MIN,
    files: &[
        ContractFile {
            part: Part::Schema,
            direction: Direction::Input,
            registered: registered!(ValidateInput),
        },
        ContractFile {
            part: Part::Output,
            direction: Direction::Output,
            registered: VALIDATE_OUTPUT,
        },
    ],
};

/// The envelope `envelope/result`: the envelope that wraps the result of every command, with
/// data of any shape, at the version of `meta.envelope_version`. The output snapshot of each
/// command repeats it with the data of that command.
const RESULT: Contract = Contract {
    area: Area::Envelope,
    name: "result",
    version: ENVELOPE_VERSION,
    files: &[ContractFile {
        part: Part::Schema,
        direction: Direction::Output,
        registered: ENVELOPE,
    }],
};

/// Every contract, in the byte order of their folders. A contract is never removed and its
/// version never goes back (ADR 0030 G5).
pub const CONTRACTS: &[Contract] = &[
    BRIEF,
    CHANGE,
    COUNCIL_REPORT,
    EPIC,
    GATE,
    HANDOVER,
    MILESTONE,
    SPECIFICATION,
    STORY,
    TASK,
    VALIDATE,
    RESULT,
    COMMIT,
    PULL_REQUEST,
];

/// A snapshot embedded in this crate: its path under `schemas/` and its text, byte for byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Embedded {
    /// The path under `schemas/`, for example `envelope/result/v1.json`.
    pub path: &'static str,
    /// The text of the snapshot, as committed.
    pub text: &'static str,
}

#[cfg(test)]
mod tests {
    use core::num::NonZeroU32;
    use std::collections::BTreeSet;

    use super::{Area, CONTRACTS, Contract, Part};
    use crate::contract::{EMBEDDED, TYPES, snapshot};

    fn version(n: u32) -> NonZeroU32 {
        NonZeroU32::new(n).unwrap()
    }

    /// The contract of `path` under `schemas/`, the version and the part of its file.
    fn parse(path: &str) -> (&'static Contract, NonZeroU32, Part) {
        let (folder, file) = path.rsplit_once('/').unwrap();
        let contract = CONTRACTS
            .iter()
            .find(|c| c.folder() == folder)
            .unwrap_or_else(|| panic!("{path}: no contract has the folder {folder}"));
        let stem = file
            .strip_prefix('v')
            .unwrap()
            .strip_suffix(".json")
            .unwrap();
        let (number, part) = match stem.split_once('.') {
            Some((number, part)) => (number, Part::from_id(part).unwrap()),
            None => (stem, Part::Schema),
        };
        assert!(!number.starts_with('0'), "{path}");
        let v = NonZeroU32::new(number.parse().unwrap()).unwrap();
        assert_eq!(
            contract.path(part, v),
            path,
            "{path} is not written as the registry writes it"
        );
        (contract, v, part)
    }

    #[test]
    fn a_path_is_the_area_the_name_the_version_and_the_part() {
        let contract = Contract {
            area: Area::Commands,
            name: "validate",
            version: version(3),
            files: &[],
        };
        assert_eq!(contract.folder(), "commands/validate");
        assert_eq!(
            contract.path(Part::Schema, version(3)),
            "commands/validate/v3.json"
        );
        assert_eq!(
            contract.path(Part::Output, version(12)),
            "commands/validate/v12.output.json"
        );
        assert_eq!(Part::Input.file_name(version(1)), "v1.input.json");
        let areas: Vec<&str> = Area::ALL.iter().map(|a| a.id()).collect();
        assert_eq!(areas, ["artifacts", "inputs", "commands", "envelope"]);
    }

    #[test]
    fn the_envelope_is_registered_at_the_version_of_the_envelope() {
        let envelope = &super::RESULT;
        assert_eq!(envelope.folder(), "envelope/result");
        assert_eq!(envelope.version, crate::ENVELOPE_VERSION);
        assert_eq!(
            envelope.path(Part::Schema, envelope.version),
            "envelope/result/v1.json"
        );
    }

    #[test]
    fn every_contract_has_one_folder_valid_names_and_registered_types() {
        let folders: Vec<String> = CONTRACTS.iter().map(Contract::folder).collect();
        let mut sorted = folders.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(folders, sorted, "folders in byte order, each once");
        for contract in CONTRACTS {
            let name = contract.name.as_bytes();
            assert!(
                name.first().is_some_and(u8::is_ascii_lowercase),
                "{}",
                contract.name
            );
            assert!(
                name.iter()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'_'),
                "{}",
                contract.name
            );
            assert_ne!(contract.files.len(), 0, "{}", contract.name);
            let parts: BTreeSet<Part> = contract.files.iter().map(|f| f.part).collect();
            assert_eq!(parts.len(), contract.files.len(), "{}", contract.name);
            for file in contract.files {
                assert!(
                    TYPES.iter().any(|t| t.name == file.registered.name),
                    "{} is not in contract::TYPES",
                    file.registered.name
                );
            }
        }
    }

    #[test]
    fn the_embedded_current_versions_are_the_snapshots_of_the_types() {
        // The committed snapshot of every current version, embedded in this crate, is the text
        // the types generate: a type changed without `just generate` fails here.
        for contract in CONTRACTS {
            for file in contract.files {
                let path = contract.path(file.part, contract.version);
                let embedded = contract
                    .embedded(file.part, contract.version)
                    .unwrap_or_else(|| panic!("{path} is not embedded: run `just generate`"));
                let schema = (file.registered.schema)(file.direction).unwrap();
                assert_eq!(
                    embedded.text,
                    snapshot(schema, &path).unwrap(),
                    "{path} differs from its type: run `just generate`"
                );
            }
        }
    }

    #[test]
    fn every_embedded_snapshot_belongs_to_a_version_of_a_registered_contract() {
        let paths: Vec<&str> = EMBEDDED.iter().map(|e| e.path).collect();
        let mut sorted = paths.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(paths, sorted, "paths in byte order, each once");
        for embedded in EMBEDDED {
            let (contract, v, part) = parse(embedded.path);
            assert!(
                v <= contract.version,
                "{}: after the current version",
                embedded.path
            );
            if v == contract.version {
                assert!(
                    contract.files.iter().any(|f| f.part == part),
                    "{}: a part the registry does not list",
                    embedded.path
                );
            }
        }
    }
}
