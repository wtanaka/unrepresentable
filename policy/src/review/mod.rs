//! Re-evaluate human approval and update the `policy` check run.
//!
//! Used by `policy-review.yml` (a `workflow_run` of `review-signal`) and
//! `policy-sweep.yml`. Only a PR number (or none: all open PRs) is taken from
//! the triggering event. Everything else is read from the API for one exact
//! head commit.
//!
//! A stored verdict is only turned into success when it was made against the
//! base branch as it is now and by the policy code that runs now. If the base
//! branch moved or the policy changed since the verdict, the evaluation is
//! stale and the check stays as it is: the policy workflow has to run again
//! (a new push to the pull request, or closing and reopening it).

#[cfg(test)]
mod tests;

/// The definitions of this module.
mod logic;

pub(crate) use logic::{open_pull_numbers, open_pulls_with_head, sweep, Running};

#[cfg(test)]
pub(crate) use logic::{evaluate_pr, ReviewStatus};
