//! Where the artifacts of a project are stored, and the bounds of a stored file (SPEC-0001,
//! Where artifacts live; plan of SPEC-0001, Boundary).
//!
//! Which files are artifacts is a rule of the specification, so it is decided here; walking
//! the folders and reading the files is I/O, done by `wayloop-runtime`. The runtime lists the
//! files below each [`Location`] down to its [`Location::depth`] and passes their paths to
//! [`discovered`], which keeps the paths of artifacts, in their one spelling, sorted by bytes;
//! or it passes the paths a person named to [`given`]. Either way the set has one order on
//! every platform, so that the canonical input of `validate` does too (SPEC-0004 R15).
//!
//! The ADRs of the project are not artifacts, but the check `adr_citations` needs their
//! numbers: the runtime lists the files of [`ADR_FOLDER`] and passes their paths to
//! [`adr_numbers`], which keeps the numbers of the files named as ADRs.
//!
//! A listing is bounded: the runtime keeps at most [`MAX_LISTED`] paths that [`is_stored`] or
//! [`is_adr`] accepts, and stops there, so a folder of any size takes bounded memory.
//!
//! The bounds are those of the files the runtime reads and of the documents it builds from
//! them, each defined here once, every size in bytes as a `usize`:
//!
//! - [`MAX_FILES`]: a set of more files is refused, discovered or named.
//! - [`MAX_FILE_BYTES`]: a larger file is not read beyond the bound and not loaded.
//! - [`MAX_NESTING`]: the depth of the values of one parsed document.
//! - [`MAX_DOCUMENT_BYTES`]: the counted bytes one parsed document may take, an estimate of
//!   its memory: the text of its scalars and keys, the pointers of its lines,
//!   [`VALUE_COST_BYTES`] for each value and each line, the values its aliases repeat and the
//!   copies kept for its anchors. A small file whose aliases or anchors would expand into a
//!   large value is refused before the value is built.
//! - [`MAX_PROJECT_BYTES`]: the counted bytes of every file read and every document built for
//!   one set; a file beyond it is not loaded.
//!
//! A file refused for a bound becomes an `unreadable` entry of the input, never an abort, so
//! the envelope is always written. Counted bytes estimate the memory of the loaded input; the
//! test `crates/wayloop-runtime/tests/memory.rs` checks on Linux that the resident memory a
//! loaded set adds stays within them.

use core::fmt;
use core::num::NonZeroU64;

use wayloop_schemas::Format;
use wayloop_schemas::validate::ArtifactFolder;
use wayloop_schemas::value::{AdrNumber, ProjectPath};

use crate::schema::format;

/// Most files one set may hold (10,000), discovered or named; a larger set is refused.
pub const MAX_FILES: usize = 10_000;

/// Largest stored file the runtime reads, in bytes (1 MiB); a larger file is refused unread.
pub const MAX_FILE_BYTES: usize = 1 << 20;

/// Deepest nesting of a parsed document: the root is level 1, and each sequence or mapping
/// adds one level. Values within it are written and dropped without exhausting the stack.
pub const MAX_NESTING: usize = 64;

/// The fixed cost counted for every value of a parsed document and for every entry of its
/// lines, in bytes, beyond their text: the JSON value, the node of the map or the slot of the
/// list that holds it, and the allocation of its text. An estimate, checked on Linux by the
/// test `crates/wayloop-runtime/tests/memory.rs` against the resident memory of a set of
/// short keys, the case where it weighs most.
pub const VALUE_COST_BYTES: usize = 128;

/// Most counted bytes one parsed document may take (8 MiB): the text of every scalar and key,
/// the pointer of every value given a line, [`VALUE_COST_BYTES`] for every value and line,
/// every value an alias repeats and every copy kept for an anchor.
pub const MAX_DOCUMENT_BYTES: usize = 8 << 20;

/// Most counted bytes one set may take (32 MiB): every file read, and every document built
/// from them counted as for [`MAX_DOCUMENT_BYTES`]. A file read beyond it is not loaded.
pub const MAX_PROJECT_BYTES: usize = 32 << 20;

// The set holds at least one file of the largest size and its document.
const _: () = assert!(MAX_PROJECT_BYTES >= MAX_FILE_BYTES + MAX_DOCUMENT_BYTES);

