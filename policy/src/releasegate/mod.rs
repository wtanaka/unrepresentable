//! Whether a release tag may be built: the tagged commit must have passed the
//! ordinary checks. The gate refuses unless the checks are proven: a missing,
//! unfinished, failed, cancelled or neutral job, a job from another app or
//! workflow, or any API error makes it refuse.
//!
//! The commit passed when either of two things holds. First, the nine jobs
//! that the `policy-pr` workflow runs for the push (`REQUIRED_JOBS`) each have
//! a check run, owned by the Actions app and belonging to that push's workflow
//! run, that completed with success. Second, those jobs were skipped because
//! the same git tree already passed as a pull request head: the push's
//! `skip-check` job succeeded, no other job failed, and the push-skip decision
//! for the commit is `Skip` (that logic lives in [`crate::pushskip`]).
//!
//! The workflow run must be a push to the default branch itself, or a manual
//! run (`workflow_dispatch`) of the default branch for that commit, from the
//! workflow file at its exact path (or at `@refs/heads/<default branch>`),
//! not a run of a modified copy on another branch. The manual run is accepted
//! because a push run can be dropped (a newer push replaces a pending run of
//! the concurrency group), after which no push run of that commit would ever
//! exist and the release could never go ahead. A manual run runs the same jobs
//! (`skip-check` is skipped for it, so it counts only through the first of the
//! two ways above: all nine jobs succeeded). Only someone with write access
//! can start one, and it runs the workflow file of the default branch.

#[cfg(test)]
mod tests;

/// The definitions of this module.
mod logic;

pub(crate) use logic::{judge, Verdict};

#[cfg(test)]
pub(crate) use logic::{Gate, REQUIRED_JOBS, SKIP_JOB, WORKFLOW_NAME, WORKFLOW_PATH};
