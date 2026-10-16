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

use crate::approval::{
    self, Attribution, Contributors, Evaluation, Inputs, Outcome, Problem, Review,
};
use crate::checkrun::{self, INPUT_JOBS};
use crate::codeowners;
use crate::data::{Reviewed, VerdictData};
use crate::failure::Failure;
use crate::github::Github;
use crate::json::Json;
use crate::post::Clients;
use crate::pushskip;
use crate::report::{self, Approval, Conclusion, Info};
use crate::types::{Answer, Count, Message, Name, Sha, Text};
use crate::verdict::{Approvable, PullNumber, Verdict};

/// The path of the policy workflow.
const WORKFLOW_PATH: &str = ".github/workflows/policy.yml";

/// What re-evaluating one pull request did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ReviewStatus {
    /// The head has no check run from the application.
    NoPolicyCheck,
    /// The check run holds no verdict data.
    NoVerdictData,
    /// The change needs no approval.
    NothingToApprove,
    /// The evaluation was skipped.
    Skipped,
    /// The verdict was made against another base or by another policy than the
    /// ones running now; the policy workflow has to run again.
    Stale,
    /// The check run already had this conclusion.
    Unchanged(Conclusion),
    /// The check run was updated to this conclusion.
    Updated(Conclusion),
}

impl ReviewStatus {
    /// The line the job prints after `PR #n: `.
    pub(crate) fn describe(&self) -> Text {
        match self {
            Self::Skipped => Text::of("skipped"),
            Self::Stale => Text::of(
                "stale: the base branch or the policy changed since the verdict; the policy workflow must run again",
            ),
            Self::NoPolicyCheck => Text::of("no policy check yet"),
            Self::NoVerdictData => Text::of("no verdict data for this head"),
            Self::NothingToApprove => Text::of("nothing to approve"),
            Self::Unchanged(conclusion) => {
                Text::from_string(format!("unchanged ({})", conclusion.name().as_str()))
            }
            Self::Updated(conclusion) => {
                Text::from_string(format!("updated to {}", conclusion.name().as_str()))
            }
        }
    }
}

/// What an evaluation decided.
struct Decision {
    /// The reviewers who approved.
    approvers: Vec<Name>,
    /// Whether the commits could be attributed; when not, the check run says so.
    attribution: Attribution,
    /// Whether the change is approved.
    state: Approval,
}

/// The pull request fields re-evaluation needs.
struct PullFacts {
    /// The login of the author.
    author: Name,
    /// How many commits the pull request itself says it has.
    commits: Option<Count>,
    /// The default branch of the repository.
    default_branch: Name,
    /// The head commit.
    head: Sha,
}

/// The base branch and the policy code a review runs with.
pub(crate) struct Running {
    /// The default branch of the repository.
    pub default_branch: Name,
    /// The commit of the policy code that runs now.
    pub policy_commit: Sha,
}

/// The App's check run for a head and the verdict data stored in it.
struct Stored {
    /// The stored verdict data.
    data: VerdictData,
    /// The check run as the API reports it.
    run: Json,
}

/// What re-evaluating a list of pull requests did.
pub(crate) struct Sweep {
    /// Whether any pull request could not be evaluated.
    pub failed: Answer,
    /// One line per pull request, in the order given.
    pub lines: Vec<Text>,
}

/// The stored verdict, or the status that says why there is none.
type Found = Result<Stored, ReviewStatus>;

