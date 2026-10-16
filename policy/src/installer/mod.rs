//! Generate the release asset `unrepresentable.sh` from `ci/unrepresentable.sh`.
//!
//! `SHA256SUMS` lines look like `<64 hex>  unrepresentable-<target>[.exe]`.
//! Every placeholder must be filled; anything left over is an error.

#[cfg(test)]
mod tests;

use crate::failure::Failure;
use crate::types::{Answer, Holds as _, Message, Name, Text};
use alloc::collections::BTreeMap;

/// The platforms the installer supports.
pub(crate) const TARGETS: [&str; 5] = [
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc",
];

/// The length of a sha256 digest in hex digits.
const DIGEST_LENGTH: usize = 64;

/// The inputs of a generated installer.
pub(crate) struct Installer {
    /// The text of the checksum file.
    pub sums: Text,
    /// The installer script template.
    pub template: Text,
    /// The version to install.
    pub version: Text,
}

/// Whether `text` is a version in the form `N.N.N` or `N.N.N-PRERELEASE` (numbers without leading zeros),
/// exactly: a trailing newline, build metadata (`+...`) and a `v` prefix are
/// refused. The pre-release part is one or more dot-separated identifiers of
/// `[0-9A-Za-z-]`, none empty, numeric ones without leading zeros.
fn is_version(text: &Text) -> Answer {
    let (core, pre_release) = match text.as_str().split_once('-') {
        Some((before, after)) => (before, Some(after)),
        None => (text.as_str(), None),
    };
    let core_parts: Vec<&str> = core.split('.').collect();
    let core_ok = core_parts.len() == 3
        && core_parts
            .iter()
            .all(|part| is_number(&Text::of(part)) == Answer::Yes);
    let pre_release_ok = pre_release.is_none_or(|rest| {
        rest.split('.')
            .all(|part| is_identifier(&Text::of(part)) == Answer::Yes)
    });
    (core_ok && pre_release_ok).answer()
}

/// Whether `text` is a number without a leading zero: `0` or digits starting
/// with 1 to 9.
fn is_number(text: &Text) -> Answer {
    let part = text.as_str();
    (!part.is_empty()
        && part.chars().all(|found| found.is_ascii_digit())
        && (part.len() == 1 || !part.starts_with('0')))
    .answer()
}

/// Whether `part` is one pre-release identifier: not empty, only
/// `[0-9A-Za-z-]`, and when all digits then without a leading zero.
fn is_identifier(text: &Text) -> Answer {
    let part = text.as_str();
    let charset_ok = !part.is_empty()
        && part
            .chars()
            .all(|found| found.is_ascii_alphanumeric() || found == '-');
    let numeric = part.chars().all(|found| found.is_ascii_digit());
    (charset_ok && !(numeric && part.len() > 1 && part.starts_with('0'))).answer()
}

/// One `SHA256SUMS` line: the digest and the binary name without `.exe`.
#[must_use]
fn sums_line(line: &Text) -> Option<(Name, Text)> {
    let (digest, name) = line.as_str().split_once("  ")?;
    let digest_ok = digest.len() == DIGEST_LENGTH
        && digest
            .chars()
            .all(|found| found.is_ascii_digit() || matches!(found, 'a'..='f'));
    let stem = name.strip_prefix("unrepresentable-")?;
    let stem_body = if let Some(found) = stem.strip_suffix(".exe") {
        found
    } else {
        stem
    };
    let name_ok = !stem_body.is_empty()
        && stem_body.chars().all(|found| {
            found.is_ascii_lowercase() || found.is_ascii_digit() || matches!(found, '_' | '-')
        });
    (digest_ok && name_ok).then(|| (Name::of(&name.replace(".exe", "")), Text::of(digest)))
}

/// Binary name to digest.
pub(crate) fn parse_sums(text: &Text) -> Result<BTreeMap<Name, Text>, Failure> {
    let mut sums = BTreeMap::new();
    for line in text.as_str().lines() {
        if line.trim().is_empty() {
            continue;
        }
        let (name, digest) = sums_line(&Text::of(line))
            .ok_or_else(|| Failure::refused(&Message::of("bad SHA256SUMS line")))?;
        match sums.insert(name, digest) {
            Some(_) | None => {}
        }
    }
    Ok(sums)
}

/// Whether any `@VERSION@` or `@SHA256_..@` marker is left in `text`.
fn has_placeholder(text: &Text) -> Answer {
    text.as_str()
        .split('@')
        .skip(1)
        .any(|piece| {
            piece == "VERSION"
                || piece.strip_prefix("SHA256_").is_some_and(|rest| {
                    !rest.is_empty()
                        && rest.chars().all(|found| {
                            found.is_ascii_uppercase() || found.is_ascii_digit() || found == '_'
                        })
                })
        })
        .answer()
}

/// The installer script with the version and every digest filled in.
pub(crate) fn generate(installer: &Installer) -> Result<Text, Failure> {
    if is_version(&installer.version) == Answer::No {
        return Err(Failure::refused(&Message::of("bad version")));
    }
    let sums = parse_sums(&installer.sums)?;
    let mut out = installer
        .template
        .as_str()
        .replace("@VERSION@", installer.version.as_str());
    for target in TARGETS {
        let key = Name::of(&format!("unrepresentable-{target}"));
        let digest = sums.get(&key).ok_or_else(|| {
            Failure::refused(&Message::from_string(format!("no hash for {target}")))
        })?;
        let marker = format!("@SHA256_{}@", target.to_uppercase().replace('-', "_"));
        out = out.replace(&marker, digest.as_str());
    }
    if has_placeholder(&Text::of(&out)) == Answer::Yes {
        return Err(Failure::refused(&Message::of("unfilled placeholder")));
    }
    Ok(Text::from_string(out))
}
