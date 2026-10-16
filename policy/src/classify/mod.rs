//! Classify the paths changed by a pull request. Runs base-branch code only.
//!
//! The head tree is read as data through `git show`, never built or executed.

pub(crate) mod gitdiff;
pub(crate) mod manifest;
pub(crate) mod zizmor;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod tests_git;

use crate::failure::Failure;
use crate::io;
use crate::reason::{Code, Commits, Counts, Reason};
use crate::render::{has_unsafe_chars, render_text};
use crate::types::{Answer, Count, Holds as _, Name, RelPath, Text};
use alloc::collections::BTreeSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// Directory prefixes whose files may only be added.
const APPEND_ONLY_PREFIXES: [&str; 4] = [
    "tests/fixtures/",
    "tests/expected/",
    "tests/canaries/",
    "canaries/rust/src/bin/",
];

/// Lower-case names of documents that merge without review.
const AUTO_DOC_NAMES: [&str; 3] = ["readme.md", "notes.md", "changelog.md"];

/// Lower-case names of toolchain and lint configuration files.
const TOOLCHAIN_NAMES: [&str; 9] = [
    "rust-toolchain",
    "rust-toolchain.toml",
    "clippy.toml",
    ".clippy.toml",
    ".gitignore",
    ".gitattributes",
    "deny.toml",
    ".deny.toml",
    "build.rs",
];

/// Lower-case names of documents that need review.
const PROTECTED_DOC_NAMES: [&str; 7] = [
    "license",
    "license.md",
    "license-mit",
    "license-apache",
    "security.md",
    "copying",
    "notice",
];

/// Directory prefixes of the merge-policy tooling.
const POLICY_PREFIXES: [&str; 2] = ["policy/", "ci/"];

/// Longest zizmor report read, in bytes.
const ZIZMOR_LIMIT: Count = Count::new(200_000);

/// A file as one commit holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Contents {
    /// The file does not exist.
    Absent,
    /// The file exists with this text.
    Present(Text),
}

/// Reads files of one commit.
pub(crate) trait Reader {
    /// Reads the file at `path`.
    fn read(&self, path: &RelPath) -> Contents;
}

/// The base and head trees of one change.
pub(crate) struct Sides<'tree> {
    /// The tree at the base commit.
    pub base: &'tree dyn Reader,
    /// The tree at the head commit.
    pub head: &'tree dyn Reader,
}

/// The reasons and the counts one classification produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Classified {
    /// The number of files in each bucket.
    pub counts: Counts,
    /// The reasons the change needs a human.
    pub reasons: Vec<Reason>,
}

/// Running totals of how the changed files were classified.
#[derive(Clone, Copy, Debug)]
struct Tally {
    /// Files that were only added to an append-only directory.
    append_only: Count,
    /// Files that merge automatically.
    auto: Count,
    /// Files that need a human.
    human: Count,
}

impl Tally {
    /// The counts, with `files` as the total.
    fn into_counts(self, files: Count) -> Counts {
        Counts::empty()
            .with(&Name::of("auto"), self.auto)
            .with(&Name::of("append_only"), self.append_only)
            .with(&Name::of("human"), self.human)
            .with(&Name::of("files"), files)
    }
}

/// Which count a classified file belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Bucket {
    /// The path is an addition to an append-only directory.
    AppendOnly,
    /// The path merges automatically.
    Auto,
    /// The path needs a human.
    Human,
}

/// What one changed file contributes.
struct Outcome {
    /// The bucket the path falls into.
    bucket: Bucket,
    /// The reasons for the bucket.
    reasons: Vec<Reason>,
}

impl Outcome {
    /// An outcome in `bucket` with no reasons.
    const fn free(bucket: Bucket) -> Self {
        Self {
            bucket,
            reasons: Vec::new(),
        }
    }

    /// An outcome that needs a human, for `reasons`.
    const fn human(reasons: Vec<Reason>) -> Self {
        Self {
            bucket: Bucket::Human,
            reasons,
        }
    }
}

/// Where to run the classification.
pub(crate) struct Request {
    /// The base and head commits.
    pub commits: Commits,
    /// The directory of the repository.
    pub dir: PathBuf,
    /// The zizmor binary, when it is available.
    pub zizmor: Option<PathBuf>,
}

