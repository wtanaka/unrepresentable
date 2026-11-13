//! Strict line format for per-job result artifacts.
//!
//! Artifacts produced by jobs that touch head code are untrusted input. They
//! are parsed with a size limit and a fixed grammar; anything else is a parse
//! failure, which the policy job turns into a blocking (non-approvable)
//! failure.
//!
//! ```text
//! unrepresentable-policy-result v1
//! job <name>
//! head <40 hex>
//! base <40 hex>
//! R <CODE> <b64 evidence> <b64 file or -> <line>
//! N <key> <non-negative int>
//! end
//! ```

#[cfg(test)]
mod tests;

/// The definitions of this module.
mod logic;

pub(crate) use logic::{emit, parse, MAX_BYTES};

#[cfg(test)]
pub(crate) use logic::Parsed;
