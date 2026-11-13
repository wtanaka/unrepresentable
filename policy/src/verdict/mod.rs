//! Verdict computation and the verdict block appended to merge commits.

#[cfg(test)]
mod tests;

/// The definitions of this module.
mod logic;

pub(crate) use logic::{
    compute, merge_message, parse_block, Approvable, BlockFacts, PullNumber, Verdict,
};

#[cfg(test)]
pub(crate) use logic::{verdict_block, Decision, ParsedBlock};