/// How a stored file is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Stored {
    /// A YAML file in the normalised form of its type, parsed into a document.
    Yaml,
    /// A living specification in Markdown, passed as text and normalised by the core (R12).
    Markdown,
}

/// The name of the files of a [`Location`], below its folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Name {
    /// `<name>.yaml`, directly in the folder.
    Yaml,
    /// `<NNNN>-<capability>/spec.md`: a folder named by the sequence of the specification
    /// and its capability, then `spec.md`. `_templates/` never matches.
    Specification,
}

/// A folder that holds stored artifacts, and the name of their files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Location {
    folder: ArtifactFolder,
    name: Name,
}

/// The extension of a YAML artifact.
const YAML: &str = ".yaml";
/// The extension of a Markdown file.
const MARKDOWN: &str = ".md";
/// The file of a living specification in its folder.
const SPECIFICATION: &str = "spec.md";

/// Every location of stored artifacts, in the order of SPEC-0001, Where artifacts live:
/// briefs, council reports, plan items before their tracker item exists, and living
/// specifications. Hand-over records are runtime files of the git common folder, outside the
/// project, and are not listed.
pub const LOCATIONS: [Location; 4] = [
    Location {
        folder: ArtifactFolder::PlanBriefs,
        name: Name::Yaml,
    },
    Location {
        folder: ArtifactFolder::PlanCouncil,
        name: Name::Yaml,
    },
    Location {
        folder: ArtifactFolder::PlanItems,
        name: Name::Yaml,
    },
    Location {
        folder: ArtifactFolder::Specs,
        name: Name::Specification,
    },
];

impl Location {
    /// The folder; its `path` is relative to the root of the project, with `/` separators, and
    /// the input of `validate` lists the folders that exist.
    #[must_use]
    pub const fn folder(&self) -> ArtifactFolder {
        self.folder
    }

    /// How many levels below the folder its files are: 1 for a file in the folder, 2 for a
    /// file in a folder of the folder. The runtime lists no deeper.
    #[must_use]
    pub const fn depth(&self) -> usize {
        match self.name {
            Name::Yaml => 1,
            Name::Specification => 2,
        }
    }

    /// How its files are read.
    #[must_use]
    pub const fn stored(&self) -> Stored {
        match self.name {
            Name::Yaml => Stored::Yaml,
            Name::Specification => Stored::Markdown,
        }
    }

    /// Whether `below`, the segments of a path below the folder, names a file of this location.
    fn holds(self, below: &[&str]) -> bool {
        match (self.name, below) {
            (Name::Yaml, [file]) => file.len() > YAML.len() && file.ends_with(YAML),
            (Name::Specification, [folder, file]) => {
                *file == SPECIFICATION && is_specification_folder(folder)
            }
            _ => false,
        }
    }

    /// The identifier that `below`, the segments of a path of a file of this location, names:
    /// the name of a YAML file without its extension, `SPEC-` and the number of the folder of
    /// a specification (SPEC-0001, Where artifacts live), whatever that number is.
    fn names(self, below: &[&str]) -> Option<String> {
        match (self.name, below) {
            (Name::Yaml, [file]) if self.holds(below) => file.strip_suffix(YAML).map(str::to_owned),
            (Name::Specification, [folder, file]) if *file == SPECIFICATION => {
                folder_identifier(folder)
            }
            _ => None,
        }
    }

    /// The segments of `path` below the folder of this location, if it is below it.
    fn below(self, path: &str) -> Option<Vec<&str>> {
        let below = path.strip_prefix(self.folder.path())?.strip_prefix('/')?;
        Some(below.split('/').collect())
    }
}

/// The identifier of the specification that `folder` is named after, when it is
/// `<number>-<capability>`: ASCII digits, a hyphen and a capability of at least one character;
/// `SPEC-` and the number, in whatever form it is written.
fn folder_identifier(folder: &str) -> Option<String> {
    numbered_parts(folder).map(|(number, _)| format!("SPEC-{number}"))
}

