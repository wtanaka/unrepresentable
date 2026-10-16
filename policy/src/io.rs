//! File system, environment and standard streams. The only module that
//! touches `std::fs`, `std::env` variables and the standard streams.

use crate::bytes::Bytes;
use crate::failure::Failure;
use crate::types::{Answer, Count, Holds as _, Message, Name, Text};
use alloc::collections::BTreeMap;
use std::env;
use std::fs::{self, File};
use std::io::{self, Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// What a directory entry is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EntryKind {
    /// A directory.
    Dir,
    /// A regular file.
    File,
    /// Anything else, such as a link.
    Other,
}

/// An entry of a directory listing.
#[derive(Clone, Debug)]
pub(crate) struct DirEntry {
    /// What the entry is.
    pub kind: EntryKind,
    /// The name of the entry.
    pub name: Name,
}

/// The process environment as a sorted map.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Vars(BTreeMap<Name, Text>);

impl Vars {
    /// An environment with no variables.
    pub(crate) const fn empty() -> Self {
        Self(BTreeMap::new())
    }

    /// The value of `key`, when it is set.
    #[must_use]
    pub(crate) fn get(&self, key: &Name) -> Option<&Text> {
        self.0.get(key)
    }

    /// The value, or a refusal naming the missing variable.
    pub(crate) fn require(&self, key: &Name) -> Result<Text, Failure> {
        match self.0.get(key) {
            Some(found) => Ok(found.clone()),
            None => Err(Failure::refused(&Message::from_string(format!(
                "environment variable {} is not set",
                key.as_str()
            )))),
        }
    }

    /// The value, or the empty text when the variable is unset.
    pub(crate) fn text(&self, key: &Name) -> Text {
        match self.0.get(key) {
            Some(found) => found.clone(),
            None => Text::of(""),
        }
    }

    /// The environment with `key` set to `value`.
    pub(crate) fn with(self, key: &Name, value: &Text) -> Self {
        let mut map = self.0;
        match map.insert(key.clone(), value.clone()) {
            Some(_) | None => {}
        }
        Self(map)
    }
}

/// A file and the place to copy it to.
pub(crate) struct CopyPlan {
    /// The file to copy.
    pub from: PathBuf,
    /// Where to copy it.
    pub to: PathBuf,
}

/// Reads the whole process environment.
pub(crate) fn read_vars() -> Vars {
    let mut vars = Vars::empty();
    for (key, value) in env::vars_os() {
        vars = vars.with(
            &Name::from_string(key.to_string_lossy().into_owned()),
            &Text::from_string(value.to_string_lossy().into_owned()),
        );
    }
    vars
}

/// Reads the whole file at `path`.
pub(crate) fn read_bytes(path: &Path) -> Result<Bytes, Failure> {
    fs::read(path)
        .map(Bytes::from_vec)
        .map_err(|error| io_failure(path, &error))
}

/// At most `limit` bytes of the file.
pub(crate) fn read_bytes_limited(path: &Path, limit: Count) -> Result<Bytes, Failure> {
    let file = File::open(path).map_err(|error| io_failure(path, &error))?;
    let mut buffer = Vec::new();
    let cap = u64::try_from(limit.get())
        .map_err(|error| Failure::internal(&Message::from_string(error.to_string())))?;
    file.take(cap)
        .read_to_end(&mut buffer)
        .map(|_read| ())
        .map_err(|error| io_failure(path, &error))?;
    Ok(Bytes::from_vec(buffer))
}

/// The file as text, with invalid UTF-8 replaced.
pub(crate) fn read_text(path: &Path) -> Result<Text, Failure> {
    Ok(read_bytes(path)?.lossy_text())
}

/// Writes `contents` to `path`.
pub(crate) fn write_bytes(path: &Path, contents: &Bytes) -> Result<(), Failure> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| io_failure(parent, &error))?;
    }
    fs::write(path, contents.as_slice()).map_err(|error| io_failure(path, &error))
}

/// Writes `contents` to `path`.
pub(crate) fn write_text(path: &Path, contents: &Text) -> Result<(), Failure> {
    write_bytes(path, &Bytes::of_text(contents))
}

