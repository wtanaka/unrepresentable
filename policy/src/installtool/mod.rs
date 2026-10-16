//! Install a pinned tool from `policy/tools.lock` into a directory, verifying
//! its sha256.

#[cfg(test)]
mod tests;

use crate::differential::corpus::{download, safe_extract, Download, Unpack};
use crate::exit::Report;
use crate::failure::Failure;
use crate::io::{self, CopyPlan, EntryKind};
use crate::process;
use crate::types::{Answer, Holds as _, Message, Name, Text};
use alloc::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The length of a sha256 digest in hex digits.
const DIGEST_LENGTH: usize = 64;
/// The platforms a pinned tool is installed for: the Linux runners of the
/// workflows.
const TRIPLES: [&str; 2] = ["x86_64-unknown-linux-gnu", "x86_64-unknown-linux-musl"];

/// One pinned tool: its version, where to download it and its sha256.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Tool {
    /// The expected sha256 digest.
    pub digest: Text,
    /// The URL to download from.
    pub url: Text,
    /// The version to install.
    pub version: Text,
}

/// Which tool to install, from which lock file, and where.
pub(crate) struct Install {
    /// Where to put the binary.
    pub dest: PathBuf,
    /// The lock file that pins the tool.
    pub lock: PathBuf,
    /// The name of the tool.
    pub tool: Name,
}

/// Whether `part` is a GitHub owner, repository or file name: letters, digits
/// and `.`, `_`, `-`, and not a dot name.
fn is_plain_segment(part: &Name) -> Answer {
    (!part.as_str().is_empty()
        && part.as_str() != "."
        && part.as_str() != ".."
        && part
            .as_str()
            .chars()
            .all(|found| found.is_ascii_alphanumeric() || matches!(found, '.' | '_' | '-')))
    .answer()
}

/// Whether the URL of `tool` is the release asset of the tool called `name`:
/// exactly `https://github.com/<owner>/<repo>/releases/download/<tag>/<asset>`,
/// where the tag is the version (with or without a leading `v`) and the asset is
/// `<name>-<platform>.tar.gz` or `<name>-<version>-<platform>.tar.gz`. A lock
/// line whose URL names another version or another file than it says is
/// refused, so a version bump cannot leave the old URL, or a different
/// download, behind.
fn is_release_asset(name: &Name, tool: &Tool) -> Answer {
    let Some(tail) = tool.url.as_str().strip_prefix("https://github.com/") else {
        return Answer::No;
    };
    let parts: Vec<&str> = tail.split('/').collect();
    let [owner, repo, "releases", "download", tag, asset] = parts.as_slice() else {
        return Answer::No;
    };
    let version = tool.version.as_str();
    let plain_names = [*owner, *repo, *asset]
        .iter()
        .all(|part| is_plain_segment(&Name::of(part)) == Answer::Yes);
    let asset_ok = TRIPLES.iter().any(|triple| {
        *asset == format!("{}-{triple}.tar.gz", name.as_str())
            || *asset == format!("{}-{version}-{triple}.tar.gz", name.as_str())
    });
    (plain_names && (*tag == version || *tag == format!("v{version}")) && asset_ok).answer()
}

/// Parses `<name> <version> <https://github.com/... url> <sha256>`.
#[must_use]
fn tool_line(line: &Text) -> Option<(Name, Tool)> {
    let parts: Vec<&str> = line.as_str().split(' ').collect();
    let [name, version, url, digest] = parts.as_slice() else {
        return None;
    };
    let name_ok = !name.is_empty()
        && name
            .chars()
            .all(|found| found.is_ascii_lowercase() || found.is_ascii_digit() || found == '-');
    let version_ok = version
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_digit())
        && version
            .chars()
            .all(|found| found.is_ascii_alphanumeric() || matches!(found, '.' | '-'));
    let claimed = Tool {
        digest: Text::of(digest),
        url: Text::of(url),
        version: Text::of(version),
    };
    let url_ok = is_release_asset(&Name::of(name), &claimed) == Answer::Yes;
    let digest_ok = digest.len() == DIGEST_LENGTH
        && digest
            .chars()
            .all(|found| found.is_ascii_digit() || matches!(found, 'a'..='f'));
    (name_ok && version_ok && url_ok && digest_ok).then(|| (Name::of(name), claimed))
}

/// The tools of a `tools.lock` text; comments and blanks are skipped and any
/// other unparsable line is an error.
pub(crate) fn parse_lock(text: &Text) -> Result<BTreeMap<Name, Tool>, Failure> {
    let mut tools = BTreeMap::new();
    for raw in text.as_str().lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (name, tool) = tool_line(&Text::of(line))
            .ok_or_else(|| Failure::parse(&Message::of("bad tools.lock line")))?;
        match tools.insert(name, tool) {
            Some(_) | None => {}
        }
    }
    Ok(tools)
}

/// The first file named `wanted` under `dir`, directories visited in order.
fn find_named(dir: &Path, wanted: &Name) -> Result<Option<PathBuf>, Failure> {
    let entries = io::list_dir(dir)?;
    if let Some(entry) = entries
        .iter()
        .find(|item| item.kind == EntryKind::File && item.name == *wanted)
    {
        return Ok(Some(dir.join(entry.name.as_str())));
    }
    for entry in entries.iter().filter(|item| item.kind == EntryKind::Dir) {
        if let Some(found) = find_named(&dir.join(entry.name.as_str()), wanted)? {
            return Ok(Some(found));
        }
    }
    Ok(None)
}

/// Downloads the tool's archive, checks its sha256 and unpacks the binary
/// into `dest`. A mismatch or a missing binary fails the step.
pub(crate) fn install(request: &Install) -> Result<Report, Failure> {
    let tools = parse_lock(&io::read_text(&request.lock)?)?;
    let Some(tool) = tools.get(&request.tool) else {
        return Err(Failure::refused(&Message::from_string(format!(
            "{} is not in the tools lock",
            request.tool.as_str()
        ))));
    };
    let scratch = process::make_temp_dir()?;
    let outcome = fetch_binary(request, tool, &scratch);
    io::remove_tree(&scratch)?;
    outcome
}

/// Downloads a tool binary and checks its digest.
fn fetch_binary(request: &Install, tool: &Tool, scratch: &Path) -> Result<Report, Failure> {
    let blob = scratch.join("tool.tar.gz");
    download(&Download {
        bearer: None,
        to: blob.clone(),
        url: tool.url.clone(),
    })?;
    if process::sha256_of(&blob)?.as_str() != tool.digest.as_str() {
        return Ok(Report::saying(&Text::from_string(format!(
            "sha256 mismatch for {}",
            request.tool.as_str()
        )))
        .into_failed());
    }
    let unpacked = scratch.join("unpacked");
    safe_extract(&Unpack {
        archive: blob,
        dest: unpacked.clone(),
    })?;
    let Some(binary) = find_named(&unpacked, &request.tool)? else {
        return Ok(Report::saying(&Text::from_string(format!(
            "binary {} not found in archive",
            request.tool.as_str()
        )))
        .into_failed());
    };
    io::create_dirs(&request.dest)?;
    let target = request.dest.join(request.tool.as_str());
    io::copy_file(&CopyPlan {
        from: binary,
        to: target.clone(),
    })?;
    process::make_executable(&target)?;
    Ok(Report::clean())
}
