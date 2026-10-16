//! Minimal CODEOWNERS parser (the last matching rule applies). The `glob` crate does
//! the pattern matching.
//!
//! A pattern means: a leading `/` (or any `/` inside the pattern) anchors it to
//! the root, otherwise it may start after any `/`; a trailing `/` matches
//! everything below a directory but not the directory itself; a pattern
//! without a trailing `/` whose last segment has no `*` or `?` matches a file or
//! a directory and everything below it; a last segment with a wildcard
//! (`docs/*`, `*.js`) matches single names only, so `docs/*` covers `docs/a.md`
//! and not `docs/a/b.md`, as GitHub documents; `*` and `?` never cross a `/`; a `**/` segment matches any number of
//! directories and a final `/**` matches everything below. A `**` that is not
//! a whole segment means `*`, and `[` is a literal character, as in every
//! earlier version of this matcher (the `glob` crate would read a character
//! class). Each pattern becomes one or two `glob` patterns, any of which
//! matching means the rule matches.

#[cfg(test)]
mod tests;

use ::glob::{MatchOptions, Pattern};

use crate::failure::Failure;
use crate::types::{Answer, Holds as _, Message, Name, RelPath, Text};

/// Case-sensitive matching in which `*` and `?` never cross a `/`.
const OPTIONS: MatchOptions = MatchOptions {
    case_sensitive: true,
    require_literal_leading_dot: false,
    require_literal_separator: true,
};

/// A pattern and the owners it names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Rule {
    /// The owners the pattern names.
    owners: Vec<Name>,
    /// The `glob` patterns, any of which matching means the rule matches.
    patterns: Vec<Pattern>,
}

impl Rule {
    /// Whether the rule's pattern matches `path`.
    fn matches(&self, path: &RelPath) -> Answer {
        self.patterns
            .iter()
            .any(|pattern| pattern.matches_with(path.as_str(), OPTIONS))
            .answer()
    }
}

/// One path segment of a pattern with every `**` run turned into `*` and `[`
/// made literal.
fn plain_segment(text: &Name) -> Name {
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
                out.push_str("[[]");
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

/// The `glob` pattern texts that together mean the CODEOWNERS pattern.
fn translate(pattern: &Text) -> Vec<Text> {
    let text = pattern.as_str();
    let anchored = text.starts_with('/');
    let directory_only = text.ends_with('/');
    let segments: Vec<&str> = text.trim_matches('/').split('/').collect();
    let rooted = anchored || segments.len() > 1;
    let joined: Vec<String> = segments
        .iter()
        .map(|segment| {
            if *segment == "**" && segments.len() > 1 {
                (*segment).to_owned()
            } else {
                plain_segment(&Name::of(segment)).as_str().to_owned()
            }
        })
        .collect();
    let prefix = if rooted { "" } else { "**/" };
    let whole = format!("{prefix}{}", joined.join("/"));
    let ends_below = segments.last() == Some(&"**") && segments.len() > 1;
    let wildcard_name = segments
        .last()
        .is_some_and(|last| last.contains(['*', '?']));
    if ends_below || (wildcard_name && !directory_only) {
        vec![Text::from_string(whole)]
    } else if directory_only {
        vec![Text::from_string(format!("{whole}/**"))]
    } else {
        vec![
            Text::from_string(whole.clone()),
            Text::from_string(format!("{whole}/**")),
        ]
    }
}

/// Compiles the `glob` patterns for one CODEOWNERS pattern.
fn compile(pattern: &Text) -> Result<Vec<Pattern>, Failure> {
    translate(pattern)
        .iter()
        .map(|candidate| {
            Pattern::new(candidate.as_str()).map_err(|error| {
                Failure::parse(&Message::from_string(format!(
                    "CODEOWNERS pattern {} cannot be matched: {error}",
                    pattern.as_str()
                )))
            })
        })
        .collect()
}

/// Parses CODEOWNERS text. Only `@owner` words are kept; teams are kept too
/// (and never resolve to a person).
pub(crate) fn parse(text: &Text) -> Result<Vec<Rule>, Failure> {
    let mut rules = Vec::new();
    for raw in text.as_str().lines() {
        let line = match raw.split_once('#') {
            Some((before, _)) => before,
            None => raw,
        };
        let mut words = line.split_whitespace();
        let Some(pattern) = words.next() else {
            continue;
        };
        let owners = words
            .filter(|word| word.starts_with('@'))
            .map(Name::of)
            .collect();
        rules.push(Rule {
            owners,
            patterns: compile(&Text::of(pattern))?,
        });
    }
    Ok(rules)
}

/// The owners of the last rule that matches `path`; empty when none does.
pub(crate) fn owners_for(rules: &[Rule], path: &RelPath) -> Vec<Name> {
    let mut found = Vec::new();
    for rule in rules {
        if rule.matches(path) == Answer::Yes {
            found.clone_from(&rule.owners);
        }
    }
    found
}
