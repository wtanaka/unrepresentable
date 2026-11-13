//! JSON reading and writing. `serde_json` parses into its untyped value, which
//! is copied into [`Json`]; objects then hold their members sorted by key,
//! because the value type is backed by a sorted map. The writer in this module
//! is compact and sorts keys itself, so that the verdict text a check run
//! carries is stable and does not depend on the library's number formatting.
//!
//! Numbers are normalised by the library, not kept as written: an integer that
//! fits `i64` or `u64` keeps its digits, and any other number becomes an `f64`
//! and is written back in that form (`1.0e2` as `100.0`, `-0` as `-0.0`). The
//! `arbitrary_precision` feature does not give the written text back either
//! (`1.0e2` becomes `1.0e+2`, `-0` becomes `0`), so it is not enabled. Nothing
//! here depends on the bytes of an untrusted number that is parsed and written
//! again: such values are compared or shown after parsing, and what this crate
//! writes itself uses counts and strings.
#[cfg(test)]
mod tests;

/// The definitions of this module.
#[macro_use]
mod logic;

pub(crate) use logic::{items_of, members_of, parse, Flag, Json};
