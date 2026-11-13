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

/// Pattern compilation and matching.
mod logic;

pub(crate) use logic::{Brackets, Glob};
