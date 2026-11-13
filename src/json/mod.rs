//! JSON reading for cargo's `--message-format=json` lines. `serde_json` parses
//! into its untyped value; this module copies that into [`Json`], which keeps
//! numbers as text and objects as sorted key and value pairs. The text of a
//! number is the library's: an integer that fits `i64` or `u64` keeps its
//! digits, any other number is an `f64` written back (`1.0e2` as `100.0`, `-0`
//! as `-0.0`). The tool reads counts and strings from cargo's messages and never
//! forwards a number's text.

#[cfg(test)]
mod tests;

/// The definitions of this module.
mod logic;

pub(crate) use logic::{parse, Flag, Json};
