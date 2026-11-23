//! Generator of the contract snapshots (ADR 0030 G4, G6). Not published.
//!
//! For every contract of the registry, `wayloop_schemas::contract::CONTRACTS`, and every file
//! of its current version, [`generate`] runs `wayloop_core::schema::contract_schema`, which
//! checks the rules of the contracts and loads the schema with the interpreter of the core, and
//! then writes its text with `wayloop_schemas::contract::snapshot`, the only source of the text
//! and of the `$id`. The binary prints the result as one JSON document on standard output and
//! writes no file: `scripts/schema_snapshots.py` writes the snapshots, byte for byte, and the
//! history of each contract.
//!
//! The document printed also lists every part a contract may have, with the name of its file
//! at version 1, so that the script recognises every file name the registry can give:
//!
//! ```json
//! {"parts": [{"part": "schema", "first_file": "v1.json"}, ...],
//!  "contracts": [{"area": "envelope", "name": "result", "version": 1, "files": [
//!   {"part": "schema", "direction": "output", "path": "envelope/result/v1.json", "text": "..."}
//! ]}]}
//! ```

use core::fmt;
use core::num::NonZeroU32;

use serde_json::{Value, json};
use wayloop_core::schema::{ContractFault, contract_schema};
use wayloop_schemas::contract::{CONTRACTS, Contract, Part, snapshot};

/// Why a snapshot could not be generated, and of which file.
#[derive(Debug)]
pub enum Fault {
    /// The schema of the file leaves the rules of the contracts or the subset of the core.
    Contract {
        /// The path of the file under `schemas/`.
        path: String,
        /// What the core refused.
        fault: ContractFault,
    },
    /// The text of the snapshot could not be written.
    Text {
        /// The path of the file under `schemas/`.
        path: String,
        /// The error of `serde_json`.
        error: serde_json::Error,
    },
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Fault::Contract { path, fault } => write!(f, "{path}: {fault}"),
            Fault::Text { path, error } => write!(f, "{path}: {error}"),
        }
    }
}

impl std::error::Error for Fault {}

/// The snapshots of the current version of `contract`, as one JSON object.
///
/// # Errors
///
/// The [`Fault`] of the first file whose schema is refused or cannot be written.
pub fn contract(contract: &Contract) -> Result<Value, Fault> {
    let mut files = Vec::with_capacity(contract.files.len());
    for file in contract.files {
        let path = contract.path(file.part, contract.version);
        let schema = match contract_schema(file.registered, file.direction) {
            Ok(schema) => schema,
            Err(fault) => return Err(Fault::Contract { path, fault }),
        };
        let text = match snapshot(schema, &path) {
            Ok(text) => text,
            Err(error) => return Err(Fault::Text { path, error }),
        };
        files.push(json!({
            "part": file.part.id(),
            "direction": file.direction.id(),
            "path": path,
            "text": text,
        }));
    }
    Ok(json!({
        "area": contract.area.id(),
        "name": contract.name,
        "version": contract.version.get(),
        "files": files,
    }))
}

/// Every part a contract may have, with the name of its file at version 1.
#[must_use]
pub fn parts() -> Value {
    Part::ALL
        .iter()
        .map(|part| json!({"part": part.id(), "first_file": part.file_name(NonZeroU32::MIN)}))
        .collect()
}

/// The parts, then the snapshots of the current version of every contract of the registry, in
/// its order.
///
/// # Errors
///
/// The [`Fault`] of the first file whose schema is refused or cannot be written.
pub fn generate() -> Result<Value, Fault> {
    let contracts = CONTRACTS
        .iter()
        .map(contract)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({ "parts": parts(), "contracts": contracts }))
}

#[cfg(test)]
mod tests {
    // A test stops at the line that failed.
    #![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

    use core::num::NonZeroU32;

    use serde_json::{Value, json};
    use wayloop_schemas::contract::{
        Area, CONTRACTS, Contract, ContractError, ContractFile, Direction, Part, Registered, TYPES,
        reread, snapshot,
    };

    use super::{Fault, contract, generate, parts};

    fn named(name: &str) -> Registered {
        *TYPES.iter().find(|t| t.name == name).unwrap()
    }

