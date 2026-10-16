//! Git access: the change range (union of range diff, working-tree diff and
//! untracked files), and file contents at the base commit.

use alloc::collections::BTreeMap;

use crate::failure::Failure;
use crate::io::Root;
use crate::process::{self, Args, Bytes, Success};
use crate::types::{Answer, Holds as _, Message, RelPath, Source, Text};

/// How a path changed relative to the base.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Status {
    /// The path did not exist before and does now.
    Added,
    /// The path existed before and no longer does.
    Deleted,
    /// The path exists on both sides with different contents.
    Modified,
}

impl Status {
    /// The status of a path seen first as `self`, then changed again as `later`.
    pub(crate) const fn then(self, later: Self) -> Self {
        match (self, later) {
            (Self::Added | Self::Modified, Self::Deleted)
            | (Self::Deleted, Self::Deleted | Self::Modified) => Self::Deleted,
            (Self::Added, Self::Added | Self::Modified) => Self::Added,
            (Self::Deleted | Self::Modified, Self::Added) | (Self::Modified, Self::Modified) => {
                Self::Modified
            }
        }
    }
}

/// A changed path with the way it changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Change {
    /// The path that changed.
    pub path: RelPath,
    /// How the path changed.
    pub status: Status,
}

/// What a run compares against.
#[derive(Clone, Debug)]
pub(crate) struct Base {
    /// The commit whose files are the "before" side.
    pub commit: Text,
}

/// Runs git in the repository root, failing if it exits unsuccessfully.
fn run_git(root: &Root, args: &Args) -> Result<Bytes, Failure> {
    let captured = process::git(root.path(), args)?;
    match captured.success {
        Success::No => Err(Failure::git(&Message::from_string(format!(
            "git failed: {}",
            captured.stderr.as_str().trim()
        )))),
        Success::Yes => Ok(captured.stdout),
    }
}

/// Decodes bytes as UTF-8, failing as a git error if they are invalid.
fn utf8(bytes: &Bytes) -> Result<Text, Failure> {
    match bytes.strict_text() {
        Some(text) => Ok(text),
        None => Err(Failure::git(&Message::of(
            "git reported a path that is not valid UTF-8",
        ))),
    }
}

/// The commit named by `rev`, as a full hash.
fn verified_commit(root: &Root, rev: &Text) -> Result<Text, Failure> {
    let spec = format!("{}^{{commit}}", rev.as_str());
    let out = run_git(root, &Args::of(&["rev-parse", "--verify", &spec]))?;
    Ok(Text::from_string(utf8(&out)?.as_str().trim().to_owned()))
}

/// Resolves the base commit for `range` and refuses shallow clones.
pub(crate) fn resolve_base(root: &Root, range: &Text) -> Result<Base, Failure> {
    let shallow = run_git(root, &Args::of(&["rev-parse", "--is-shallow-repository"]))?;
    if utf8(&shallow)?.as_str().trim() == "true" {
        return Err(Failure::git(&Message::of(
            "shallow clone: fetch full history (the merge base cannot be proven)",
        )));
    }
    let text = range.as_str();
    let commit = if let Some((left, raw_right)) = text.split_once("...") {
        let right = if raw_right.is_empty() {
            "HEAD"
        } else {
            raw_right
        };
        let out = run_git(root, &Args::of(&["merge-base", left, right]))?;
        Text::from_string(utf8(&out)?.as_str().trim().to_owned())
    } else if let Some((left, _)) = text.split_once("..") {
        verified_commit(root, &Text::of(left))?
    } else {
        verified_commit(root, range)?
    };
    Ok(Base { commit })
}

/// Parses NUL-separated `git diff --name-status -z` output.
fn parse_name_status(bytes: &Bytes) -> Result<Vec<Change>, Failure> {
    let text = utf8(bytes)?;
    let mut parts = text.as_str().split('\0').filter(|part| !part.is_empty());
    let mut out = Vec::new();
    while let Some(code) = parts.next() {
        let Some(path) = parts.next() else {
            return Err(Failure::git(&Message::of(
                "malformed `git diff --name-status -z` output",
            )));
        };
        let status = match code.chars().next() {
            Some('A') => Status::Added,
            Some('D') => Status::Deleted,
            Some('M' | 'T' | 'U' | 'X') => Status::Modified,
            Some(other) => {
                return Err(Failure::git(&Message::from_string(format!(
                    "unexpected git status `{other}`"
                ))))
            }
            None => continue,
        };
        out.push(Change {
            path: RelPath::of(path),
            status,
        });
    }
    Ok(out)
}

/// The union of the range diff, the working-tree diff and untracked files.
pub(crate) fn changes(root: &Root, range: &Text) -> Result<Vec<Change>, Failure> {
    let ranged = parse_name_status(&run_git(
        root,
        &Args::of(&[
            "diff",
            "--name-status",
            "--no-renames",
            "-z",
            range.as_str(),
        ]),
    )?)?;
    let working = parse_name_status(&run_git(
        root,
        &Args::of(&["diff", "--name-status", "--no-renames", "-z", "HEAD"]),
    )?)?;
    let untracked_raw = run_git(
        root,
        &Args::of(&["ls-files", "--others", "--exclude-standard", "-z"]),
    )?;
    let mut merged: BTreeMap<RelPath, Status> = BTreeMap::new();
    for change in ranged.into_iter().chain(working) {
        match merged.get(&change.path).copied() {
            Some(existing) => {
                merged.extend([(change.path, existing.then(change.status))]);
            }
            None => {
                merged.extend([(change.path, change.status)]);
            }
        }
    }
    let untracked = utf8(&untracked_raw)?;
    for path in untracked
        .as_str()
        .split('\0')
        .filter(|part| !part.is_empty())
    {
        let key = RelPath::of(path);
        match merged.get(&key).copied() {
            Some(existing) => {
                merged.extend([(key, existing.then(Status::Added))]);
            }
            None => {
                merged.extend([(key, Status::Added)]);
            }
        }
    }
    Ok(merged
        .into_iter()
        .map(|(path, status)| Change { path, status })
        .collect())
}

/// The `commit:path` object spec naming `path` at the base commit.
fn object_spec(base: &Base, path: &RelPath) -> Text {
    Text::from_string(format!("{}:{}", base.commit.as_str(), path.as_str()))
}

/// Contents of `path` at the base commit; a missing file is exit 2.
pub(crate) fn show(root: &Root, base: &Base, path: &RelPath) -> Result<Source, Failure> {
    let bytes = run_git(root, &Args::of(&["show", object_spec(base, path).as_str()]))?;
    match bytes.strict_text() {
        Some(text) => Ok(Source::from_string(text.as_str().to_owned())),
        None => Err(Failure::git(&Message::from_string(format!(
            "{} at the base commit is not valid UTF-8",
            path.as_str()
        )))),
    }
}

/// Does `path` exist at the base commit?
pub(crate) fn exists_at(root: &Root, base: &Base, path: &RelPath) -> Answer {
    run_git(
        root,
        &Args::of(&["cat-file", "-e", object_spec(base, path).as_str()]),
    )
    .is_ok()
    .answer()
}
