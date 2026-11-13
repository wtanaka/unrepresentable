//! Whether a push to the default branch can skip its checks.
//!
//! A pull request's head commit already went through the `policy` check. When
//! the pushed commit has the same git tree as the head of a pull request it
//! belongs to, and the App's `policy` check run for that head succeeded, the
//! same files were already checked and the expensive jobs can be skipped. Any
//! doubt means the checks run: a missing, failed or unfinished check run, a
//! different tree, no pull request, or an API error. The decision never skips
//! on an error.

#[cfg(test)]
mod tests;

/// The definitions of this module.
mod logic;

pub(crate) use logic::{judge, one_line, Decision, Judged};
