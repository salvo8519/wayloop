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
//! The bounds are those of the files the runtime reads and of the documents it builds from
//! them, each defined here once:
//!
//! - [`MAX_FILE_BYTES`]: a larger file is not read beyond the bound and not loaded.
//! - [`MAX_NESTING`] and [`MAX_VALUES`]: the depth and the count of the values of one parsed
//!   document, those its aliases repeat included.
//! - [`MAX_DOCUMENT_BYTES`]: the bytes one parsed document may take: the text of its scalars
//!   and keys, the pointers of its lines, the values its aliases repeat and the copies kept
//!   for its anchors. A small file whose aliases or anchors would expand into a large value is
//!   refused before the value is built.
//! - [`MAX_PROJECT_BYTES`]: the bytes of every file read and every document built for one
//!   set; a file beyond it is not read.
//!
//! A file refused for a bound becomes an `unreadable` entry of the input, never an abort, so
//! the envelope is always written.

use core::fmt;

use wayloop_schemas::Format;
use wayloop_schemas::value::ProjectPath;

use crate::schema::format;

/// Largest stored file the runtime reads, in bytes (1 MiB); a larger file is refused unread.
pub const MAX_FILE_BYTES: u64 = 1 << 20;

/// Deepest nesting of a parsed document: the root is level 1, and each sequence or mapping
/// adds one level. Values within it are written and dropped without exhausting the stack.
pub const MAX_NESTING: usize = 64;

/// Most values one parsed document may hold, counting every value an alias repeats: a
/// scalar, a sequence and a mapping count one each.
pub const MAX_VALUES: usize = 100_000;

/// Most bytes one parsed document may take (8 MiB): the text of every scalar and key, the
/// pointer of every value given a line, every value an alias repeats and every copy kept for
/// an anchor.
pub const MAX_DOCUMENT_BYTES: usize = 8 << 20;

/// Most bytes one set may take (64 MiB): every file read, and every document built from them
/// counted as for [`MAX_DOCUMENT_BYTES`]. A file read beyond it is not loaded.
pub const MAX_PROJECT_BYTES: u64 = 64 << 20;

// The set holds at least one file of the largest size and its document.
const _: () = assert!(MAX_PROJECT_BYTES >= MAX_FILE_BYTES + MAX_DOCUMENT_BYTES as u64);

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
    folder: &'static str,
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
        folder: "plan/briefs",
        name: Name::Yaml,
    },
    Location {
        folder: "plan/council",
        name: Name::Yaml,
    },
    Location {
        folder: "plan/items",
        name: Name::Yaml,
    },
    Location {
        folder: "specs",
        name: Name::Specification,
    },
];

impl Location {
    /// The folder, relative to the root of the project, with `/` separators.
    #[must_use]
    pub const fn folder(&self) -> &'static str {
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
    fn holds(&self, below: &[&str]) -> bool {
        match (self.name, below) {
            (Name::Yaml, [file]) => file.len() > YAML.len() && file.ends_with(YAML),
            (Name::Specification, [folder, file]) => {
                *file == SPECIFICATION && is_specification_folder(folder)
            }
            _ => false,
        }
    }
}

/// Whether `folder` is `<NNNN>-<capability>`: the sequence of a specification identifier, a
/// hyphen and a capability of at least one character.
fn is_specification_folder(folder: &str) -> bool {
    folder
        .split_once('-')
        .is_some_and(|(sequence, capability)| {
            !capability.is_empty() && format::accepts(Format::SpecId, &format!("SPEC-{sequence}"))
        })
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
#[must_use]
pub fn discovered(found: impl IntoIterator<Item = String>) -> Vec<StoredFile> {
    let files = found.into_iter().filter_map(|path| {
        let stored = located(&path)?;
        Some(StoredFile {
            path: ProjectPath::new(path),
            stored,
        })
    });
    set(files)
}

/// How the file at `path` is read when it is in a [`Location`].
fn located(path: &str) -> Option<Stored> {
    if !format::accepts(Format::ProjectPath, path) {
        return None;
    }
    LOCATIONS.iter().find_map(|location| {
        let below = path
            .strip_prefix(location.folder)?
            .strip_prefix('/')?
            .split('/')
            .collect::<Vec<_>>();
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

/// Why a path a person named is not read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    /// The path is absolute, has a `..` segment or a `\`, or reaches a file outside the root
    /// of the project through a symbolic link.
    OutsideTheProject,
    /// The file is neither YAML (`.yaml`) nor Markdown (`.md`).
    NotAnArtifactFile,
}

impl Refused {
    /// What is wrong, for a person, without the path (SPEC-0004 R15: the envelope holds no
    /// absolute path).
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Refused::OutsideTheProject => {
                "A path must be relative to the root of the project, the current folder, \
                 without `..` segments or `\\` characters, and reach a file inside it, not \
                 through a link that leaves it."
            }
            Refused::NotAnArtifactFile => {
                "A path must name a YAML file (.yaml) or a Markdown file (.md)."
            }
        }
    }
}

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

/// The files of the paths a person named, each given as its segments as the platform splits
/// it (`.` for the current folder, `..` for the parent, an empty segment for the root): once
/// each, sorted by the bytes of their path, read as YAML or as Markdown by their extension,
/// wherever they are in the project. The first path refused, with its index and the reason,
/// when one is outside the project or is not an artifact file.
///
/// # Errors
///
/// [`Refused`] with the index of the first path refused.
pub fn given<S: AsRef<str>>(
    paths: impl IntoIterator<Item = impl IntoIterator<Item = S>>,
) -> Result<Vec<StoredFile>, (usize, Refused)> {
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
            return Err((index, Refused::OutsideTheProject));
        }
        let stored = if path.ends_with(YAML) {
            Stored::Yaml
        } else if path.ends_with(MARKDOWN) {
            Stored::Markdown
        } else {
            return Err((index, Refused::NotAnArtifactFile));
        };
        files.push(StoredFile {
            path: ProjectPath::new(path),
            stored,
        });
    }
    Ok(set(files))
}

#[cfg(test)]
mod tests {
    use super::{
        LOCATIONS, MAX_DOCUMENT_BYTES, MAX_FILE_BYTES, MAX_NESTING, MAX_PROJECT_BYTES, MAX_VALUES,
        Refused, Stored, StoredFile, discovered, given,
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
            .map(|l| (l.folder(), l.depth(), l.stored()))
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
        let kept = discovered(found.iter().map(ToString::to_string));
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
        );
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
            assert_eq!(given(paths), Err((1, refused)), "{segments:?}");
        }
        for refused in [Refused::OutsideTheProject, Refused::NotAnArtifactFile] {
            assert_eq!(refused.to_string(), refused.message());
            assert!(refused.message().starts_with("A path must"), "{refused:?}");
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
        assert_eq!(MAX_VALUES, 100_000);
        assert_eq!(MAX_DOCUMENT_BYTES, 8_388_608);
        assert_eq!(MAX_PROJECT_BYTES, 67_108_864);
    }
}