/// Creates `path` and its parents.
pub(crate) fn create_dirs(path: &Path) -> Result<(), Failure> {
    fs::create_dir_all(path).map_err(|error| io_failure(path, &error))
}

/// Removes the directory `path` and everything in it.
pub(crate) fn remove_tree(path: &Path) -> Result<(), Failure> {
    fs::remove_dir_all(path).map_err(|error| io_failure(path, &error))
}

/// Removes the file `path`.
pub(crate) fn remove_file(path: &Path) -> Result<(), Failure> {
    fs::remove_file(path).map_err(|error| io_failure(path, &error))
}

/// Copies a file as `plan` says.
pub(crate) fn copy_file(plan: &CopyPlan) -> Result<(), Failure> {
    fs::copy(&plan.from, &plan.to)
        .map(|_copied| ())
        .map_err(|error| io_failure(&plan.from, &error))
}

/// Whether `path` is itself a symbolic link (not what it points at).
pub(crate) fn is_link(path: &Path) -> Answer {
    match fs::symlink_metadata(path) {
        Ok(found) => found.file_type().is_symlink().answer(),
        Err(_) => Answer::No,
    }
}

/// Whether `path` is a regular file.
pub(crate) fn is_file(path: &Path) -> Answer {
    path.is_file().answer()
}

/// Whether `path` is a directory.
pub(crate) fn is_dir(path: &Path) -> Answer {
    path.is_dir().answer()
}

/// Whether anything, a dangling link included, exists at `path`.
pub(crate) fn exists_no_follow(path: &Path) -> Answer {
    fs::symlink_metadata(path).is_ok().answer()
}

/// Lists the entries of the directory `path`, sorted by name.
pub(crate) fn list_dir(path: &Path) -> Result<Vec<DirEntry>, Failure> {
    let mut entries = Vec::new();
    for result in fs::read_dir(path).map_err(|error| io_failure(path, &error))? {
        let item = result.map_err(|error| io_failure(path, &error))?;
        let file_type = item.file_type().map_err(|error| io_failure(path, &error))?;
        let name = Name::from_string(item.file_name().to_string_lossy().into_owned());
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

/// Copies the tree under `from` to `to`, skipping `.git` entries and anything
/// that is not a plain file or directory (symbolic links are not followed).
pub(crate) fn copy_tree(plan: &CopyPlan) -> Result<(), Failure> {
    create_dirs(&plan.to)?;
    for entry in list_dir(&plan.from)? {
        if entry.name.as_str() == ".git" {
            continue;
        }
        let step = CopyPlan {
            from: plan.from.join(entry.name.as_str()),
            to: plan.to.join(entry.name.as_str()),
        };
        match entry.kind {
            EntryKind::Dir => copy_tree(&step)?,
            EntryKind::File => copy_file(&step)?,
            EntryKind::Other => {}
        }
    }
    Ok(())
}

/// The absolute form of `path`.
pub(crate) fn absolute(path: &Path) -> Result<PathBuf, Failure> {
    fs::canonicalize(path).map_err(|error| io_failure(path, &error))
}

/// The failure for an input or output error on `path`.
fn io_failure(path: &Path, error: &io::Error) -> Failure {
    Failure::io(&Message::from_string(format!(
        "{}: {error}",
        path.display()
    )))
}

/// Seconds since the Unix epoch, now.
pub(crate) fn now_epoch_seconds() -> Result<Count, Failure> {
    let since = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|error| Failure::internal(&Message::from_string(error.to_string())))?;
    Ok(Count::new(usize::try_from(since.as_secs()).map_err(
        |error| Failure::internal(&Message::from_string(error.to_string())),
    )?))
}

/// Writes `text` to standard output.
pub(crate) fn write_stdout(text: &Text) -> Result<(), Failure> {
    let mut stream = io::stdout().lock();
    stream
        .write_all(text.as_str().as_bytes())
        .map_err(|error| Failure::io(&Message::from_string(error.to_string())))
}

/// Writes `text` to standard error.
pub(crate) fn write_stderr(text: &Text) -> Result<(), Failure> {
    let mut stream = io::stderr().lock();
    stream
        .write_all(text.as_str().as_bytes())
        .map_err(|error| Failure::io(&Message::from_string(error.to_string())))
}
