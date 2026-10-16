//! Glob grammar: `*` within one segment, `**` as a whole segment for zero or
//! more segments, `?` one character, a trailing `/` matches the directory and
//! everything below it. Matching is on repo-relative `/`-separated paths,
//! case-sensitive.
//!
//! The `glob` crate does the matching. This module only rewrites the pattern
//! text into what the crate accepts and fixes the places where its meaning
//! differs:
//! - a final `**` segment matches zero segments here, so `a/**` also matches `a`;
//! - a trailing `/` means the directory and its contents: `a/` matches `a`
//!   itself (so a file named `a` is covered too) and `a/b/c`, and never `ab/c`;
//! - a `**` that is not a whole segment means `*`;
//! - `[` is literal for the globs of `.unrepresentable/config` ([`Brackets::Literal`]
//!   makes `a[b]` match the text `a[b]`) and a character class for workspace
//!   member patterns ([`Brackets::Class`], what cargo reads with the same crate).
//!
//! A pattern is compiled once, when it is parsed. Text that cannot be a
//! repo-relative glob is an error at that point and never falls back to
//! literal matching: an empty pattern, a leading `/` (a repo-relative path never
//! starts with one, so the glob could not match anything), and, in
//! [`Brackets::Class`] mode, a character class the `glob` crate rejects.

#[cfg(test)]
mod tests;

use ::glob::{MatchOptions, Pattern};

use crate::types::{Answer, Holds as _, Message, Name, RelPath, Text};

/// Case-sensitive matching in which `*` and `?` never cross a `/`.
const OPTIONS: MatchOptions = MatchOptions {
    case_sensitive: true,
    require_literal_leading_dot: false,
    require_literal_separator: true,
};

/// How `[` is read in a pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Brackets {
    /// `[` starts a character class, as in cargo's workspace member patterns.
    Class,
    /// `[` is the literal character, as in every glob of the config file.
    Literal,
}

/// A path glob such as `src/**` or `tests/*.rs`, compiled once: the pattern as
/// written, then the `glob` crate patterns that together mean it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Glob(Text, Vec<Pattern>);

impl Glob {
    /// Whether the glob matches the repo-relative `path`.
    pub(crate) fn matches(&self, path: &RelPath) -> Answer {
        self.1
            .iter()
            .any(|pattern| pattern.matches_with(path.as_str(), OPTIONS))
            .answer()
    }

    /// Compiles `text`, or says why it is not a repo-relative glob.
    pub(crate) fn parse(text: &Text, brackets: Brackets) -> Result<Self, Message> {
        let raw = text.as_str();
        if raw.is_empty() {
            return Err(Message::of("a glob must not be empty"));
        }
        if raw.starts_with('/') {
            return Err(Message::from_string(format!(
                "glob `{raw}` starts with `/`, but globs are repo-relative and never match such a path"
            )));
        }
        let patterns = translate(text, brackets)
            .iter()
            .map(|candidate| {
                Pattern::new(candidate.as_str()).map_err(|error| {
                    Message::from_string(format!("glob `{raw}` is invalid: {error}"))
                })
            })
            .collect::<Result<Vec<Pattern>, Message>>()?;
        Ok(Self(text.clone(), patterns))
    }

    /// The glob as written.
    pub(crate) const fn text(&self) -> &Text {
        &self.0
    }
}

/// One path segment of a glob with every `**` run turned into `*`, and `[`
/// made literal unless `brackets` keeps it; a segment that is exactly `**` is
/// kept.
fn segment(text: &Name, brackets: Brackets) -> Name {
    if text.as_str() == "**" {
        return text.clone();
    }
    let mut out = String::new();
    let mut after_star = false;
    for found in text.as_str().chars() {
        match found {
            '*' if after_star => {}
            '*' => {
                out.push('*');
                after_star = true;
            }
            '[' => {
                match brackets {
                    Brackets::Literal => out.push_str("[[]"),
                    Brackets::Class => out.push('['),
                }
                after_star = false;
            }
            other => {
                out.push(other);
                after_star = false;
            }
        }
    }
    Name::from_string(out)
}

/// The `glob` crate patterns that together mean `raw`.
fn translate(raw: &Text, brackets: Brackets) -> Vec<Text> {
    let text = raw.as_str();
    let mut segments: Vec<Name> = text
        .trim_end_matches('/')
        .split('/')
        .map(|part| segment(&Name::of(part), brackets))
        .collect();
    if text.ends_with('/') {
        segments.push(Name::of("**"));
    }
    let joined = |parts: &[Name]| {
        Text::from_string(
            parts
                .iter()
                .map(Name::as_str)
                .collect::<Vec<&str>>()
                .join("/"),
        )
    };
    let mut out = vec![joined(&segments)];
    if let Some((last, init)) = segments.split_last() {
        if last.as_str() == "**" && !init.is_empty() {
            out.push(joined(init));
        }
    }
    out
}
