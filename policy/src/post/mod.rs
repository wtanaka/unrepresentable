//! The `policy` job: base code only, App credentials, no head code ever.
//!
//! Inputs (validated): `PR_NUMBER`, `HEAD_SHA`, `BASE_SHA`, `GH_TOKEN` (App
//! installation token), `READ_TOKEN`, `GITHUB_REPOSITORY`, `GITHUB_RUN_ID`,
//! `GITHUB_RUN_ATTEMPT`, `RESULT_<JOB>`, `DEFAULT_BRANCH` (from repository
//! settings, not the PR) and `RUNNER_TEMP`.

#[cfg(test)]
mod tests;

/// The definitions of this module.
mod logic;

pub(crate) use logic::{
    decide, merge, policy_commit, validated_env, verdict_file, Clients, DecideInput, MergeInput,
    MergeOutcome, RunIds,
};

#[cfg(test)]
pub(crate) use logic::{gather, settings_audit_state, AuditState, PolicyEnv};
