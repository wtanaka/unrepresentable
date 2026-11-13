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

/// The definitions of this module.
mod logic;

pub(crate) use logic::{owners_for, parse, Rule};
