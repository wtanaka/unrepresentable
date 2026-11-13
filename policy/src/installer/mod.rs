//! Generate the release asset `unrepresentable.sh` from `ci/unrepresentable.sh`.
//!
//! `SHA256SUMS` lines look like `<64 hex>  unrepresentable-<target>[.exe]`.
//! Every placeholder must be filled; anything left over is an error.

#[cfg(test)]
mod tests;

/// The definitions of this module.
mod logic;

pub(crate) use logic::{generate, Installer};

#[cfg(test)]
pub(crate) use logic::{parse_sums, TARGETS};
