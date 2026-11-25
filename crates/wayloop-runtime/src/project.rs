//! The stored artifacts of a project, read into the canonical input of `validate` (SPEC-0001,
//! Where artifacts live; plan of SPEC-0001, Boundary and D1).
//!
//! The core decides which files are artifacts, in which order, and the bounds of what is read
//! ([`wayloop_core::artifacts::locations`]); this module only lists the folders the core names,
//! reads the files and changes their syntax. Its guarantees:
//!
//! - [`discover`] lists the files below each location of the core, down to its depth, and
//!   passes their paths, relative to the root with `/` separators, to the core, which refuses
//!   more than [`MAX_FILES`](locations::MAX_FILES) of them. A location whose folder, or a
//!   parent of it below the root, is a symbolic link is treated as absent, as is a missing
//!   one; below a location, a link, a special file and a name that is not UTF-8 are skipped.
//!   Discovery therefore lists no folder and no file outside the project.
//! - [`load`] resolves the root and every file through their links and refuses, with
//!   [`LoadError::Refused`], a file whose resolved path is not inside the resolved root or does
//!   not have the extension of its path, so that a link cannot turn another file into an
//!   artifact. It refuses a path that is not a regular file (a folder, a pipe, a device) as an
//!   error of the environment before opening it, so that a read never blocks. It reads at most
//!   [`MAX_FILE_BYTES`] of a file and counts at most [`MAX_PROJECT_BYTES`] for the whole set:
//!   the bytes read and the counted bytes of every document built, an estimate of their
//!   memory (see [`locations::VALUE_COST_BYTES`]). It builds one entry of `documents` per
//!   file, in the order given: a YAML file as a `document` with the line of each value, a
//!   Markdown file as `markdown` text. A file that is larger than its bound, would exceed the
//!   bound of the set or of a document, is not UTF-8 or does not parse becomes an `unreadable`
//!   entry with the line of the fault and a message, which the core reports as a `syntax`
//!   error of the file (R12); it is never loaded and never aborts the run.
//!
//! - [`adrs`] lists the files of the folder of the ADRs that the core names, as a location is
//!   listed, and passes their paths to the core, which keeps the numbers of the ADRs; with no
//!   such folder, `adrs` stays absent and the core does not check the citations of ADRs.
//!
//! The entries carry the path of their file as their origin, so every error of the core gives
//! the file and the line of the faulty value in `source` (R12).
//!
//! A file is resolved, checked and then opened: a link changed between those steps by another
//! process is not detected; the project is assumed not to change while it is read.

use std::fmt;
use std::fs;
use std::io::{self, Read as _};
use std::path::{Component, Path, PathBuf};

use wayloop_core::artifacts::locations::{
    self, ADR_DEPTH, ADR_FOLDER, LOCATIONS, MAX_DOCUMENT_BYTES, MAX_FILE_BYTES, MAX_PROJECT_BYTES,
    Refused, Stored, StoredFile,
};
use wayloop_schemas::validate::{
    ArtifactFolder, DocumentEntry, EntryFormat, FreeDocument, Origin, ParseError, ValidateInput,
};
use wayloop_schemas::value::{AdrNumber, ProjectPath};

use crate::refusal::fault;
use crate::yaml;

/// A file or a folder of the project that the environment did not let the runtime read, with
/// the error of the platform: an error of the environment (exit code 3).
#[derive(Debug)]
pub struct ReadError {
    path: PathBuf,
    error: io::Error,
}

impl ReadError {
    fn new(path: &Path, error: io::Error) -> Self {
        Self {
            path: path.to_path_buf(),
            error,
        }
    }

    /// The path that could not be read, as the platform spells it.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The error of the platform.
    #[must_use]
    pub fn error(&self) -> &io::Error {
        &self.error
    }
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path.display(), self.error)
    }
}

impl std::error::Error for ReadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

