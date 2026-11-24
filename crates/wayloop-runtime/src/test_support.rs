//! Temporary projects for tests: the runtime's own and those of the adapters, which enable the
//! feature `test-support` as a development dependency. Not part of a release.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// The number of the next temporary project of this process. Tests run in parallel, and two of
/// them may give the same name (a shared helper does): the number keeps their folders apart, so
/// that one never removes the folder another is reading.
static NEXT: AtomicU64 = AtomicU64::new(0);

/// A folder in the temporary folder of the platform with files, emptied first and removed when
/// dropped.
#[derive(Debug)]
pub struct TempProject(PathBuf);

impl TempProject {
    /// A project named `name`, in a folder of its own even when another project of this process
    /// has the same name, holding `files` (path relative to the root, content).
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
        let number = NEXT.fetch_add(1, Ordering::Relaxed);
        let process = std::process::id();
        let root = std::env::temp_dir().join(format!("wayloop-{process}-{number}-{name}"));
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

#[cfg(test)]
mod tests {
    use super::TempProject;

    #[test]
    fn two_projects_of_the_same_name_have_folders_of_their_own() {
        // A race between two tests that built "read-examples" at once (#260): one removed the
        // folder the other was reading.
        let first = TempProject::new("same", &[("a.txt", b"1")]);
        let second = TempProject::new("same", &[("a.txt", b"2")]);
        assert_ne!(first.root(), second.root());
        drop(second);
        assert_eq!(std::fs::read(first.root().join("a.txt")).unwrap(), b"1");
    }
}
