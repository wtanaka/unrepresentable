//! TOML reading: the `toml` crate parses (TOML 1.1), this module turns its
//! result into the section-and-entry shape the rest of the tool reads.
//!
//! The crate returns a merged tree with byte spans and no record of how a
//! table was written. The tool needs the written form: which `[table]` or
//! `[[array]]` header an entry sits under, the line of each header and entry,
//! and dotted keys as written. This module recovers that from the spans: a
//! table whose span starts at `[` was written as a header, `{` as an inline
//! table, and anything else was made by a dotted key or an enclosing header.
//! Floats and dates are kept as their text; the tool never computes with them.
//! So is an integer outside the 64-bit range: it is not an error for the file
//! (a clippy knob may be a `u64`), only for a reader that needs an integer.
//! Any failure names file, line and column.

#[cfg(test)]
mod tests;

/// The definitions of this module.
mod logic;

pub(crate) use logic::{parse, Doc, Entry, Section, SectionKind, Value};

#[cfg(test)]
pub(crate) use logic::KeyPath;