/// Why the files of a set were not loaded.
#[derive(Debug)]
pub enum LoadError {
    /// A file, a folder or the root could not be read: an error of the environment.
    Read(ReadError),
    /// The core refused the set, or the file at `path` once its links are resolved: outside
    /// the root, or not an artifact file by its extension.
    Refused {
        /// The file refused; `None` when the whole set is (too many files).
        path: Option<ProjectPath>,
        /// Why.
        refused: Refused,
    },
}

impl From<ReadError> for LoadError {
    fn from(error: ReadError) -> Self {
        LoadError::Read(error)
    }
}

/// What discovery found in a project: its stored artifacts and the folders that hold them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Discovery {
    /// The stored artifacts, sorted by the core.
    pub files: Vec<StoredFile>,
    /// The folders of the locations that exist, in the order of the locations; the core warns
    /// when there is none (SPEC-0001 R26).
    pub locations: Vec<ArtifactFolder>,
}

/// The stored artifacts of the project at `root`, sorted by the core, and the folders of the
/// locations that exist.
///
/// # Errors
///
/// [`LoadError::Read`] when a folder of a location exists but cannot be listed;
/// [`LoadError::Refused`] when the project holds more than
/// [`MAX_FILES`](locations::MAX_FILES) artifacts.
pub fn discover(root: &Path) -> Result<Discovery, LoadError> {
    let mut found = Vec::new();
    let mut folders = Vec::new();
    for location in &LOCATIONS {
        let folder = location.folder();
        // A folder reached through a link is not a location of the project.
        if is_real_folder(root, folder.path())? {
            folders.push(folder);
            list(root, folder.path(), location.depth(), &mut found)?;
        }
    }
    let files = locations::discovered(found).map_err(|refused| LoadError::Refused {
        path: None,
        refused,
    })?;
    Ok(Discovery {
        files,
        locations: folders,
    })
}

/// The canonical input of `validate` for the project at `root`: its stored artifacts, loaded,
/// and the folders of the locations that exist, which the core checks (SPEC-0001 R26).
///
/// # Errors
///
/// As [`discover`] and [`load`].
pub fn discovered_input(root: &Path) -> Result<ValidateInput, LoadError> {
    let found = discover(root)?;
    Ok(ValidateInput {
        locations: Some(found.locations),
        ..named_input(root, found.files)?
    })
}

/// The canonical input of `validate` for `files` of the project at `root`, named by a person:
/// the files, loaded as by [`load`], and the numbers of the ADRs of the project, as by
/// [`adrs`], so that the citations of ADRs are checked however the files were chosen.
///
/// # Errors
///
/// As [`load`], and [`LoadError::Read`] when the folder of the ADRs exists but cannot be
/// listed.
pub fn named_input(root: &Path, files: Vec<StoredFile>) -> Result<ValidateInput, LoadError> {
    let adrs = adrs(root)?;
    Ok(ValidateInput {
        adrs,
        ..load(root, files)?
    })
}

/// The numbers of the ADRs of the project at `root`, which the core reads from the names of
/// the files in [`ADR_FOLDER`] ([`locations::adr_numbers`]; SPEC-0001 Q5, #344); `None` when
/// that folder does not exist or is reached through a link, as a location is. Below it, a link,
/// a special file and a name that is not UTF-8 are skipped.
///
/// # Errors
///
/// [`ReadError`] when the folder, or a parent of it, exists but cannot be read.
pub fn adrs(root: &Path) -> Result<Option<Vec<AdrNumber>>, ReadError> {
    if !is_real_folder(root, ADR_FOLDER)? {
        return Ok(None);
    }
    let mut found = Vec::new();
    list(root, ADR_FOLDER, ADR_DEPTH, &mut found)?;
    Ok(Some(locations::adr_numbers(found)))
}

/// Whether `folder`, a path below `root` with `/` separators, is a folder reached without a
/// link: every segment exists and none is a symbolic link.
fn is_real_folder(root: &Path, folder: &str) -> Result<bool, ReadError> {
    let mut path = root.to_path_buf();
    for segment in folder.split('/') {
        path.push(segment);
        match fs::symlink_metadata(&path) {
            Ok(meta) if meta.file_type().is_symlink() => return Ok(false),
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(ReadError::new(&path, error)),
        }
    }
    Ok(true)
}