    #[test]
    fn every_file_is_the_snapshot_of_its_type_at_its_registry_path() {
        let document = generate().unwrap();
        let contracts = document["contracts"].as_array().unwrap();
        let folders: Vec<String> = contracts
            .iter()
            .map(|c| {
                format!(
                    "{}/{}",
                    c["area"].as_str().unwrap(),
                    c["name"].as_str().unwrap()
                )
            })
            .collect();
        // Every contract of the registry, in its order, which is the byte order of the folders.
        let registered: Vec<String> = CONTRACTS.iter().map(Contract::folder).collect();
        assert_eq!(folders, registered);
        for folder in ["artifacts/brief", "commands/validate", "envelope/result"] {
            assert!(folders.iter().any(|f| f == folder), "{folder}");
        }
        let at = |folder: &str| &contracts[folders.iter().position(|f| f == folder).unwrap()];
        let envelope = at("envelope/result");
        assert_eq!(envelope["version"], 1);
        let [file] = envelope["files"].as_array().unwrap().as_slice() else {
            panic!("one file")
        };
        assert_eq!(file["part"], "schema");
        assert_eq!(file["direction"], "output");
        assert_eq!(file["path"], "envelope/result/v1.json");
        let schema = (named("Envelope").schema)(Direction::Output).unwrap();
        assert_eq!(
            file["text"],
            snapshot(schema, "envelope/result/v1.json").unwrap()
        );
        let parts: Vec<(&str, &str, &str)> = at("commands/validate")["files"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| {
                (
                    f["part"].as_str().unwrap(),
                    f["direction"].as_str().unwrap(),
                    f["path"].as_str().unwrap(),
                )
            })
            .collect();
        assert_eq!(
            parts,
            [
                ("schema", "input", "commands/validate/v1.json"),
                ("output", "output", "commands/validate/v1.output.json"),
            ]
        );
        assert_eq!(generate().unwrap(), document, "the same every run");
    }

    #[test]
    fn every_part_is_printed_with_the_name_of_its_first_file() {
        assert_eq!(
            parts(),
            json!([
                {"part": "schema", "first_file": "v1.json"},
                {"part": "output", "first_file": "v1.output.json"},
                {"part": "input", "first_file": "v1.input.json"},
            ])
        );
        assert_eq!(generate().unwrap()["parts"], parts());
    }

    #[test]
    fn every_part_of_a_contract_is_generated_in_its_direction() {
        let validate = Contract {
            area: Area::Commands,
            name: "validate",
            version: NonZeroU32::new(2).unwrap(),
            files: &[
                ContractFile {
                    part: Part::Schema,
                    direction: Direction::Input,
                    registered: Registered {
                        name: "Source",
                        schema: wayloop_schemas::contract::schema_for::<wayloop_schemas::Source>,
                        reread: reread::<wayloop_schemas::Source>,
                    },
                },
                ContractFile {
                    part: Part::Output,
                    direction: Direction::Output,
                    registered: Registered {
                        name: "Envelope",
                        schema: wayloop_schemas::contract::schema_for::<
                            wayloop_schemas::Envelope<Value>,
                        >,
                        reread: reread::<wayloop_schemas::Envelope<Value>>,
                    },
                },
            ],
        };
        let generated = contract(&validate).unwrap();
        let files = generated["files"].as_array().unwrap();
        let paths: Vec<&Value> = files.iter().map(|f| &f["path"]).collect();
        assert_eq!(
            paths,
            [
                &json!("commands/validate/v2.json"),
                &json!("commands/validate/v2.output.json")
            ]
        );
        assert_eq!(files[0]["direction"], "input");
        assert_eq!(files[1]["part"], "output");
        let input: Value = serde_json::from_str(files[0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(
            input["additionalProperties"], false,
            "an input is generated closed"
        );
    }

    /// A type whose schema leaves the subset of the core: `pattern` is not in it.
    #[allow(
        clippy::unnecessary_wraps,
        reason = "the signature of `Registered::schema`"
    )]
    fn refused(_: Direction) -> Result<Value, ContractError> {
        Ok(json!({"description": "d", "type": "object", "pattern": "a"}))
    }

    #[test]
    fn a_refused_schema_names_its_file_and_stops_the_generator() {
        let broken = Contract {
            area: Area::Artifacts,
            name: "broken",
            version: NonZeroU32::MIN,
            files: &[ContractFile {
                part: Part::Schema,
                direction: Direction::Output,
                registered: Registered {
                    name: "Broken",
                    schema: refused,
                    reread: reread::<Value>,
                },
            }],
        };
        let fault = contract(&broken).unwrap_err();
        assert!(matches!(fault, Fault::Contract { .. }));
        assert!(
            fault.to_string().starts_with("artifacts/broken/v1.json: "),
            "{fault}"
        );
    }
}
