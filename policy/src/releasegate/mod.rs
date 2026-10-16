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

use crate::exit::Report;
use crate::failure::Failure;
use crate::github::Github;
use crate::json::Json;
use crate::pushskip::{self, Decision};
use crate::types::{Answer, Count, Holds as _, Name, Sha, Text};

/// The slug of the app that owns check runs made by Actions jobs.
const ACTIONS_APP: &str = "github-actions";
/// What the operator does after a refusal.
const ADVICE: &str = "re-run the release workflow after CI is green";
/// The events whose run of the `policy-pr` workflow can carry the checks of a
/// commit of the default branch.
const EVENTS: [&str; 2] = ["push", "workflow_dispatch"];
/// The name of the workflow whose push run carries the checks.
pub(crate) const WORKFLOW_NAME: &str = "policy-pr";
/// The file of that workflow.
pub(crate) const WORKFLOW_PATH: &str = ".github/workflows/policy.yml";
/// The jobs of the push run that must have passed.
pub(crate) const REQUIRED_JOBS: [&str; 9] = [
    "audit-workflows",
    "canaries",
    "clippy-main",
    "clippy-policy",
    "deny",
    "fmt",
    "policytest",
    "self-checks",
    "test",
];
/// The job that decides whether the others are skipped.
pub(crate) const SKIP_JOB: &str = "skip-check";

/// Whether the release may go ahead.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Gate {
    /// Refuse the release.
    Closed,
    /// The commit passed its checks.
    Open,
}

/// What a job's check run says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Standing {
    /// Completed with any conclusion but success or skipped.
    Failed,
    /// Completed with success.
    Passed,
    /// Completed as skipped.
    Skipped,
    /// Not completed.
    Unfinished,
}

/// How a skipped or absent job is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tolerance {
    /// Only success counts, and the job must exist.
    Exact,
    /// Success and skipped count, and the job may be absent.
    SkippedToo,
}

/// The gate's decision and the one-line reason for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Verdict {
    /// Whether to go ahead.
    pub gate: Gate,
    /// Why, on one line.
    pub note: Text,
}

impl Verdict {
    /// Refuse, for `why`; the note tells the operator what to do next.
    pub(crate) fn closed(why: &Text) -> Self {
        Self {
            gate: Gate::Closed,
            note: Text::from_string(format!("release refused: {}; {ADVICE}", why.as_str())),
        }
    }

    /// Go ahead, for `why`.
    fn open(why: &Text) -> Self {
        Self {
            gate: Gate::Open,
            note: Text::from_string(format!("release gate open: {}", why.as_str())),
        }
    }

    /// The report the command prints: clean when open, failed when closed.
    pub(crate) fn report(&self) -> Report {
        match self.gate {
            Gate::Open => Report::saying(&self.note),
            Gate::Closed => Report::failed(vec![self.note.clone()]),
        }
    }
}

/// Whether the check run `run` was made by an Actions job of the suite in
/// `suites` for `commit`.
fn belongs(run: &Json, suites: &[Count], commit: &Sha) -> Answer {
    let app = run
        .dig(&keys!["app", "slug"])
        .and_then(Json::as_text)
        .map(Text::as_str);
    let suite = run
        .dig(&keys!["check_suite", "id"])
        .and_then(Json::as_count);
    (app == Some(ACTIONS_APP)
        && run.text_at(&Name::of("head_sha")).map(Text::as_str) == Some(commit.as_str())
        && suite.is_some_and(|found| suites.contains(&found)))
    .answer()
}

/// Every check run of `commit`, latest attempt of each.
fn check_runs(client: &dyn Github, commit: &Sha) -> Result<Vec<Json>, Failure> {
    client.paged(
        &client.repo_path(&Text::from_string(format!(
            "/commits/{}/check-runs?filter=latest",
            commit.as_str()
        ))),
        Some(&Name::of("check_runs")),
    )
}

/// The verdict, or the failure that kept it from being reached.
fn decide(client: &dyn Github, commit: &Sha, default_branch: &Name) -> Result<Verdict, Failure> {
    let suites = workflow_suites(client, commit, default_branch)?;
    if suites.is_empty() {
        return Ok(Verdict::closed(&Text::from_string(format!(
            "no {WORKFLOW_NAME} workflow run (push or manual) for {}",
            commit.short().as_str()
        ))));
    }
    let all = check_runs(client, commit)?;
    let own: Vec<&Json> = all
        .iter()
        .filter(|run| belongs(run, &suites, commit) == Answer::Yes)
        .collect();
    let Some(first) = REQUIRED_JOBS
        .iter()
        .find_map(|job| problem(&own, &Name::of(job), Tolerance::Exact))
    else {
        return Ok(Verdict::open(&Text::from_string(format!(
            "all {} push jobs of {} succeeded",
            REQUIRED_JOBS.len(),
            commit.short().as_str()
        ))));
    };
    if let Some(why) = problem(&own, &Name::of(SKIP_JOB), Tolerance::Exact) {
        return Ok(Verdict::closed(&Text::from_string(format!(
            "{}; {}",
            first.as_str(),
            why.as_str()
        ))));
    }
    if let Some(why) = REQUIRED_JOBS
        .iter()
        .find_map(|job| problem(&own, &Name::of(job), Tolerance::SkippedToo))
    {
        return Ok(Verdict::closed(&why));
    }
    let skip = pushskip::judge(client, commit);
    Ok(match skip.decision {
        Decision::Skip => Verdict::open(&Text::from_string(format!(
            "the jobs were skipped and {SKIP_JOB} succeeded: {}",
            skip.note.as_str()
        ))),
        Decision::Run => Verdict::closed(&Text::from_string(format!(
            "{}, and it was not skipped: {}",
            first.as_str(),
            skip.note.as_str()
        ))),
    })
}