/// Adds to `found` the paths of the regular files below `folder`, a folder reached without a
/// link, `depth` levels down at most. A folder inside it is entered when the listing says it
/// is a folder, which a symbolic link is not.
pub(crate) fn list(
    root: &Path,
    folder: &str,
    depth: usize,
    found: &mut Vec<String>,
) -> Result<(), ReadError> {
    let path = root.join(folder);
    let entries = fs::read_dir(&path).map_err(|e| ReadError::new(&path, e))?;
    for entry in entries {
        let entry = entry.map_err(|e| ReadError::new(&path, e))?;
        let kind = entry
            .file_type()
            .map_err(|e| ReadError::new(&entry.path(), e))?;
        // A name that is not UTF-8 cannot be a path of the project: skipped, as a link is.
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        let below = format!("{folder}/{name}");
        if kind.is_dir() && depth > 1 {
            list(root, &below, depth - 1, found)?;
        } else if kind.is_file() {
            found.push(below);
        }
    }
    Ok(())
}

/// The segments of `path`, a path a person named, as the platform splits it: `.` for the
/// current folder, `..` for the parent, an empty segment for the root and the prefix of a
/// drive as written, for [`locations::given`], which refuses what leaves the project.
#[must_use]
pub fn segments(path: &Path) -> Vec<String> {
    path.components()
        .map(|component| match component {
            Component::Prefix(prefix) => prefix.as_os_str().to_string_lossy().into_owned(),
            Component::RootDir => String::new(),
            Component::CurDir => ".".to_owned(),
            Component::ParentDir => "..".to_owned(),
            Component::Normal(name) => name.to_string_lossy().into_owned(),
        })
        .collect()
}

/// The canonical input of `validate` for `files` of the project at `root`, one entry per file
/// in the order given, within [`MAX_PROJECT_BYTES`].
///
/// # Errors
///
/// [`LoadError::Refused`] for the first file that, once its links are resolved, is outside
/// the root or has another extension than its path; [`LoadError::Read`] for the root or the
/// first file that cannot be read or is not a regular file.
pub fn load(root: &Path, files: Vec<StoredFile>) -> Result<ValidateInput, LoadError> {
    load_within(root, files, MAX_PROJECT_BYTES)
}

/// [`load`] with `budget` counted bytes for the whole set.
pub(crate) fn load_within(
    root: &Path,
    files: Vec<StoredFile>,
    budget: usize,
) -> Result<ValidateInput, LoadError> {
    let real_root = fs::canonicalize(root).map_err(|e| ReadError::new(root, e))?;
    let mut used: usize = 0;
    let mut documents = Vec::with_capacity(files.len());
    for file in files {
        let path = root.join(file.path.as_str());
        let resolved = fs::canonicalize(&path).map_err(|e| ReadError::new(&path, e))?;
        let refused = if !resolved.starts_with(&real_root) {
            Some(Refused::OutsideTheProject)
        } else if locations::stored_by_extension(&resolved.to_string_lossy()) != Some(file.stored) {
            Some(Refused::NotAnArtifactFile)
        } else {
            None
        };
        if let Some(refused) = refused {
            return Err(LoadError::Refused {
                path: Some(file.path),
                refused,
            });
        }
        let content = read(&resolved, budget.saturating_sub(used), budget)
            .map_err(|e| ReadError::new(&path, e))?;
        used = used.saturating_add(content.bytes());
        let (entry, built) = entry(file, content, budget.saturating_sub(used));
        used = used.saturating_add(built);
        documents.push(entry);
    }
    Ok(ValidateInput {
        documents,
        base: None,
        policy: None,
        adrs: None,
        language: None,
        locations: None,
    })
}

