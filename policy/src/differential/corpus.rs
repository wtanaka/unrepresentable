//! Fetching and unpacking the differential corpora.

use crate::bytes::Bytes;
use crate::failure::Failure;
use crate::io::{self, Vars};
use crate::process::{self, Args, Invocation};
use crate::types::{Answer, Count, Holds as _, Message, Name, Text};
use std::path::{Path, PathBuf};

/// The largest corpus archive accepted, compressed or not.
const MAX_ARCHIVE_BYTES: usize = 0x2000_0000;

/// The longest archive name accepted.
const MAX_ARCHIVE_NAME: usize = 64;
/// The length of a sha256 digest in hex digits.
const DIGEST_LENGTH: usize = 64;

/// What `tar` lists of an archive: the names (`tar -tf`) and the details
/// (`tar -tvf`).
pub(crate) struct Listing {
    /// The output of `tar -tvf`.
    pub details: Text,
    /// The output of `tar -tf`.
    pub names: Text,
}

/// One line of `corpus.lock`: where to fetch a corpus, its sha256 and a name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LockLine {
    /// The expected sha256 digest of the archive.
    pub digest: Text,
    /// The name of the corpus.
    pub name: Name,
    /// The URL of the archive.
    pub url: Text,
}

/// What kind of thing an archive member is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MemberKind {
    /// A directory.
    Directory,
    /// Anything else, such as a link or a device.
    Other,
    /// A regular file.
    PlainFile,
}

/// One archive member as `tar -tv` shows it.
struct Member {
    /// What kind of member this is.
    kind: MemberKind,
    /// The size of the member in bytes.
    size: Count,
}

/// An archive and the directory to unpack it into.
pub(crate) struct Unpack {
    /// The archive to unpack.
    pub archive: PathBuf,
    /// The directory to unpack into.
    pub dest: PathBuf,
}

/// A download: where from, where to, and an optional bearer token.
pub(crate) struct Download {
    /// A bearer token to send, when the download needs one.
    pub bearer: Option<Text>,
    /// Where to write the download.
    pub to: PathBuf,
    /// The URL to download.
    pub url: Text,
}

/// The lock file to read and the directory to unpack the corpora into.
pub(crate) struct PublicCorpus {
    /// The directory to unpack the corpora into.
    pub dest: PathBuf,
    /// The lock file that names the corpora.
    pub lock: PathBuf,
}

/// A scratch directory for downloads and the final destination.
struct Staging {
    /// The final destination.
    dest: PathBuf,
    /// The scratch directory for downloads.
    scratch: PathBuf,
}

/// Parses `<https url> <sha256> <name>`.
#[must_use]
pub(crate) fn parse_lock_line(line: &Text) -> Option<LockLine> {
    let parts: Vec<&str> = line.as_str().split(' ').collect();
    let [url, digest, name] = parts.as_slice() else {
        return None;
    };
    let body = url.strip_prefix("https://")?;
    let url_ok = !body.is_empty() && !body.chars().any(char::is_whitespace);
    let digest_ok = digest.len() == DIGEST_LENGTH
        && digest
            .chars()
            .all(|found| found.is_ascii_digit() || matches!(found, 'a'..='f'));
    let name_ok = !name.is_empty()
        && name.len() <= MAX_ARCHIVE_NAME
        && name
            .chars()
            .all(|found| found.is_ascii_alphanumeric() || matches!(found, '.' | '_' | '-'));
    (url_ok && digest_ok && name_ok).then(|| LockLine {
        digest: Text::of(digest),
        name: Name::of(name),
        url: Text::of(url),
    })
}

/// The lock lines of a `corpus.lock` text; comments and blanks are skipped
/// and any other unparsable line is an error.
pub(crate) fn parse_lock(text: &Text) -> Result<Vec<LockLine>, Failure> {
    let mut lines = Vec::new();
    for raw in text.as_str().lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        lines.push(
            parse_lock_line(&Text::of(line))
                .ok_or_else(|| Failure::parse(&Message::of("bad corpus.lock line")))?,
        );
    }
    Ok(lines)
}