/// The number and the rest of `name` when it is `<number>-<rest>`: ASCII digits, a hyphen and
/// a rest of at least one character, both as written. The folder of a specification is
/// `<number>-<capability>`, the file of an ADR `<number>-<slug>.md`.
fn numbered_parts(name: &str) -> Option<(&str, &str)> {
    let (number, rest) = name.split_once('-')?;
    let digits = !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit());
    (digits && !rest.is_empty()).then_some((number, rest))
}

/// Whether `folder` is `<NNNN>-<capability>`: a folder named after a specification whose
/// number is the canonical sequence of an identifier.
fn is_specification_folder(folder: &str) -> bool {
    folder_identifier(folder).is_some_and(|id| format::accepts(Format::SpecId, &id))
}

/// The identifier that the path of a stored artifact names, `path` relative to the root of the
/// project: the file name of a brief, a council report or a plan item, the folder number of a
/// living specification; `None` for a path outside the [`LOCATIONS`]. The identity of an
/// artifact is its `id`; a path that names another one gives a warning (SPEC-0001).
#[must_use]
pub(crate) fn named_identifier(path: &str) -> Option<String> {
    LOCATIONS.iter().find_map(|location| {
        let below = location.below(path)?;
        location.names(&below)
    })
}

/// The capability of the living specification stored at `path`, the `<capability>` of
/// `specs/<number>-<capability>/spec.md`, as written (SPEC-0001, Markdown form of a living
/// specification); `None` for any other path, such as a Markdown file named outside a folder
/// of a specification.
pub(crate) fn specification_capability(path: &str) -> Option<&str> {
    let below = path
        .strip_prefix(ArtifactFolder::Specs.path())?
        .strip_prefix('/')?;
    match below.split('/').collect::<Vec<_>>().as_slice() {
        [folder, file] if *file == SPECIFICATION => numbered_parts(folder).map(|(_, c)| c),
        _ => None,
    }
}

/// A stored file to read: its path in the project and how it is read.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StoredFile {
    /// The path relative to the root of the project, with `/` separators.
    pub path: ProjectPath,
    /// How it is read.
    pub stored: Stored,
}

/// The stored files among `found`, paths of files relative to the root of the project with
/// `/` separators: those of a [`Location`], once each, sorted by the bytes of their path.
///
/// # Errors
///
/// [`Refused::TooManyFiles`] when they are more than [`MAX_FILES`].
pub fn discovered(found: impl IntoIterator<Item = String>) -> Result<Vec<StoredFile>, Refused> {
    let files = found.into_iter().filter_map(|path| {
        let stored = located(&path)?;
        Some(StoredFile {
            path: ProjectPath::new(path),
            stored,
        })
    });
    bounded(set(files))
}

/// Whether `path`, relative to the root of the project with `/` separators, is the file of a
/// stored artifact, which [`discovered`] keeps.
#[must_use]
pub fn is_stored(path: &str) -> bool {
    located(path).is_some()
}

/// The most paths the runtime keeps from the listing of one set, stored artifacts
/// ([`is_stored`]) or ADRs ([`is_adr`]): one more than [`MAX_FILES`], so that a listing that
/// reaches it is refused with [`Refused::TooManyFiles`] and the runtime lists no further.
pub const MAX_LISTED: usize = MAX_FILES + 1;

/// `files`, when they are no more than [`MAX_FILES`].
fn bounded(files: Vec<StoredFile>) -> Result<Vec<StoredFile>, Refused> {
    if files.len() > MAX_FILES {
        return Err(Refused::TooManyFiles);
    }
    Ok(files)
}

/// How a file named `path` is read, by its extension: YAML for `.yaml`, Markdown for `.md`;
/// `None` for any other. The runtime applies it to a named path and to the file it resolves
/// to, so that a link cannot turn another file into an artifact.
#[must_use]
pub fn stored_by_extension(path: &str) -> Option<Stored> {
    if path.ends_with(YAML) {
        Some(Stored::Yaml)
    } else if path.ends_with(MARKDOWN) {
        Some(Stored::Markdown)
    } else {
        None
    }
}

/// How the file at `path` is read when it is in a [`Location`].
fn located(path: &str) -> Option<Stored> {
    if !format::accepts(Format::ProjectPath, path) {
        return None;
    }
    LOCATIONS.iter().find_map(|location| {
        let below = location.below(path)?;
        location.holds(&below).then_some(location.stored())
    })
}

