//! The `policy` job: base code only, App credentials, no head code ever.
//!
//! Inputs (validated): `PR_NUMBER`, `HEAD_SHA`, `BASE_SHA`, `GH_TOKEN` (App
//! installation token), `READ_TOKEN`, `GITHUB_REPOSITORY`, `GITHUB_RUN_ID`,
//! `GITHUB_RUN_ATTEMPT`, `RESULT_<JOB>`, `DEFAULT_BRANCH` (from repository
//! settings, not the PR) and `RUNNER_TEMP`.

#[cfg(test)]
mod tests;

use crate::artifacts::{self, MAX_BYTES};
use crate::checkrun;
use crate::data::{InputResults, VerdictData};
use crate::failure::Failure;
use crate::github::{Github, Method};
use crate::io::{self, Vars};
use crate::json::{Flag, Json};
use crate::process::{self, Args};
use crate::reason::{Blocker, BlockerCode, Commits, Counts, Job, Reason};
use crate::report::{self, Approval, Info};
use crate::timestamp;
use crate::types::{Answer, Count, Holds as _, Message, Name, Sha, Text};
use crate::verdict::{self, BlockFacts, PullNumber, Verdict};
use std::path::{Path, PathBuf};

/// The oldest settings audit, in seconds, that is still current.
const AUDIT_MAX_AGE_SECONDS: usize = 86_400;
/// The name of the verdict file.
const VERDICT_FILE: &str = "verdict.json";

/// The pull request and commits an event names, validated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PolicyEnv {
    /// The base and head commits.
    pub commits: Commits,
    /// The pull request number.
    pub pull: PullNumber,
}

/// What the input jobs reported.
#[derive(Clone, Debug)]
pub(crate) struct Gathered {
    /// The blockers that stop an approval.
    pub blockers: Vec<Blocker>,
    /// The counts of findings by name.
    pub counts: Counts,
    /// The results of the input jobs.
    pub inputs: InputResults,
    /// The reasons the change needs a human.
    pub reasons: Vec<Reason>,
}

/// Whether the settings audit is recent and clean.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AuditState {
    /// The audit is recent enough.
    Fresh,
    /// The audit is not usable, for this reason.
    Problem(Text),
}

/// Whether the current head already has a comment from the App.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CommentState {
    /// The result exists.
    Exists,
    /// The result is missing.
    Missing,
}

/// The two API clients of the job: the App (writes) and the job token
/// (`actions: read`).
pub(crate) struct Clients<'client> {
    /// The client that posts as the application.
    pub app: &'client dyn Github,
    /// The client that reads as a user.
    pub reader: &'client dyn Github,
}

/// What `decide` needs besides the clients.
pub(crate) struct DecideInput {
    /// The default branch of the repository.
    pub default_branch: Name,
    /// The policy environment.
    pub env: PolicyEnv,
    /// The current time, in seconds.
    pub now: Count,
    /// The commit of the policy that runs.
    pub policy_commit: Sha,
    /// The directory of the job results.
    pub results_dir: PathBuf,
    /// The identifiers of the workflow run.
    pub run: RunIds,
}

/// Which workflow run produced the decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RunIds {
    /// The attempt number of the run.
    pub attempt: Count,
    /// The identifier of the run.
    pub run_id: Count,
}

/// What `merge` did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MergeOutcome {
    /// The pull request was merged.
    Merged,
    /// The change does not merge automatically.
    NotAuto,
    /// The pull request changed since it was evaluated.
    PullChanged,
    /// The merge was refused, for this reason.
    Refused(Text),
    /// The pull request targets another branch.
    WrongBase,
}

impl MergeOutcome {
    /// The line the job prints.
    pub(crate) fn describe(&self) -> Text {
        match self {
            Self::NotAuto => Text::of("not merging: verdict is not auto"),
            Self::PullChanged => Text::of("not merging: PR is closed, draft, or has a newer head"),
            Self::WrongBase => Text::of("not merging: PR does not target the default branch"),
            Self::Merged => Text::of("merged"),
            Self::Refused(why) => Text::from_string(format!("merge refused: {}", why.as_str())),
        }
    }
}

