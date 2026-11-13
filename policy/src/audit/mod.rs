//! Settings audit: live repository settings and recent merge commits against
//! `policy/settings.json`. Read-only token (`AUDIT_TOKEN`). Prints one drift
//! per line (our own words; no PR text) and exits 1 on drift.

pub(crate) mod live;

#[cfg(test)]
mod tests;

/// The definitions of this module.
mod logic;

pub(crate) use logic::{check_merge_commits, compare, exempt_commit, Audited, MergeCommit};