/// Condition 4: every input job of the `policy.yml` run that produced the
/// verdict completed successfully (checked again through the jobs API).
fn inputs_succeeded(
    reader: &dyn Github,
    data: &VerdictData,
    default_branch: &Name,
) -> Result<Inputs, Failure> {
    let run = reader.get(&reader.repo_path(&Text::from_string(format!(
        "/actions/runs/{}",
        data.run_id.get()
    ))))?;
    let location = match run.text_at(&Name::of("path")) {
        Some(found) => found.as_str(),
        None => "",
    };
    let (path, reference) = match location.split_once('@') {
        Some((before, after)) => (before, after),
        None => (location, ""),
    };
    let branch_ref = format!("refs/heads/{}", default_branch.as_str());
    let from_policy = path == WORKFLOW_PATH
        && (reference.is_empty() || reference == branch_ref)
        && run.text_at(&Name::of("status")).map(Text::as_str) == Some("completed");
    if !from_policy {
        return Ok(Inputs::SomeFailed);
    }
    let jobs = reader.get(&reader.repo_path(&Text::from_string(format!(
        "/actions/runs/{}/attempts/{}/jobs?per_page=100",
        data.run_id.get(),
        data.attempt.get()
    ))))?;
    let listed = match jobs.get(&Name::of("jobs")).and_then(Json::as_array) {
        Some(found) => found,
        None => &[],
    };
    let succeeded = |job_name: &Text| {
        listed.iter().any(|item| {
            item.text_at(&Name::of("name")) == Some(job_name)
                && item.text_at(&Name::of("conclusion")).map(Text::as_str) == Some("success")
        })
    };
    let all_jobs = INPUT_JOBS.iter().all(|job| succeeded(&job.name()));
    let recorded = INPUT_JOBS.iter().all(|job| {
        data.inputs
            .iter()
            .any(|(found, result)| found == job && result.as_str() == "success")
    });
    Ok(if all_jobs && recorded {
        Inputs::AllSucceeded
    } else {
        Inputs::SomeFailed
    })
}

/// The facts of an open pull request into `default_branch`, or `None`.
#[must_use]
fn open_pull(pull: &Json, default_branch: &Name) -> Option<PullFacts> {
    let open = pull.text_at(&Name::of("state")).map(Text::as_str) == Some("open");
    let base = pull
        .get(&Name::of("base"))?
        .text_at(&Name::of("ref"))?
        .as_str()
        == default_branch.as_str();
    if !(open && base) {
        return None;
    }
    let head = Sha::parse(pull.get(&Name::of("head"))?.text_at(&Name::of("sha"))?)?;
    let author = Name::of(
        pull.get(&Name::of("user"))?
            .text_at(&Name::of("login"))?
            .as_str(),
    );
    Some(PullFacts {
        author,
        commits: pull.count_at(&Name::of("commits")),
        default_branch: default_branch.clone(),
        head,
    })
}

/// The stored verdict data of the App's check run for `head`, if it is for
/// this pull request and head.
fn stored_verdict(app: &dyn Github, pull: PullNumber, head: &Sha) -> Result<Found, Failure> {
    let runs = checkrun::app_check_runs(app, head)?;
    let Some(run) = runs.first() else {
        return Ok(Err(ReviewStatus::NoPolicyCheck));
    };
    let Some(text) = run
        .get(&Name::of("output"))
        .and_then(|output| output.text_at(&Name::of("text")))
    else {
        return Ok(Err(ReviewStatus::NoVerdictData));
    };
    match checkrun::decode_data(text).and_then(|found| VerdictData::from_json(&found)) {
        Ok(data) if data.commits.head == *head && data.pull == pull => Ok(Ok(Stored {
            data,
            run: run.clone(),
        })),
        Ok(_) | Err(_) => Ok(Err(ReviewStatus::NoVerdictData)),
    }
}

/// The reviews of a pull request, as the API lists them.
fn reviews_of(client: &dyn Github, number: PullNumber) -> Result<Vec<Review>, Failure> {
    let items = client.paged(
        &client.repo_path(&Text::from_string(format!(
            "/pulls/{}/reviews",
            number.count().get()
        ))),
        None,
    )?;
    Ok(items.iter().filter_map(Review::from_json).collect())
}

/// The logins that authored or committed a commit of the pull request, and
/// whether every commit names them (see `Contributors::from_commits`).
fn contributors_of(
    client: &dyn Github,
    number: PullNumber,
    expected: Option<Count>,
) -> Result<Contributors, Failure> {
    let items = client.paged(
        &client.repo_path(&Text::from_string(format!(
            "/pulls/{}/commits",
            number.count().get()
        ))),
        None,
    )?;
    Ok(Contributors::from_commits(&items, expected))
}