/// What `merge` needs.
pub(crate) struct MergeInput {
    /// The verdict data to act on.
    pub data: VerdictData,
    /// The default branch of the repository.
    pub default_branch: Name,
    /// The policy environment.
    pub env: PolicyEnv,
}

/// Reads and validates `PR_NUMBER`, `HEAD_SHA` and `BASE_SHA`.
pub(crate) fn validated_env(vars: &Vars) -> Result<PolicyEnv, Failure> {
    let pull = PullNumber::parse(&vars.text(&Name::of("PR_NUMBER")));
    let head = Sha::parse(&vars.text(&Name::of("HEAD_SHA")));
    let base = Sha::parse(&vars.text(&Name::of("BASE_SHA")));
    match (pull, head, base) {
        (Some(found_pull), Some(found_head), Some(found_base)) => Ok(PolicyEnv {
            commits: Commits {
                base: found_base,
                head: found_head,
            },
            pull: found_pull,
        }),
        (None, _, _) | (_, None, _) | (_, _, None) => Err(Failure::refused(&Message::of(
            "refusing: PR_NUMBER/HEAD_SHA/BASE_SHA failed validation",
        ))),
    }
}

/// The path of the result file of `job`.
fn result_file(results_dir: &Path, job: Job) -> PathBuf {
    results_dir
        .join(format!("result-{}", job.name().as_str()))
        .join("result.txt")
}

/// Reads every job's artifact from `results_dir`; a failed job, a missing
/// artifact or an artifact that does not parse is a blocker.
pub(crate) fn gather(
    results_dir: &Path,
    vars: &Vars,
    commits: &Commits,
) -> Result<Gathered, Failure> {
    let mut gathered = Gathered {
        blockers: Vec::new(),
        counts: Counts::empty(),
        inputs: Vec::new(),
        reasons: Vec::new(),
    };
    for job in Job::ALL {
        let result = vars.text(&job.result_variable());
        gathered.inputs.push((job, result.clone()));
        if result.as_str() != "success" {
            let shown = if result.as_str().is_empty() {
                "missing"
            } else {
                result.as_str()
            };
            gathered.blockers.push(Blocker {
                code: BlockerCode::InputJobFailed,
                detail: Text::from_string(format!(
                    "job {} finished as {shown}",
                    job.name().as_str()
                )),
            });
            continue;
        }
        let limit = Count::new(MAX_BYTES).next()?;
        let parsed = io::read_bytes_limited(&result_file(results_dir, job), limit)
            .and_then(|data| artifacts::parse(&data, job, commits));
        match parsed {
            Ok(found) => {
                gathered.reasons.extend(found.reasons);
                if job == Job::Classify {
                    gathered.counts = found.counts;
                }
            }
            Err(failure) => gathered.blockers.push(Blocker {
                code: BlockerCode::ResultUnparsable,
                detail: Text::from_string(format!(
                    "job {} artifact rejected: {}",
                    job.name().as_str(),
                    failure.text().as_str()
                )),
            }),
        }
    }
    Ok(gathered)
}

/// The state of the latest completed `settings-audit` run on the default
/// branch, as of `now` (seconds since the epoch).
pub(crate) fn settings_audit_state(
    reader: &dyn Github,
    default_branch: &Name,
    now: Count,
) -> Result<AuditState, Failure> {
    let path = reader.repo_path(&Text::from_string(format!(
        "/actions/workflows/settings-audit.yml/runs?branch={}&status=completed&per_page=5",
        default_branch.as_str()
    )));
    let reply = reader.get(&path)?;
    let runs = match reply
        .get(&Name::of("workflow_runs"))
        .and_then(Json::as_array)
    {
        Some(found) => found,
        None => &[],
    };
    let Some(latest) = runs.first() else {
        return Ok(AuditState::Problem(Text::of(
            "no completed settings-audit run",
        )));
    };
    if latest.text_at(&Name::of("conclusion")).map(Text::as_str) != Some("success") {
        return Ok(AuditState::Problem(Text::of(
            "latest settings-audit run did not succeed",
        )));
    }
    let updated = match latest.text_at(&Name::of("updated_at")) {
        Some(found) => timestamp::parse_utc(found)?,
        None => {
            return Err(Failure::api(&Message::of(
                "settings-audit run has no updated_at",
            )))
        }
    };
    let expires = updated.plus(Count::new(AUDIT_MAX_AGE_SECONDS))?;
    if now.get() > expires.get() {
        return Ok(AuditState::Problem(Text::of(
            "latest settings-audit run is older than 24 hours",
        )));
    }
    Ok(AuditState::Fresh)
}

