//! Temporary projects for tests: the runtime's own and those of the adapters, which enable the
//! feature `test-support` as a development dependency. Not part of a release.

use std::fs;
use std::path::{Path, PathBuf};

/// A folder in the temporary folder of the platform with files, emptied first and removed when
/// dropped.
#[derive(Debug)]
pub struct TempProject(PathBuf);

impl TempProject {
    /// A project named `name`, unique to this process, holding `files` (path relative to the
    /// root, content).
    ///
    /// # Panics
    ///
    /// When the folder or a file cannot be written: a test stops there.
    #[must_use]
    #[expect(
        clippy::unwrap_used,
        reason = "a helper of tests stops at the line that failed"
    )]
    pub fn new(name: &str, files: &[(&str, &[u8])]) -> Self {
        let root = std::env::temp_dir().join(format!("wayloop-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        for (path, content) in files {
            let path = root.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, content).unwrap();
        }
        Self(root)
    }

    /// The root of the project.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
