//! The stored artifacts of a project, read into the canonical input of `validate` (SPEC-0001,
//! Where artifacts live; plan of SPEC-0001, Boundary and D1).
//!
//! The core decides which files are artifacts and in which order
//! ([`wayloop_core::artifacts::locations`]); this module only lists the folders the core
//! names, reads the files and changes their syntax:
//!
//! - [`discover`] lists the files below each location of the core, down to its depth, and
//!   passes their paths, relative to the root with `/` separators, to the core. Only regular
//!   files and folders are listed: a symbolic link is not followed, so that discovery never
//!   reads outside the project. A missing location is a project without such artifacts.
//! - [`load`] reads each file, at most [`MAX_FILE_BYTES`] of it, into one entry of
//!   `documents`, in the order given: a YAML file as a `document` with the line of each value,
//!   a Markdown file as `markdown` text, and a file that is larger than the bound, is not
//!   UTF-8 or does not parse as an `unreadable` entry with the line of the fault and a
//!   message; the core reports it as a `syntax` error of the file (R12). A file that cannot be
//!   read at all stops the load with [`Unreadable`], an error of the environment.
//!
//! The entries carry the path of their file as their origin, so every error of the core gives
//! the file and the line of the faulty value in `source` (R12).

use std::fmt;
use std::fs;
use std::io::{self, Read as _};
use std::path::{Component, Path, PathBuf};

use wayloop_core::artifacts::locations::{self, LOCATIONS, MAX_FILE_BYTES, Stored, StoredFile};
use wayloop_schemas::validate::{
    DocumentEntry, EntryFormat, FreeDocument, Origin, ParseError, ValidateInput,
};

use crate::yaml;

/// A file or a folder of the project that could not be read, with the error of the platform.
#[derive(Debug)]
pub struct Unreadable {
    path: PathBuf,
    error: io::Error,
}

impl Unreadable {
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

impl fmt::Display for Unreadable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path.display(), self.error)
    }
}

impl std::error::Error for Unreadable {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

/// The stored artifacts of the project at `root`, sorted by the core.
///
/// # Errors
///
/// [`Unreadable`] when a folder of a location exists but cannot be listed, or holds a name that
/// is not UTF-8.
pub fn discover(root: &Path) -> Result<Vec<StoredFile>, Unreadable> {
    let mut found = Vec::new();
    for location in &LOCATIONS {
        list(root, location.folder(), location.depth(), &mut found)?;
    }
    Ok(locations::discovered(found))
}

/// Adds to `found` the paths of the files below `folder`, `depth` levels down at most.
fn list(
    root: &Path,
    folder: &str,
    depth: usize,
    found: &mut Vec<String>,
) -> Result<(), Unreadable> {
    let path = root.join(folder);
    let entries = match fs::read_dir(&path) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(Unreadable::new(&path, error)),
    };
    for entry in entries {
        let entry = entry.map_err(|e| Unreadable::new(&path, e))?;
        let kind = entry
            .file_type()
            .map_err(|e| Unreadable::new(&entry.path(), e))?;
        let name = entry.file_name().into_string().map_err(|_| {
            let error = io::Error::new(io::ErrorKind::InvalidData, "the name is not UTF-8");
            Unreadable::new(&entry.path(), error)
        })?;
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
/// in the order given.
///
/// # Errors
///
/// [`Unreadable`] for the first file that cannot be read.
pub fn load(root: &Path, files: Vec<StoredFile>) -> Result<ValidateInput, Unreadable> {
    let mut documents = Vec::with_capacity(files.len());
    for file in files {
        let path = root.join(file.path.as_str());
        let content = read(&path).map_err(|e| Unreadable::new(&path, e))?;
        documents.push(entry(file, content));
    }
    Ok(ValidateInput {
        documents,
        base: None,
        policy: None,
        adrs: None,
        language: None,
    })
}

/// What was read of a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Content {
    /// All of it.
    Bytes(Vec<u8>),
    /// More than [`MAX_FILE_BYTES`]: not kept.
    TooLarge,
}

/// The content of the file at `path`, reading one byte beyond the bound at most.
fn read(path: &Path) -> io::Result<Content> {
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)?;
    let size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    Ok(if size > MAX_FILE_BYTES {
        Content::TooLarge
    } else {
        Content::Bytes(bytes)
    })
}

/// The entry of `file` with its `content`.
pub(crate) fn entry(file: StoredFile, content: Content) -> DocumentEntry {
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
        Err(error) => return unreadable(empty, error),
    };
    match file.stored {
        Stored::Markdown => DocumentEntry {
            format: EntryFormat::Markdown,
            text: Some(text),
            ..empty
        },
        Stored::Yaml => match yaml::parse(&text) {
            Ok(parsed) => DocumentEntry {
                format: EntryFormat::Document,
                document: Some(FreeDocument(parsed.document)),
                lines: Some(parsed.lines),
                ..empty
            },
            Err(error) => unreadable(empty, error),
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

/// The text of `content`; a refusal when it is larger than the bound, at line 1, or is not
/// UTF-8, at the line of the first byte that is not.
fn text(content: Content) -> Result<String, ParseError> {
    let bytes = match content {
        Content::Bytes(bytes) => bytes,
        Content::TooLarge => {
            let message =
                format!("the file is larger than {MAX_FILE_BYTES} bytes and was not read");
            return Err(yaml::fault(1, message));
        }
    };
    String::from_utf8(bytes).map_err(|e| {
        let valid = e.utf8_error().valid_up_to();
        let before = e.as_bytes().get(..valid).unwrap_or_default();
        // The line of the first byte that is not UTF-8: one more than the line breaks before it.
        let at = before.split(|b| *b == b'\n').count();
        yaml::fault(at, "the file is not UTF-8 text")
    })
}

#[cfg(test)]
mod tests;