/// `files` once each, sorted by the bytes of their path.
fn set(files: impl IntoIterator<Item = StoredFile>) -> Vec<StoredFile> {
    let mut files: Vec<StoredFile> = files.into_iter().collect();
    files.sort_by(|a, b| a.path.as_str().cmp(b.path.as_str()));
    files.dedup_by(|a, b| a.path == b.path);
    files
}

/// The folder of the ADRs of the project, relative to its root with `/` separators (plan of
/// SPEC-0001, Q3). Fixed: the folder of this repository; a configured folder comes with the
/// configuration (ADR 0010). When it does not exist, `adrs` is not given and the check
/// `adr_citations` is not done.
pub const ADR_FOLDER: &str = "docs/en/project/adr";

/// How many levels below [`ADR_FOLDER`] the files of the ADRs are: 1, directly in it. The
/// runtime lists no deeper.
pub const ADR_DEPTH: usize = 1;

/// The numbers of the ADRs among `found`, paths of files relative to the root of the project
/// with `/` separators, as the runtime lists them below [`ADR_FOLDER`]: once each, in
/// ascending order, for `adrs` of the input of `validate` (SPEC-0001 Q5; #344, decided by the
/// maintainer on 2026-11-25). A file of an ADR is `<number>-<slug>.md` directly in the folder
/// ([`is_adr`]); any other file, such as the index `index.md`, is not an ADR and is ignored.
///
/// The rule has a copy in `scripts/_adr.py`, for the checks of the documentation; the cases of
/// `tests/process/adr_names.json` keep the two equal.
///
/// # Errors
///
/// [`Refused::TooManyFiles`] when more than [`MAX_FILES`] files are ADRs.
pub fn adr_numbers(found: impl IntoIterator<Item = String>) -> Result<Vec<AdrNumber>, Refused> {
    let mut numbers = Vec::new();
    for number in found.into_iter().filter_map(|path| adr_number(&path)) {
        if numbers.len() == MAX_FILES {
            return Err(Refused::TooManyFiles);
        }
        numbers.push(number);
    }
    numbers.sort_unstable();
    numbers.dedup();
    Ok(numbers)
}

/// Whether `path`, relative to the root of the project with `/` separators, is the file of an
/// ADR: `<number>-<slug>.md` directly in [`ADR_FOLDER`], where the number is a sequence from
/// `0001` (four digits, or five or more without a leading zero, as in an identifier) and the
/// slug has at least one character.
#[must_use]
pub fn is_adr(path: &str) -> bool {
    adr_number(path).is_some()
}

/// The number of the ADR whose file is at `path`, when it is one (see [`is_adr`]).
fn adr_number(path: &str) -> Option<AdrNumber> {
    let name = path.strip_prefix(ADR_FOLDER)?.strip_prefix('/')?;
    let (number, slug) = numbered_parts(name.strip_suffix(MARKDOWN)?)?;
    if !format::is_sequence(number) || slug.contains('/') {
        return None;
    }
    let number = NonZeroU64::new(number.parse().ok()?)?;
    Some(AdrNumber::new(number))
}

/// Why a path, or a set of paths, is not read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    /// The path is absolute, has a `..` segment or a `\`, or reaches a file outside the root
    /// of the project through a symbolic link.
    OutsideTheProject,
    /// The file is neither YAML (`.yaml`) nor Markdown (`.md`), as named or once its links are
    /// resolved.
    NotAnArtifactFile,
    /// The set holds more than [`MAX_FILES`] files.
    TooManyFiles,
}

impl Refused {
    /// What is wrong, for a person, without the path (SPEC-0004 R15: the envelope holds no
    /// absolute path).
    #[must_use]
    pub fn message(self) -> String {
        match self {
            Refused::OutsideTheProject => "A path must be relative to the root of the project, \
                 the current folder, without `..` segments or `\\` characters, and reach a file \
                 inside it, not through a link that leaves it."
                .to_owned(),
            Refused::NotAnArtifactFile => "A path must name a YAML file (.yaml) or a Markdown \
                 file (.md), also once its links are resolved."
                .to_owned(),
            Refused::TooManyFiles => {
                format!(
                    "A set holds at most {MAX_FILES} files: name the files to validate in parts."
                )
            }
        }
    }
}

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message())
    }
}