/// A stale verdict never turns a check into success. A check that already says
/// success is turned back to failure, so a base that moved or a policy that
/// changed after an approval cannot leave an old success standing.
fn stale(
    clients: &Clients<'_>,
    facts: &PullFacts,
    stored: &Stored,
) -> Result<ReviewStatus, Failure> {
    let current = stored
        .run
        .text_at(&Name::of("conclusion"))
        .and_then(Conclusion::parse);
    if current == Some(Conclusion::Success) {
        return publish(
            clients,
            facts,
            stored,
            Decision {
                approvers: Vec::new(),
                attribution: Attribution::Complete,
                state: Approval::NotApproved,
            },
        );
    }
    Ok(ReviewStatus::Stale)
}

/// The commit the default branch points at now.
fn current_base(client: &dyn Github, default_branch: &Name) -> Result<Sha, Failure> {
    let branch = client.get(&client.repo_path(&Text::from_string(format!(
        "/branches/{}",
        default_branch.as_str()
    ))))?;
    branch
        .dig(&keys!["commit", "sha"])
        .and_then(Json::as_text)
        .and_then(Sha::parse)
        .ok_or_else(|| Failure::api(&Message::of("the default branch has no commit")))
}

/// Evaluates approval of a pull request whose verdict is stored, and updates
/// the check run when the conclusion changes.
fn reevaluate(
    clients: &Clients<'_>,
    number: PullNumber,
    facts: &PullFacts,
    stored: &Stored,
) -> Result<ReviewStatus, Failure> {
    let data = &stored.data;
    let default_branch = &facts.default_branch;
    let owners_text = clients
        .app
        .get_text(&clients.app.repo_path(&Text::from_string(format!(
            "/contents/.github/CODEOWNERS?ref={}",
            default_branch.as_str()
        ))))?;
    let rules = codeowners::parse(&owners_text)?;
    let reviews = reviews_of(clients.app, number)?;
    let contributors = contributors_of(clients.app, number, facts.commits)?;
    let outcome = approval::evaluate(&Evaluation {
        author: &facts.author,
        contributors: &contributors,
        head: &facts.head,
        inputs: inputs_succeeded(clients.reader, data, default_branch)?,
        reasons: &data.reasons,
        reviews: &reviews,
        rules: &rules,
    });
    let decision = match outcome {
        Outcome::Approved { approvers } => Decision {
            approvers,
            attribution: Attribution::Complete,
            state: Approval::Approved,
        },
        Outcome::NotApproved { problems } => Decision {
            approvers: Vec::new(),
            attribution: if problems.contains(&Problem::CommitsNotAttributable) {
                Attribution::Incomplete
            } else {
                Attribution::Complete
            },
            state: Approval::NotApproved,
        },
    };
    publish(clients, facts, stored, decision)
}

/// Updates the check run to what the decision gives, when that is not what it
/// already says.
fn publish(
    clients: &Clients<'_>,
    facts: &PullFacts,
    stored: &Stored,
    decision: Decision,
) -> Result<ReviewStatus, Failure> {
    let Decision {
        approvers,
        attribution,
        state: approval_state,
    } = decision;
    let data = &stored.data;
    let info = Info {
        approvers: approvers.clone(),
        blockers: data.blockers.clone(),
        commits: data.commits.clone(),
        counts: data.counts.clone(),
        policy_commit: data.policy_commit.clone(),
        reasons: data.reasons.clone(),
        verdict: Verdict::NeedsHuman,
    };
    let built = report::build_check(&info, approval_state);
    let check = match attribution {
        Attribution::Complete => built,
        Attribution::Incomplete => report::not_attributable(built),
    };
    let wanted = check.conclusion;
    let current = stored
        .run
        .text_at(&Name::of("conclusion"))
        .and_then(Conclusion::parse);
    let said = stored
        .run
        .get(&Name::of("output"))
        .and_then(|output| output.text_at(&Name::of("summary")))
        .is_some_and(|summary| summary == &check.summary);
    let explained = match attribution {
        Attribution::Complete => true,
        Attribution::Incomplete => said,
    };
    if current == Some(wanted) && explained {
        return Ok(ReviewStatus::Unchanged(wanted));
    }
    let updated = VerdictData {
        reviewed: Some(Reviewed {
            approval: approval_state,
            approvers,
        }),
        ..data.clone()
    };
    checkrun::upsert(
        clients.app,
        &facts.head,
        &checkrun::payload(&check, &updated),
    )?;
    Ok(ReviewStatus::Updated(wanted))
}

