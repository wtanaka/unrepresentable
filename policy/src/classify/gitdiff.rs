//! Reading `git diff` plumbing output and running it.

use crate::bytes::Bytes;
use crate::classify::{Contents, Reader};
use crate::failure::Failure;
use crate::process::{self, Args};
use crate::reason::Commits;
use crate::types::{Answer, Message, RelPath, Sha, Text};
use alloc::collections::BTreeSet;
use core::iter;
use std::path::{Path, PathBuf};

/// Git's one-letter change status.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Status {
    /// The path was added.
    Added,
    /// The path was deleted.
    Deleted,
    /// The contents of the path changed.
    Modified,
    /// The file type of the path changed.
    TypeChanged,
    /// Git could not determine the change.
    Unknown,
    /// The path has unresolved merge conflicts.
    Unmerged,
}

impl Status {
    /// Whether the file existed on both sides.
    pub(crate) const fn is_modification(self) -> Answer {
        match self {
            Self::Modified | Self::TypeChanged => Answer::Yes,
            Self::Added | Self::Deleted | Self::Unmerged | Self::Unknown => Answer::No,
        }
    }

    /// Reads a status letter from `git diff --raw`.
    #[must_use]
    fn parse(letter: &Text) -> Option<Self> {
        match letter.as_str() {
            "A" => Some(Self::Added),
            "M" => Some(Self::Modified),
            "D" => Some(Self::Deleted),
            "T" => Some(Self::TypeChanged),
            "U" => Some(Self::Unmerged),
            "X" => Some(Self::Unknown),
            _ => None,
        }
    }
}

/// One changed path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Entry {
    /// The path that changed.
    pub path: RelPath,
    /// How the path changed.
    pub status: Status,
}

/// How git's plumbing output was malformed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Malformed {
    /// The raw output is not NUL terminated.
    NotNulTerminated,
    /// The raw output has an odd number of tokens.
    OddTokens,
    /// A raw record is malformed.
    RawRecord,
    /// A status letter is not one that git defines.
    UnknownStatus,
}

impl Malformed {
    /// The failure this problem is reported as.
    fn failure(self) -> Failure {
        Failure::parse(&Message::of(match self {
            Self::NotNulTerminated => "diff output not NUL terminated",
            Self::OddTokens => "odd number of tokens",
            Self::UnknownStatus => "unknown status",
            Self::RawRecord => "bad raw record",
        }))
    }
}

/// What a diff between two commits touched.
pub(crate) struct Changes {
    /// One entry for each changed path.
    pub entries: Vec<Entry>,
    /// Paths that are symbolic links or submodules in either commit.
    pub symlinks_and_submodules: BTreeSet<RelPath>,
}

/// A commit of a local git repository, read through `git show`.
pub(crate) struct GitTree {
    /// The directory of the repository.
    pub dir: PathBuf,
    /// The commit to read.
    pub rev: Sha,
}

impl Reader for GitTree {
    fn read(&self, path: &RelPath) -> Contents {
        let spec = format!("{}:{}", self.rev.as_str(), path.as_str());
        match process::git(&self.dir, &Args::of(&["show", &spec])) {
            Ok(bytes) => Contents::Present(bytes.lossy_text()),
            Err(_) => Contents::Absent,
        }
    }
}

/// Parses `git diff --name-status --no-renames -z`.
pub(crate) fn parse_name_status(data: &Bytes) -> Result<Vec<Entry>, Failure> {
    if data.is_empty() == Answer::Yes {
        return Ok(Vec::new());
    }
    let Some(body) = data.strip_nul_suffix() else {
        return Err(Malformed::NotNulTerminated.failure());
    };
    let tokens = body.split_nul();
    let (pairs, remainder) = tokens.as_chunks::<2>();
    if !remainder.is_empty() {
        return Err(Malformed::OddTokens.failure());
    }
    let mut out = Vec::new();
    for [status, path] in pairs {
        let Some(known) = Status::parse(&status.lossy_text()) else {
            return Err(Malformed::UnknownStatus.failure());
        };
        out.push(Entry {
            path: RelPath::from_string(path.lossy_text().as_str().to_owned()),
            status: known,
        });
    }
    Ok(out)
}

/// Paths whose old or new mode is a symlink or a submodule, from `--raw -z`.
pub(crate) fn parse_raw_modes(data: &Bytes) -> Result<BTreeSet<RelPath>, Failure> {
    let mut bad = BTreeSet::new();
    let tokens = data.split_nul();
    for pair in tokens.chunks(2) {
        let Some(meta) = pair.first() else {
            break;
        };
        if meta.is_empty() == Answer::Yes {
            break;
        }
        let Some(path) = pair.get(1) else {
            return Err(Malformed::RawRecord.failure());
        };
        let text = meta.lossy_text();
        let parts: Vec<&str> = text
            .as_str()
            .trim_start_matches(':')
            .split_whitespace()
            .collect();
        let (Some(old_mode), Some(new_mode)) = (parts.first(), parts.get(1)) else {
            return Err(Malformed::RawRecord.failure());
        };
        if parts.len() < 5 {
            return Err(Malformed::RawRecord.failure());
        }
        let special = ["120000", "160000"];
        if special.contains(old_mode) || special.contains(new_mode) {
            bad.extend(iter::once(RelPath::from_string(
                path.lossy_text().as_str().to_owned(),
            )));
        }
    }
    Ok(bad)
}

/// Runs the two `git diff` commands in `dir` for `commits`.
pub(crate) fn changes(dir: &Path, commits: &Commits) -> Result<Changes, Failure> {
    let range = format!("{}...{}", commits.base.as_str(), commits.head.as_str());
    let names = process::git(
        dir,
        &Args::of(&["diff", "--name-status", "--no-renames", "-z", &range]),
    )?;
    let raw = process::git(
        dir,
        &Args::of(&["diff", "--raw", "--no-renames", "-z", &range]),
    )?;
    Ok(Changes {
        entries: parse_name_status(&names)?,
        symlinks_and_submodules: parse_raw_modes(&raw)?,
    })
}