/// Case-insensitive comparison key: lowercase, with the few characters whose
/// case folding is not their lowercase spelled out (`\u{17f}`, `\u{df}` and the
/// `\u{fb00}` to `\u{fb06}` ligatures). There is no Unicode normalisation (NFC),
/// because no tables exist without a dependency. Only non-ASCII names are
/// affected, and an unknown path needs a human anyway, so this can change a
/// reason code but cannot auto-merge anything.
fn key_of(path: &RelPath) -> Text {
    let lowered = path.as_str().to_lowercase();
    let folded: String = lowered
        .chars()
        .map(|found| match found {
            '\u{17f}' => "s".to_owned(),
            '\u{df}' => "ss".to_owned(),
            '\u{fb00}' => "ff".to_owned(),
            '\u{fb01}' => "fi".to_owned(),
            '\u{fb02}' => "fl".to_owned(),
            '\u{fb03}' => "ffi".to_owned(),
            '\u{fb04}' => "ffl".to_owned(),
            '\u{fb05}' | '\u{fb06}' => "st".to_owned(),
            other => other.to_string(),
        })
        .collect();
    Text::from_string(folded)
}

/// The last segment of a comparison key.
fn basename(key: &Text) -> Text {
    match key.as_str().rsplit_once('/') {
        Some((_, last)) => Text::of(last),
        None => key.clone(),
    }
}

/// Whether `key` names a file in an append-only directory.
fn is_append_only_key(key: &Text) -> Answer {
    APPEND_ONLY_PREFIXES
        .iter()
        .any(|prefix| key.as_str().starts_with(prefix))
        .answer()
}

/// Whether `key` names a file of the merge-policy tooling.
fn is_policy_key(key: &Text) -> Answer {
    POLICY_PREFIXES
        .iter()
        .any(|prefix| key.as_str().starts_with(prefix))
        .answer()
}

/// Whether `key` names a Markdown file.
fn is_markdown_key(key: &Text) -> Answer {
    (Path::new(key.as_str()).extension() == Some(OsStr::new("md"))).answer()
}

/// The reason code if `path` is protected, else `None`.
#[must_use]
pub(crate) fn protected_code(path: &RelPath) -> Option<Code> {
    let key = key_of(path);
    let name = basename(&key);
    let base = name.as_str();
    let text = key.as_str();
    if text.starts_with(".github/") || text == ".github" {
        return Some(if base == "codeowners" {
            Code::CodeownersChanged
        } else {
            Code::WorkflowChanged
        });
    }
    if base == "codeowners" {
        return Some(Code::CodeownersChanged);
    }
    if base == "cargo.toml" || base == "cargo.lock" {
        return Some(Code::ManifestChanged);
    }
    if TOOLCHAIN_NAMES.contains(&base)
        || format!("/{text}").contains("/.cargo/")
        || text.starts_with(".cargo/")
    {
        return Some(Code::ToolchainChanged);
    }
    if text == ".unrepresentable/config" {
        return Some(Code::ConfigFileChanged);
    }
    if text.starts_with(".unrepresentable/") {
        return Some(Code::LedgerFileChanged);
    }
    if is_policy_key(&key) == Answer::Yes {
        return Some(Code::PolicyChanged);
    }
    if text.starts_with("src/") {
        return Some(Code::RulesCodeChanged);
    }
    if PROTECTED_DOC_NAMES.contains(&base) {
        return Some(Code::ProtectedDocChanged);
    }
    if text.starts_with("tests/") && is_append_only_key(&key) == Answer::No {
        return Some(Code::TestCodeChanged);
    }
    None
}

/// Whether `path` is inside an append-only directory.
pub(crate) fn append_only_dir(path: &RelPath) -> Answer {
    is_append_only_key(&key_of(path))
}

/// Whether `path` is a document that merges automatically.
pub(crate) fn is_auto_doc(path: &RelPath) -> Answer {
    if protected_code(path).is_some() {
        return Answer::No;
    }
    let key = key_of(path);
    if key.as_str().contains('/') {
        (key.as_str().starts_with("docs/") && is_markdown_key(&key) == Answer::Yes).answer()
    } else {
        AUTO_DOC_NAMES.contains(&key.as_str()).answer()
    }
}

/// Reasons for a modified `Cargo.toml` or `Cargo.lock`: weakened lints and
/// changed dependencies. Needs both versions to compare.
fn manifest_reasons(path: &RelPath, sides: &Sides<'_>) -> Result<Vec<Reason>, Failure> {
    let key = key_of(path);
    match (sides.base.read(path), sides.head.read(path)) {
        (Contents::Present(base_text), Contents::Present(head_text)) => {
            let texts = manifest::Revisions {
                base: base_text,
                head: head_text,
            };
            match basename(&key).as_str() {
                "cargo.toml" => manifest::lint_reasons(path, &texts),
                "cargo.lock" => manifest::lock_reasons(path, &texts),
                _ => Ok(Vec::new()),
            }
        }
        (Contents::Present(_) | Contents::Absent, Contents::Present(_) | Contents::Absent) => {
            Ok(Vec::new())
        }
    }
}

