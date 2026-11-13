//! Approval evaluation. Pure functions; the callers fetch everything from the
//! GitHub API for one exact head commit.
//!
//! Conditions (all):
//!  1. the reviewer's latest effective review is APPROVED on commit == head;
//!  2. the reviewer owns (per the BASE-branch CODEOWNERS) the path of every
//!     path-based reason; non-path reasons need an owner of `policy/**`;
//!  3. the reviewer is not the PR author, did not author or commit any commit
//!     of the PR (a code owner who pushes to someone else's PR cannot approve
//!     the result), and is not a bot;
//!  4. every commit of the PR names a login for its author and for its
//!     committer, and the commits listed are all the PR has: otherwise the
//!     contributors cannot be known and nothing is approvable;
//!  5. every input job of the latest policy run for that head succeeded.
//!
//! `/approve` comments are never read. Team owners are never resolved: a rule
//! that names only teams approves nothing.

#[cfg(test)]
mod tests;

/// The definitions of this module.
mod logic;

pub(crate) use logic::{
    evaluate, Attribution, Contributors, Evaluation, Inputs, Outcome, Problem, Review,
};

#[cfg(test)]
pub(crate) use logic::ReviewState;
