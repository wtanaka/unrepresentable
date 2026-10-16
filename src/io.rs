//! File system and stdio access. The only module that touches `std::fs` and
//! the standard streams.

use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use crate::failure::Failure;
use crate::types::{Answer, Holds as _, Message, Name, RelPath, Source};

/// The workspace root on disk.
#[derive(Clone, Debug)]
pub(crate) struct Root(PathBuf);

/// Kind of a directory entry, as far as this tool distinguishes them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EntryKind {
    /// A directory.
    Dir,
    /// A regular file.
    File,
    /// Anything else, such as a symbolic link.
    Other,
}

/// One entry of a directory listing.
#[derive(Clone, Debug)]
pub(crate) struct DirEntry {
    /// Whether the entry is a directory, a file or something else.
    pub kind: EntryKind,
    /// The entry's file name, without its parent path.
    pub name: Name,
}

/// A path with every symbolic link resolved: two paths with the same real path
/// are one file or directory.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RealPath(PathBuf);

impl Root {
    /// `.cargo/config*` files in directories above the workspace root, as
    /// `../`-prefixed paths relative to the root.
    pub(crate) fn ancestor_cargo_configs(&self) -> Vec<RelPath> {
        let mut found = Vec::new();
        let Ok(start) = self.0.canonicalize() else {
            return found;
        };
        let mut prefix = String::new();
        let mut current = start.parent().map(Path::to_path_buf);
        while let Some(dir) = current {
            prefix.push_str("../");
            for name in ["config", "config.toml"] {
                if dir.join(".cargo").join(name).is_file() {
                    found.push(RelPath::from_string(format!("{prefix}.cargo/{name}")));
                }
            }
            current = dir.parent().map(Path::to_path_buf);
        }
        found
    }

    /// Does the workspace-relative path `rel` resolve to somewhere outside the
    /// workspace root? A path that does not resolve at all counts as outside.
    pub(crate) fn escapes(&self, rel: &RelPath) -> Answer {
        let inside = match (
            fs::canonicalize(&self.0),
            fs::canonicalize(self.resolve(rel)),
        ) {
            (Ok(base), Ok(target)) => target.starts_with(base),
            (Err(_), _) | (_, Err(_)) => false,
        };
        (!inside).answer()
    }

    /// Does the workspace-relative path `rel` resolve to an existing file or
    /// directory (following symbolic links)?
    pub(crate) fn exists(&self, rel: &RelPath) -> Answer {
        (self.is_file(rel) == Answer::Yes || self.is_dir(rel) == Answer::Yes).answer()
    }

    /// Is the workspace-relative path `rel` an existing directory?
    pub(crate) fn is_dir(&self, rel: &RelPath) -> Answer {
        self.resolve(rel).is_dir().answer()
    }

    /// Is the workspace-relative path `rel` an existing regular file?
    pub(crate) fn is_file(&self, rel: &RelPath) -> Answer {
        self.resolve(rel).is_file().answer()
    }

    /// Lists the entries of the directory at `rel`, sorted by name.
    pub(crate) fn list(&self, rel: &RelPath) -> Result<Vec<DirEntry>, Failure> {
        list_dir(&self.resolve(rel))
    }

    /// Wraps an on-disk path as a workspace root.
    pub(crate) const fn new(path: PathBuf) -> Self {
        Self(path)
    }

    /// The root's on-disk path.
    pub(crate) fn path(&self) -> &Path {
        &self.0
    }

    /// Reads the UTF-8 file at `rel` under the root.
    pub(crate) fn read(&self, rel: &RelPath) -> Result<Source, Failure> {
        read_file(&self.resolve(rel))
    }

    /// The path `rel` with every symbolic link resolved, if it exists.
    #[must_use]
    pub(crate) fn real(&self, rel: &RelPath) -> Option<RealPath> {
        match fs::canonicalize(self.resolve(rel)) {
            Ok(found) => Some(RealPath(found)),
            Err(_) => None,
        }
    }

    /// Joins `rel` onto the root; an empty path or `.` is the root itself.
    pub(crate) fn resolve(&self, rel: &RelPath) -> PathBuf {
        if rel.as_str().is_empty() || rel.as_str() == "." {
            return self.0.clone();
        }
        self.0.join(rel.as_str())
    }

    /// Writes `contents` to `rel` under the root, creating parent directories as needed.
    pub(crate) fn write(&self, rel: &RelPath, contents: &Source) -> Result<(), Failure> {
        let target = self.resolve(rel);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|error| io_failure(parent, &error))?;
        }
        fs::write(&target, contents.as_str()).map_err(|error| io_failure(&target, &error))
    }
}

/// Reads the whole UTF-8 file at `path`.
pub(crate) fn read_file(path: &Path) -> Result<Source, Failure> {
    fs::read_to_string(path)
        .map(Source::from_string)
        .map_err(|error| io_failure(path, &error))
}

/// Lists the entries of the directory at `path`, sorted by name.
pub(crate) fn list_dir(path: &Path) -> Result<Vec<DirEntry>, Failure> {
    let mut entries = Vec::new();
    for result in fs::read_dir(path).map_err(|error| io_failure(path, &error))? {
        let item = result.map_err(|error| io_failure(path, &error))?;
        let file_type = item.file_type().map_err(|error| io_failure(path, &error))?;
        let name = match item.file_name().into_string() {
            Ok(text) => Name::from_string(text),
            Err(_) => {
                return Err(Failure::io(&Message::from_string(format!(
                    "non-UTF-8 file name under {}",
                    path.display()
                ))))
            }
        };
        let kind = if file_type.is_dir() {
            EntryKind::Dir
        } else if file_type.is_symlink() {
            EntryKind::Other
        } else {
            EntryKind::File
        };
        entries.push(DirEntry { kind, name });
    }
    entries.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(entries)
}

/// Builds an I/O failure that names `path` and carries the underlying error text.
fn io_failure(path: &Path, error: &io::Error) -> Failure {
    Failure::io(&Message::from_string(format!(
        "{}: {error}",
        path.display()
    )))
}

/// Writes `text` to standard output.
pub(crate) fn write_stdout(text: &Message) -> Result<(), Failure> {
    let mut stream = io::stdout().lock();
    stream
        .write_all(text.as_str().as_bytes())
        .map_err(|error| Failure::io(&Message::from_string(error.to_string())))
}

/// Writes `text` to standard error.
pub(crate) fn write_stderr(text: &Message) -> Result<(), Failure> {
    let mut stream = io::stderr().lock();
    stream
        .write_all(text.as_str().as_bytes())
        .map_err(|error| Failure::io(&Message::from_string(error.to_string())))
}
