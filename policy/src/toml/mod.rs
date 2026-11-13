//! TOML reading for manifests and lock files. The `toml` crate parses (TOML
//! 1.1); this module copies the result into a tree whose tables list their
//! entries in source order, each with the line of its key. How a table was
//! written (header, inline or dotted keys) does not matter to the policy, so
//! it is not kept. Floats, dates and integers are not needed and read as
//! [`Value::Other`].

#[cfg(test)]
mod tests;

/// The definitions of this module.
mod logic;

pub(crate) use logic::{parse, Table, Value};