/// The failure for an archive member that is not safe to unpack.
fn unsafe_member() -> Failure {
    Failure::refused(&Message::of("unsafe archive member"))
}

/// The type letter and size of a `tar -tv` line (GNU or bsdtar layout).
fn member_of(line: &Text) -> Result<Member, Failure> {
    let kind = match line.as_str().chars().next().ok_or_else(unsafe_member)? {
        '-' => MemberKind::PlainFile,
        'd' => MemberKind::Directory,
        _ => MemberKind::Other,
    };
    let tokens: Vec<&str> = line.as_str().split_whitespace().collect();
    let size_token = if tokens.get(1).is_some_and(|owner| owner.contains('/')) {
        tokens.get(2)
    } else {
        tokens.get(4)
    };
    let size = size_token
        .and_then(|token| Count::parse(&Text::of(token)))
        .ok_or_else(unsafe_member)?;
    Ok(Member { kind, size })
}

/// Whether `name` is a safe archive name.
fn name_is_safe(name: &Text) -> Answer {
    let text = name.as_str();
    (!text.is_empty() && !text.starts_with('/') && !text.split('/').any(|part| part == ".."))
        .answer()
}

/// Refuses an archive listing unless every member is a plain file or
/// directory with a relative name inside the tree, within the size limit. The
/// names and the details are paired by line, so a name with a control character
/// (a newline would split one member over two lines) is refused, and each
/// detail line must end with the name that stands beside it.
pub(crate) fn check_listing(listing: &Listing) -> Result<(), Failure> {
    let name_lines: Vec<&str> = listing.names.as_str().lines().collect();
    let detail_lines: Vec<&str> = listing.details.as_str().lines().collect();
    if name_lines.len() != detail_lines.len() {
        return Err(unsafe_member());
    }
    let mut total = Count::new(0);
    for (name, detail) in name_lines.iter().zip(&detail_lines) {
        let member = member_of(&Text::of(detail))?;
        let plain = match member.kind {
            MemberKind::PlainFile | MemberKind::Directory => true,
            MemberKind::Other => false,
        };
        let listed = detail.trim_end().ends_with(name.trim_end());
        if name_is_safe(&Text::of(name)) == Answer::No
            || name.chars().any(char::is_control)
            || !listed
            || !plain
        {
            return Err(unsafe_member());
        }
        total = total.plus(member.size)?;
        if total.get() > MAX_ARCHIVE_BYTES {
            return Err(Failure::refused(&Message::of("archive too large")));
        }
    }
    Ok(())
}

/// Lists an archive and refuses it unless `check_listing` approves it.
pub(crate) fn check_archive(archive: &Path) -> Result<(), Failure> {
    let target = archive.to_string_lossy().into_owned();
    let program = Name::of("tar");
    let names = process::run(&Invocation::new(&program, &Args::of(&["-tf", &target])))?
        .stdout_or_fail(&program)?
        .lossy_text();
    let verbose = process::run(&Invocation::new(&program, &Args::of(&["-tvf", &target])))?
        .stdout_or_fail(&program)?
        .lossy_text();
    check_listing(&Listing {
        details: verbose,
        names,
    })
}

/// Extracts an archive into its destination after `check_archive` approves it.
pub(crate) fn safe_extract(job: &Unpack) -> Result<(), Failure> {
    check_archive(&job.archive)?;
    io::create_dirs(&job.dest)?;
    let program = Name::of("tar");
    let args = Args::of(&[
        "-xf",
        &job.archive.to_string_lossy(),
        "--no-same-owner",
        "-C",
        &job.dest.to_string_lossy(),
    ]);
    process::run(&Invocation::new(&program, &args))?
        .stdout_or_fail(&program)
        .map(|_output| ())
}

/// Whether `value` is safe to put in a curl configuration.
fn config_value_is_safe(value: &Text) -> Answer {
    (!value
        .as_str()
        .chars()
        .any(|found| found == '"' || found == '\\' || found.is_control()))
    .answer()
}