/// The marker line that identifies a comment for `head`.
fn marker(head: &Sha) -> Text {
    Text::from_string(format!(
        "<!-- unrepresentable-policy:head:{} -->",
        head.as_str()
    ))
}

/// Minimises the App's comments for older heads; reports whether one for
/// `head` exists already.
pub(crate) fn minimize_old_comments(
    client: &dyn Github,
    pull: PullNumber,
    head: &Sha,
) -> Result<CommentState, Failure> {
    let path = client.repo_path(&Text::from_string(format!(
        "/issues/{}/comments",
        pull.count().get()
    )));
    let wanted = marker(head);
    let bot_login = format!("{}[bot]", checkrun::APP_SLUG);
    let mut state = CommentState::Missing;
    for comment in client.paged(&path, None)? {
        let author = comment.get(&Name::of("user"));
        let by_app = author
            .and_then(|user| user.text_at(&Name::of("login")))
            .map(Text::as_str)
            == Some(bot_login.as_str())
            && author
                .and_then(|user| user.text_at(&Name::of("type")))
                .map(Text::as_str)
                == Some("Bot");
        if !by_app {
            continue;
        }
        let body = match comment.text_at(&Name::of("body")) {
            Some(found) => found.as_str(),
            None => "",
        };
        if body.contains(wanted.as_str()) {
            state = CommentState::Exists;
            continue;
        }
        let Some(node) = comment.text_at(&Name::of("node_id")) else {
            continue;
        };
        client
            .graphql(
                &Text::of("mutation($id:ID!){minimizeComment(input:{subjectId:$id,classifier:OUTDATED}){clientMutationId}}"),
                &Json::object_of(vec![(Name::of("id"), Json::text_of(node))]),
            )
            .map(|_data| ())?;
    }
    Ok(state)
}

/// Posts the comment for the current head.
fn post_comment(clients: &Clients<'_>, input: &DecideInput, info: &Info) -> Result<(), Failure> {
    let body = format!(
        "{}\n{}",
        marker(&input.env.commits.head).as_str(),
        report::build_comment(info).as_str()
    );
    clients
        .app
        .send(
            Method::Post,
            &clients.app.repo_path(&Text::from_string(format!(
                "/issues/{}/comments",
                input.env.pull.count().get()
            ))),
            &Json::object_of(vec![(
                Name::of("body"),
                Json::text_of(&Text::from_string(body)),
            )]),
        )
        .map(|_reply| ())
}

/// Decides, posts the `policy` check run and (once per head) the comment, and
/// returns the verdict data.
pub(crate) fn decide(
    clients: &Clients<'_>,
    vars: &Vars,
    input: &DecideInput,
) -> Result<VerdictData, Failure> {
    let commits = &input.env.commits;
    let mut gathered = gather(&input.results_dir, vars, commits)?;
    match settings_audit_state(clients.reader, &input.default_branch, input.now)? {
        AuditState::Fresh => {}
        AuditState::Problem(why) => gathered.blockers.push(Blocker {
            code: BlockerCode::SettingsAudit,
            detail: why,
        }),
    }
    let decision = verdict::compute(&gathered.reasons, &gathered.blockers);
    let info = Info {
        approvers: Vec::new(),
        blockers: gathered.blockers.clone(),
        commits: commits.clone(),
        counts: gathered.counts.clone(),
        policy_commit: input.policy_commit.clone(),
        reasons: gathered.reasons.clone(),
        verdict: decision.verdict,
    };
    let data = VerdictData {
        approvable: decision.approvable,
        attempt: input.run.attempt,
        blockers: gathered.blockers,
        commits: commits.clone(),
        counts: gathered.counts,
        inputs: gathered.inputs,
        policy_commit: input.policy_commit.clone(),
        pull: input.env.pull,
        reasons: gathered.reasons,
        reviewed: None,
        run_id: input.run.run_id,
        verdict: decision.verdict,
    };
    let check = report::build_check(&info, Approval::NotApproved);
    checkrun::upsert(
        clients.app,
        &commits.head,
        &checkrun::payload(&check, &data),
    )?;
    match minimize_old_comments(clients.app, input.env.pull, &commits.head)? {
        CommentState::Exists => {}
        CommentState::Missing => post_comment(clients, input, &info)?,
    }
    Ok(data)
}