/// What was read of a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Content {
    /// All of it.
    Bytes(Vec<u8>),
    /// More than [`MAX_FILE_BYTES`]: not kept.
    TooLarge,
    /// More than the bytes left of a set of this many counted bytes: not kept.
    BeyondTheSet(usize),
}

impl Content {
    /// The bytes kept.
    fn bytes(&self) -> usize {
        match self {
            Content::Bytes(bytes) => bytes.len(),
            Content::TooLarge | Content::BeyondTheSet(_) => 0,
        }
    }
}

/// The content of the regular file at `path`, reading at most one byte beyond the smaller of
/// [`MAX_FILE_BYTES`] and `room`, the bytes left of a set of `budget`; a path that is not a
/// regular file is refused before it is opened.
fn read(path: &Path, room: usize, budget: usize) -> io::Result<Content> {
    if !fs::metadata(path)?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "not a regular file",
        ));
    }
    let limit = MAX_FILE_BYTES.min(room);
    let mut file = fs::File::open(path)?;
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 8192];
    while bytes.len() <= limit {
        let n = file.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        bytes.extend_from_slice(chunk.get(..n).unwrap_or_default());
    }
    Ok(if bytes.len() > MAX_FILE_BYTES {
        Content::TooLarge
    } else if bytes.len() > room {
        Content::BeyondTheSet(budget)
    } else {
        Content::Bytes(bytes)
    })
}

/// The entry of `file` with its `content`, whose document may take `room` counted bytes (and
/// at most [`MAX_DOCUMENT_BYTES`]); and the counted bytes the document took.
pub(crate) fn entry(file: StoredFile, content: Content, room: usize) -> (DocumentEntry, usize) {
    let origin = Some(Origin {
        file: Some(file.path),
        issue: None,
        milestone: None,
    });
    // Unreadable until it is read; every other format replaces it.
    let empty = DocumentEntry {
        origin,
        format: EntryFormat::Unreadable,
        document: None,
        lines: None,
        text: None,
        error: None,
        input_kind: None,
    };
    let text = match text(content) {
        Ok(text) => text,
        Err(error) => return (unreadable(empty, error), 0),
    };
    match file.stored {
        Stored::Markdown => {
            let entry = DocumentEntry {
                format: EntryFormat::Markdown,
                text: Some(text),
                ..empty
            };
            (entry, 0)
        }
        Stored::Yaml => match yaml::parse(&text, room.min(MAX_DOCUMENT_BYTES)) {
            Ok(parsed) => {
                let entry = DocumentEntry {
                    format: EntryFormat::Document,
                    document: Some(FreeDocument(parsed.document)),
                    lines: Some(parsed.lines),
                    ..empty
                };
                (entry, parsed.bytes)
            }
            Err(error) => (unreadable(empty, error), 0),
        },
    }
}

/// `entry`, an unreadable file, with what was wrong.
fn unreadable(entry: DocumentEntry, error: ParseError) -> DocumentEntry {
    DocumentEntry {
        error: Some(error),
        ..entry
    }
}

/// The text of `content`; a refusal at line 1 when it is beyond the bound of a file or of its
/// set, or when it is not UTF-8, at the line of the first byte that is not.
fn text(content: Content) -> Result<String, ParseError> {
    let bytes = match content {
        Content::Bytes(bytes) => bytes,
        Content::TooLarge => {
            let message =
                format!("the file is larger than {MAX_FILE_BYTES} bytes and was not loaded");
            return Err(fault(1, message));
        }
        Content::BeyondTheSet(budget) => {
            let message = format!(
                "the files of the set take more than {budget} counted bytes, an estimate of \
                 their memory; this one was not loaded"
            );
            return Err(fault(1, message));
        }
    };
    String::from_utf8(bytes).map_err(|e| {
        let valid = e.utf8_error().valid_up_to();
        let before = e.as_bytes().get(..valid).unwrap_or_default();
        // The line of the first byte that is not UTF-8: one more than the line breaks before it.
        let at = before.split(|b| *b == b'\n').count();
        fault(at, "the file is not UTF-8 text")
    })
}

#[cfg(test)]
mod tests;