/// Downloads over https with curl. The URL and token go to curl on standard
/// input (as a config), never on the command line.
pub(crate) fn download(request: &Download) -> Result<(), Failure> {
    let token_safe = match &request.bearer {
        Some(token) => config_value_is_safe(token) == Answer::Yes,
        None => true,
    };
    if !request.url.as_str().starts_with("https://")
        || config_value_is_safe(&request.url) == Answer::No
        || !token_safe
    {
        return Err(Failure::refused(&Message::of("unsafe download request")));
    }
    let mut config = format!(
        "url = \"{}\"\noutput = \"{}\"\n",
        request.url.as_str(),
        request.to.to_string_lossy()
    );
    if let Some(token) = &request.bearer {
        let header = format!("header = \"Authorization: Bearer {}\"\n", token.as_str());
        config.push_str(&header);
    }
    let limit = MAX_ARCHIVE_BYTES.to_string();
    let args = Args::of(&[
        "--fail",
        "--silent",
        "--show-error",
        "--location",
        "--proto",
        "=https",
        "--proto-redir",
        "=https",
        "--max-time",
        "600",
        "--max-filesize",
        &limit,
        "--config",
        "-",
    ]);
    let program = Name::of("curl");
    let captured =
        process::run(&Invocation::new(&program, &args).feeding(&Bytes::of(config.as_bytes())))?;
    captured.stdout_or_fail(&program).map(|_output| ())
}

/// Downloads the public corpora listed in the lock, verifies each sha256 and
/// unpacks it under `dest/<name>`.
pub(crate) fn fetch_public(corpus: &PublicCorpus) -> Result<(), Failure> {
    let lines = parse_lock(&io::read_text(&corpus.lock)?)?;
    if lines.is_empty() {
        return Err(Failure::parse(&Message::of("corpus.lock lists no corpus")));
    }
    let scratch = process::make_temp_dir()?;
    let staging = Staging {
        dest: corpus.dest.clone(),
        scratch: scratch.clone(),
    };
    let result = fetch_lines(&lines, &staging);
    io::remove_tree(&scratch)?;
    result
}

/// Downloads and unpacks every corpus the lock lines name.
fn fetch_lines(lines: &[LockLine], staging: &Staging) -> Result<(), Failure> {
    for line in lines {
        let blob = staging.scratch.join("corpus.tar");
        download(&Download {
            bearer: None,
            to: blob.clone(),
            url: line.url.clone(),
        })?;
        if process::sha256_of(&blob)?.as_str() != line.digest.as_str() {
            return Err(Failure::refused(&Message::from_string(format!(
                "corpus hash mismatch for {}",
                line.name.as_str()
            ))));
        }
        safe_extract(&Unpack {
            archive: blob.clone(),
            dest: staging.dest.join(line.name.as_str()),
        })?;
        io::remove_file(&blob)?;
    }
    Ok(())
}

/// Downloads the hidden corpus named by `HIDDEN_CORPUS_URL` with
/// `HIDDEN_CORPUS_TOKEN` and unpacks it into `dest`.
pub(crate) fn fetch_hidden(vars: &Vars, dest: &Path) -> Result<(), Failure> {
    let url = vars.text(&Name::of("HIDDEN_CORPUS_URL"));
    let token = vars.text(&Name::of("HIDDEN_CORPUS_TOKEN"));
    if !url.as_str().starts_with("https://") || token.as_str().is_empty() {
        return Err(Failure::refused(&Message::of(
            "hidden corpus is not configured",
        )));
    }
    let scratch = process::make_temp_dir()?;
    let blob = scratch.join("corpus.tar");
    let result = download(&Download {
        bearer: Some(token),
        to: blob.clone(),
        url,
    })
    .and_then(|()| {
        safe_extract(&Unpack {
            archive: blob.clone(),
            dest: dest.to_path_buf(),
        })
    });
    io::remove_tree(&scratch)?;
    result
}