/// Re-evaluates approval of one pull request and updates the check run.
/// `policy_commit` is the commit of the policy code that runs now.
pub(crate) fn evaluate_pr(
    clients: &Clients<'_>,
    number: PullNumber,
    running: &Running,
) -> Result<ReviewStatus, Failure> {
    let default_branch = &running.default_branch;
    let pull = clients
        .app
        .get(&clients.app.repo_path(&Text::from_string(format!(
            "/pulls/{}",
            number.count().get()
        ))))?;
    let Some(facts) = open_pull(&pull, default_branch) else {
        return Ok(ReviewStatus::Skipped);
    };
    let stored = match stored_verdict(clients.app, number, &facts.head)? {
        Ok(found) => found,
        Err(status) => return Ok(status),
    };
    if stored.data.verdict != Verdict::NeedsHuman || stored.data.approvable != Approvable::Yes {
        return Ok(ReviewStatus::NothingToApprove);
    }
    let base_now = current_base(clients.reader, default_branch)?;
    if stored.data.commits.base != base_now || stored.data.policy_commit != running.policy_commit {
        return stale(clients, &facts, &stored);
    }
    reevaluate(clients, number, &facts, &stored)
}

/// The open pull requests into `default_branch`, all of them: the listing
/// fails instead of stopping early.
pub(crate) fn open_pull_numbers(
    client: &dyn Github,
    default_branch: &Name,
) -> Result<Vec<PullNumber>, Failure> {
    let path = client.repo_path(&Text::from_string(format!(
        "/pulls?state=open&base={}",
        default_branch.as_str()
    )));
    let numbers = client
        .paged(&path, None)?
        .iter()
        .filter_map(|item| item.count_at(&Name::of("number")))
        .filter_map(|count| PullNumber::parse(&count.text()))
        .collect();
    Ok(numbers)
}

/// Re-evaluates every pull request in `numbers`. A pull request whose
/// evaluation fails (an API error, say) is reported and does not stop the
/// others: the rest still get their re-evaluation, including the downgrade of
/// a success that is stale. The sweep as a whole is `failed` when any one was.
pub(crate) fn sweep(clients: &Clients<'_>, numbers: &[PullNumber], running: &Running) -> Sweep {
    let mut lines = Vec::new();
    let mut failed = Answer::No;
    for number in numbers {
        let said = match evaluate_pr(clients, *number, running) {
            Ok(status) => status.describe(),
            Err(failure) => {
                failed = Answer::Yes;
                Text::from_string(format!(
                    "could not be re-evaluated: {}",
                    pushskip::one_line(&failure).as_str()
                ))
            }
        };
        lines.push(Text::from_string(format!(
            "PR #{}: {}",
            number.count().get(),
            said.as_str()
        )));
    }
    Sweep { failed, lines }
}

/// The open pull requests into `default_branch` whose head is `head`, which is
/// how a review of a fork's pull request is found: the event that starts the
/// review names no pull request for a fork. Empty when there are none.
pub(crate) fn open_pulls_with_head(
    client: &dyn Github,
    default_branch: &Name,
    head: &Sha,
) -> Result<Vec<PullNumber>, Failure> {
    let path = client.repo_path(&Text::from_string(format!(
        "/commits/{}/pulls",
        head.as_str()
    )));
    Ok(client
        .paged(&path, None)?
        .iter()
        .filter(|item| open_pull(item, default_branch).is_some_and(|facts| facts.head == *head))
        .filter_map(|item| item.count_at(&Name::of("number")))
        .filter_map(|count| PullNumber::parse(&count.text()))
        .collect())
}