/// The verdict for the tagged commit `commit`, whose checks must have run on
/// the push to `default_branch`. It opens only on positive evidence and closes
/// on every error.
pub(crate) fn judge(client: &dyn Github, commit: &Sha, default_branch: &Name) -> Verdict {
    match decide(client, commit, default_branch) {
        Ok(found) => found,
        Err(failure) => Verdict::closed(&Text::from_string(format!(
            "could not confirm CI: {}",
            pushskip::one_line(&failure).as_str()
        ))),
    }
}

/// Why the job `job` does not count as passed, or none when it does.
#[must_use]
fn problem(runs: &[&Json], job: &Name, tolerance: Tolerance) -> Option<Text> {
    let named: Vec<&&Json> = runs
        .iter()
        .filter(|run| run.text_at(&Name::of("name")).map(Text::as_str) == Some(job.as_str()))
        .collect();
    if named.is_empty() {
        return match tolerance {
            Tolerance::Exact => Some(Text::from_string(format!(
                "job {} has no check run",
                job.as_str()
            ))),
            Tolerance::SkippedToo => None,
        };
    }
    for run in named {
        let said = match (standing(run), tolerance) {
            (Standing::Passed, Tolerance::Exact | Tolerance::SkippedToo)
            | (Standing::Skipped, Tolerance::SkippedToo) => continue,
            (Standing::Skipped, Tolerance::Exact) => "was skipped",
            (Standing::Unfinished, Tolerance::Exact | Tolerance::SkippedToo) => "has not finished",
            (Standing::Failed, Tolerance::Exact | Tolerance::SkippedToo) => "did not succeed",
        };
        return Some(Text::from_string(format!("job {} {said}", job.as_str())));
    }
    None
}

/// Whether `run` is the `policy-pr` workflow's run for the push of `commit` to
/// `default_branch`, or a manual run of the default branch at `commit`: the workflow file at its exact path (or at the default
/// branch's own ref, which is how the API spells a run from that branch), and
/// a run of the default branch, not of another branch that holds a copy of the
/// commit with a changed workflow.
fn is_push_run(run: &Json, commit: &Sha, default_branch: &Name) -> Answer {
    let text = |key: &str| run.text_at(&Name::of(key)).map(Text::as_str);
    let at_path = text("path").is_some_and(|found| {
        found == WORKFLOW_PATH
            || found
                .strip_prefix(WORKFLOW_PATH)
                .and_then(|rest| rest.strip_prefix("@refs/heads/"))
                == Some(default_branch.as_str())
    });
    (text("name") == Some(WORKFLOW_NAME)
        && at_path
        && text("event").is_some_and(|event| EVENTS.contains(&event))
        && text("head_branch") == Some(default_branch.as_str())
        && text("head_sha") == Some(commit.as_str()))
    .answer()
}

/// What the check run `run` says about its job.
fn standing(run: &Json) -> Standing {
    if run.text_at(&Name::of("status")).map(Text::as_str) != Some("completed") {
        return Standing::Unfinished;
    }
    match run.text_at(&Name::of("conclusion")).map(Text::as_str) {
        Some("success") => Standing::Passed,
        Some("skipped") => Standing::Skipped,
        _ => Standing::Failed,
    }
}

/// The check suites of the `policy-pr` workflow runs (push or manual) of the
/// default branch at `commit`; the event is checked here, not in the query.
fn workflow_suites(
    client: &dyn Github,
    commit: &Sha,
    default_branch: &Name,
) -> Result<Vec<Count>, Failure> {
    let runs = client.paged(
        &client.repo_path(&Text::from_string(format!(
            "/actions/runs?head_sha={}",
            commit.as_str()
        ))),
        Some(&Name::of("workflow_runs")),
    )?;
    Ok(runs
        .iter()
        .filter(|run| is_push_run(run, commit, default_branch) == Answer::Yes)
        .filter_map(|run| run.count_at(&Name::of("check_suite_id")))
        .collect())
}