/// The outcome for a path in an append-only directory.
fn append_only_outcome(entry: &gitdiff::Entry) -> Outcome {
    let how = match entry.status {
        gitdiff::Status::Added => return Outcome::free(Bucket::AppendOnly),
        gitdiff::Status::Deleted => "deleted",
        gitdiff::Status::Modified
        | gitdiff::Status::TypeChanged
        | gitdiff::Status::Unmerged
        | gitdiff::Status::Unknown => "modified",
    };
    Outcome::human(vec![Reason::at(
        Code::JudgeModified,
        &Text::from_string(format!(
            "{} (append-only directory, {how})",
            entry.path.as_str()
        )),
        &entry.path,
        Count::new(1),
    )])
}

/// The outcome for a path that needs protection.
fn protected_outcome(
    entry: &gitdiff::Entry,
    code: Code,
    sides: &Sides<'_>,
) -> Result<Outcome, Failure> {
    let mut reasons = vec![Reason::at(
        code,
        &entry.path.to_text(),
        &entry.path,
        Count::new(1),
    )];
    if entry.status.is_modification() == Answer::Yes {
        reasons.extend(manifest_reasons(&entry.path, sides)?);
    }
    Ok(Outcome::human(reasons))
}

/// The outcome for one changed path.
fn classify_entry(
    entry: &gitdiff::Entry,
    bad: &BTreeSet<RelPath>,
    sides: &Sides<'_>,
) -> Result<Outcome, Failure> {
    let path = &entry.path;
    if has_unsafe_chars(&path.to_text()) == Answer::Yes {
        return Ok(Outcome::human(vec![Reason::general(
            Code::UnsafePathChars,
            &render_text(&path.to_text()),
        )]));
    }
    if bad.contains(path) {
        return Ok(Outcome::human(vec![Reason::at(
            Code::SymlinkOrSubmodule,
            &Text::from_string(format!("{} (symlink or submodule)", path.as_str())),
            path,
            Count::new(1),
        )]));
    }
    if append_only_dir(path) == Answer::Yes {
        return Ok(append_only_outcome(entry));
    }
    if let Some(code) = protected_code(path) {
        return protected_outcome(entry, code, sides);
    }
    if is_auto_doc(path) == Answer::Yes {
        return Ok(Outcome::free(Bucket::Auto));
    }
    Ok(Outcome::human(vec![Reason::at(
        Code::OtherPathChanged,
        &path.to_text(),
        path,
        Count::new(1),
    )]))
}

/// Classifies the changed `entries`; `bad` holds symlinks and submodules.
pub(crate) fn classify(
    entries: &[gitdiff::Entry],
    bad: &BTreeSet<RelPath>,
    sides: &Sides<'_>,
) -> Result<Classified, Failure> {
    let mut reasons = Vec::new();
    let mut tally = Tally {
        append_only: Count::new(0),
        auto: Count::new(0),
        human: Count::new(0),
    };
    for entry in entries {
        let outcome = classify_entry(entry, bad, sides)?;
        match outcome.bucket {
            Bucket::Auto => tally.auto = tally.auto.next()?,
            Bucket::AppendOnly => tally.append_only = tally.append_only.next()?,
            Bucket::Human => tally.human = tally.human.next()?,
        }
        reasons.extend(outcome.reasons);
    }
    Ok(Classified {
        counts: tally.into_counts(Count::new(entries.len())),
        reasons,
    })
}

/// Classifies the pull request in `request.dir` (a repository holding both
/// commits). A diff that cannot be read is itself a reason.
pub(crate) fn run(request: &Request) -> Result<Classified, Failure> {
    let Ok(changed) = gitdiff::changes(&request.dir, &request.commits) else {
        return Ok(Classified {
            counts: Counts::empty().with(&Name::of("files"), Count::new(0)),
            reasons: vec![Reason::general(
                Code::DiffUnparsable,
                &Text::of("git diff output could not be parsed"),
            )],
        });
    };
    let base = gitdiff::GitTree {
        dir: request.dir.clone(),
        rev: request.commits.base.clone(),
    };
    let head = gitdiff::GitTree {
        dir: request.dir.clone(),
        rev: request.commits.head.clone(),
    };
    let mut classified = classify(
        &changed.entries,
        &changed.symlinks_and_submodules,
        &Sides {
            base: &base,
            head: &head,
        },
    )?;
    if let Some(report) = &request.zizmor {
        let text = io::read_bytes_limited(report, ZIZMOR_LIMIT)?.lossy_text();
        classified.reasons.extend(zizmor::reasons(&text)?);
    }
    Ok(classified)
}