/// The files of the paths a person named, each given as its segments as the platform splits
/// it (`.` for the current folder, `..` for the parent, an empty segment for the root): once
/// each, sorted by the bytes of their path, read as YAML or as Markdown by their extension,
/// wherever they are in the project.
///
/// # Errors
///
/// The first path refused, with its index, when one is outside the project or is not an
/// artifact file; [`Refused::TooManyFiles`], with no index, when the files are more than
/// [`MAX_FILES`].
pub fn given<S: AsRef<str>>(
    paths: impl IntoIterator<Item = impl IntoIterator<Item = S>>,
) -> Result<Vec<StoredFile>, (Option<usize>, Refused)> {
    let mut files = Vec::new();
    for (index, segments) in paths.into_iter().enumerate() {
        let segments: Vec<S> = segments.into_iter().collect();
        let path = segments
            .iter()
            .map(AsRef::as_ref)
            .filter(|segment| *segment != ".")
            .collect::<Vec<_>>()
            .join("/");
        if path.is_empty() || !format::accepts(Format::ProjectPath, &path) {
            return Err((Some(index), Refused::OutsideTheProject));
        }
        let Some(stored) = stored_by_extension(&path) else {
            return Err((Some(index), Refused::NotAnArtifactFile));
        };
        files.push(StoredFile {
            path: ProjectPath::new(path),
            stored,
        });
    }
    bounded(set(files)).map_err(|refused| (None, refused))
}

#[cfg(test)]
mod tests {
    use super::{
        ADR_FOLDER, LOCATIONS, MAX_DOCUMENT_BYTES, MAX_FILE_BYTES, MAX_FILES, MAX_NESTING,
        MAX_PROJECT_BYTES, Refused, Stored, StoredFile, VALUE_COST_BYTES, adr_numbers, discovered,
        given, is_adr, is_stored, named_identifier, stored_by_extension,
    };
    use wayloop_schemas::value::ProjectPath;

    fn file(path: &str, stored: Stored) -> StoredFile {
        StoredFile {
            path: ProjectPath::new(path),
            stored,
        }
    }

    #[test]
    fn the_locations_are_those_of_the_specification() {
        let listed: Vec<(&str, usize, Stored)> = LOCATIONS
            .iter()
            .map(|l| (l.folder().path(), l.depth(), l.stored()))
            .collect();
        assert_eq!(
            listed,
            [
                ("plan/briefs", 1, Stored::Yaml),
                ("plan/council", 1, Stored::Yaml),
                ("plan/items", 1, Stored::Yaml),
                ("specs", 2, Stored::Markdown),
            ]
        );
    }

    #[test]
    fn discovery_keeps_the_artifacts_sorted_by_bytes_with_relative_slash_paths() {
        let found = [
            "specs/0001-artifact-model/spec.md",
            "plan/items/TASK-0107.yaml",
            "plan/briefs/README.md",
            "plan/briefs/BRIEF-0002.yaml",
            "plan/briefs/BRIEF-0001.yaml",
            "plan/council/COUNCIL-0001.yaml",
            "plan/briefs/BRIEF-0001.yaml",
            "plan/briefs/.yaml",
            "plan/briefs/nested/BRIEF-0003.yaml",
            "plan/briefs/BRIEF-0004.yml",
            "plan/other/BRIEF-0005.yaml",
            "plan/briefsx/BRIEF-0006.yaml",
            "specs/_templates/spec.md",
            "specs/README.md",
            "specs/constitution.md",
            "specs/0002-state-machine/plan.md",
            "specs/0003-ledger/deep/spec.md",
            "specs/12345-large/spec.md",
            "specs/01234-padded/spec.md",
            "specs/0004-/spec.md",
            "specs/0005/spec.md",
            "specs/abcd-letters/spec.md",
            "plan\\briefs\\BRIEF-0007.yaml",
            "/plan/briefs/BRIEF-0008.yaml",
            "plan/briefs/../briefs/BRIEF-0009.yaml",
        ];
        let kept = discovered(found.iter().map(ToString::to_string)).unwrap();
        assert_eq!(
            kept,
            [
                file("plan/briefs/BRIEF-0001.yaml", Stored::Yaml),
                file("plan/briefs/BRIEF-0002.yaml", Stored::Yaml),
                file("plan/council/COUNCIL-0001.yaml", Stored::Yaml),
                file("plan/items/TASK-0107.yaml", Stored::Yaml),
                file("specs/0001-artifact-model/spec.md", Stored::Markdown),
                file("specs/12345-large/spec.md", Stored::Markdown),
            ]
        );
    }