/// Whether the pull request is still open at `head` and can be merged.
fn pull_is_mergeable(pull: &Json, head: &Sha) -> Answer {
    let open = pull.text_at(&Name::of("state")).map(Text::as_str) == Some("open");
    let draft = pull
        .get(&Name::of("draft"))
        .and_then(Json::as_flag)
        .is_some_and(|flag| flag == Flag::On);
    let current = pull
        .get(&Name::of("head"))
        .and_then(|found| found.text_at(&Name::of("sha")))
        .map(Text::as_str)
        == Some(head.as_str());
    (open && !draft && current).answer()
}

/// Squash-merges the pull request at exactly the evaluated head when the
/// verdict was `auto`.
pub(crate) fn merge(client: &dyn Github, input: &MergeInput) -> Result<MergeOutcome, Failure> {
    let data = &input.data;
    let head = &input.env.commits.head;
    let clean = data.verdict == Verdict::Auto
        && data.reasons.is_empty()
        && data.blockers.is_empty()
        && data.commits.head == *head;
    if !clean {
        return Ok(MergeOutcome::NotAuto);
    }
    let pull_path = client.repo_path(&Text::from_string(format!(
        "/pulls/{}",
        input.env.pull.count().get()
    )));
    let pull = client.get(&pull_path)?;
    if pull_is_mergeable(&pull, head) == Answer::No {
        return Ok(MergeOutcome::PullChanged);
    }
    let base_ref = pull
        .get(&Name::of("base"))
        .and_then(|found| found.text_at(&Name::of("ref")))
        .map(Text::as_str);
    if base_ref != Some(input.default_branch.as_str()) {
        return Ok(MergeOutcome::WrongBase);
    }
    let message = verdict::merge_message(
        input.env.pull,
        &BlockFacts {
            approvers: Vec::new(),
            codes: Vec::new(),
            commits: input.env.commits.clone(),
            policy_commit: data.policy_commit.clone(),
        },
    )?;
    let body = Json::object_of(vec![
        (Name::of("sha"), Json::text_of(&Text::of(head.as_str()))),
        (Name::of("merge_method"), Json::text_of(&Text::of("squash"))),
        (Name::of("commit_title"), Json::text_of(&message.subject)),
        (Name::of("commit_message"), Json::text_of(&message.body)),
    ]);
    let merge_path = Text::from_string(format!("{}/merge", pull_path.as_str()));
    match client.send(Method::Put, &merge_path, &body) {
        Ok(_reply) => Ok(MergeOutcome::Merged),
        Err(failure) => Ok(MergeOutcome::Refused(Text::from_string(
            failure.text().as_str().to_owned(),
        ))),
    }
}

/// The file the decision is handed to the merge step in.
pub(crate) fn verdict_file(vars: &Vars) -> PathBuf {
    let dir = match vars.get(&Name::of("RUNNER_TEMP")) {
        Some(found) => PathBuf::from(found.as_str()),
        None => PathBuf::from("."),
    };
    dir.join(VERDICT_FILE)
}

/// The commit of the base checkout in `base_dir`.
pub(crate) fn policy_commit(base_dir: &Path) -> Result<Sha, Failure> {
    let output = process::git(base_dir, &Args::of(&["rev-parse", "HEAD"]))?;
    Sha::parse(&Text::of(output.lossy_text().as_str().trim()))
        .ok_or_else(|| Failure::process(&Message::of("git rev-parse HEAD printed no commit id")))
}