    #[test]
    fn discovery_sorts_by_bytes_not_by_letters() {
        let kept = discovered(
            [
                "plan/items/b.yaml",
                "plan/items/B.yaml",
                "plan/items/a.yaml",
            ]
            .map(String::from),
        )
        .unwrap();
        let paths: Vec<&str> = kept.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(
            paths,
            [
                "plan/items/B.yaml",
                "plan/items/a.yaml",
                "plan/items/b.yaml"
            ]
        );
    }

    #[test]
    fn named_paths_are_read_by_their_extension_once_each_in_byte_order() {
        let files = given([
            vec!["specs", "README.md"],
            vec![".", "plan", "briefs", "BRIEF-0001.yaml"],
            vec!["notes", "a.yaml"],
            vec!["plan", "briefs", "BRIEF-0001.yaml"],
        ])
        .unwrap();
        assert_eq!(
            files,
            [
                file("notes/a.yaml", Stored::Yaml),
                file("plan/briefs/BRIEF-0001.yaml", Stored::Yaml),
                file("specs/README.md", Stored::Markdown),
            ]
        );
        assert_eq!(given(Vec::<Vec<&str>>::new()), Ok(Vec::new()));
    }

    #[test]
    fn a_named_path_outside_the_project_or_not_an_artifact_is_refused_with_its_index() {
        let outside = Refused::OutsideTheProject;
        for (segments, refused) in [
            (vec!["", "etc", "a.yaml"], outside),
            (vec!["..", "a.yaml"], outside),
            (vec!["plan", "..", "..", "a.yaml"], outside),
            (vec!["C:", "a.yaml"], outside),
            (vec!["plan", "..", "plan", "a.yaml"], outside),
            (vec!["plan", "a\\b.yaml"], outside),
            (vec!["."], outside),
            (vec![], outside),
            (vec!["plan", "a.yml"], Refused::NotAnArtifactFile),
            (vec!["plan", "a.json"], Refused::NotAnArtifactFile),
        ] {
            let paths = [vec!["plan", "ok.yaml"], segments.clone()];
            assert_eq!(given(paths), Err((Some(1), refused)), "{segments:?}");
        }
        for refused in [
            Refused::OutsideTheProject,
            Refused::NotAnArtifactFile,
            Refused::TooManyFiles,
        ] {
            assert_eq!(refused.to_string(), refused.message());
            assert!(refused.message().starts_with('A'), "{refused:?}");
        }
        // The message states the rule a refused path breaks.
        let rule = Refused::OutsideTheProject.message();
        for part in ["relative", "`..`", "`\\`", "link"] {
            assert!(rule.contains(part), "{part}: {rule}");
        }
    }

    #[test]
    fn the_bounds_of_a_stored_file() {
        assert_eq!(MAX_FILE_BYTES, 1_048_576);
        assert_eq!(MAX_NESTING, 64);
        assert_eq!(MAX_DOCUMENT_BYTES, 8_388_608);
        assert_eq!(MAX_PROJECT_BYTES, 33_554_432);
        assert_eq!(MAX_FILES, 10_000);
        assert_eq!(VALUE_COST_BYTES, 128);
    }

    #[test]
    fn a_set_of_more_than_max_files_is_refused_discovered_or_named() {
        let paths = |n: usize| -> Vec<String> {
            (0..n).map(|i| format!("plan/items/i{i:05}.yaml")).collect()
        };
        assert_eq!(discovered(paths(MAX_FILES)).unwrap().len(), MAX_FILES);
        assert_eq!(discovered(paths(MAX_FILES + 1)), Err(Refused::TooManyFiles));
        // Paths that are not artifacts and repeated paths do not count.
        let mut found = paths(MAX_FILES);
        found.extend(["plan/items/README.md".to_owned(), found[0].clone()]);
        assert_eq!(discovered(found).unwrap().len(), MAX_FILES);
        let named =
            |n: usize| -> Vec<Vec<String>> { paths(n).into_iter().map(|p| vec![p]).collect() };
        assert_eq!(given(named(MAX_FILES)).unwrap().len(), MAX_FILES);
        assert_eq!(
            given(named(MAX_FILES + 1)),
            Err((None, Refused::TooManyFiles))
        );
        assert!(
            Refused::TooManyFiles
                .message()
                .contains(&MAX_FILES.to_string())
        );
    }

    #[test]
    fn a_stored_path_names_the_identifier_of_its_artifact() {
        // SPEC-0001, Where artifacts live: the file name of a YAML artifact, the folder number
        // of a living specification; no identifier outside the locations.
        for (path, named) in [
            ("plan/briefs/BRIEF-0001.yaml", Some("BRIEF-0001")),
            ("plan/council/COUNCIL-0002.yaml", Some("COUNCIL-0002")),
            ("plan/items/TASK-10000.yaml", Some("TASK-10000")),
            ("plan/items/notes.yaml", Some("notes")),
            ("specs/0001-artifact-model/spec.md", Some("SPEC-0001")),
            ("specs/12-x/spec.md", Some("SPEC-12")),
            ("specs/notes-draft/spec.md", None),
            ("specs/0001-/spec.md", None),
            ("specs/-x/spec.md", None),
            ("plan/briefs/deeper/BRIEF-0001.yaml", None),
            ("plan/briefs/README.md", None),
            ("specs/0001-artifact-model/plan.md", None),
            ("specs/0001-artifact-model/examples/brief.valid.yaml", None),
            ("examples/BRIEF-0001.yaml", None),
        ] {
            assert_eq!(named_identifier(path).as_deref(), named, "{path}");
        }
    }

    #[test]
    fn the_adrs_are_the_numbered_markdown_files_directly_in_their_folder() {
        // Plan of SPEC-0001, Q3; #344: one file per ADR, `<number>-<slug>.md`, the number a
        // sequence from 0001, once each number in ascending order. The cases of the rule shared
        // with `scripts/_adr.py` are read by a test of the runtime, which may read files.
        let found = [
            "docs/en/project/adr/10000-five-digits.md",
            "docs/en/project/adr/0014-specifications-by-capability.md",
            "docs/en/project/adr/0002-tracker-as-database.md",
            "docs/en/project/adr/0014-again.md",
            "docs/en/project/adr/index.md",
            "docs/en/project/adr/01234-leading-zero.md",
            "docs/en/project/adr/deeper/0020-x.md",
            "docs/en/project/adrs/0021-x.md",
            "docs/it/project/adr/0022-x.md",
            "0023-x.md",
        ];
        let read: Vec<u64> = adr_numbers(found.map(String::from))
            .unwrap()
            .into_iter()
            .map(|n| n.get().get())
            .collect();
        assert_eq!(read, [2, 14, 10_000]);
        assert_eq!(adr_numbers(Vec::new()), Ok(Vec::new()));
    }

    #[test]
    fn more_than_max_files_adrs_are_refused() {
        let names = |n: usize| (1..=n).map(|i| format!("{ADR_FOLDER}/{i:04}-x.md"));
        assert_eq!(
            adr_numbers(names(MAX_FILES)).map(|n| n.len()),
            Ok(MAX_FILES)
        );
        assert_eq!(
            adr_numbers(names(MAX_FILES + 1)),
            Err(Refused::TooManyFiles)
        );
    }

    #[test]
    fn the_listing_keeps_only_the_files_the_core_reads() {
        assert!(is_stored("plan/items/TASK-0001.yaml"));
        assert!(!is_stored("plan/items/README.md"));
        assert!(!is_adr(&format!("{ADR_FOLDER}/index.md")));
    }

    #[test]
    fn the_extension_tells_how_a_file_is_read() {
        assert_eq!(stored_by_extension("a/b.yaml"), Some(Stored::Yaml));
        assert_eq!(stored_by_extension("b.md"), Some(Stored::Markdown));
        for other in ["a/.env", "a.yml", "a.yaml.bak", "md", ""] {
            assert_eq!(stored_by_extension(other), None, "{other}");
        }
    }
}
