//! The subcommands that talk to GitHub: `post`, `review`, `push-skip`, `release-gate` and `audit`.

use std::path::PathBuf;

use crate::audit::{self, live, Audited, MergeCommit};
use crate::data::VerdictData;
use crate::exit::Report;
use crate::failure::Failure;
use crate::github::{GhClient, Github};
use crate::io::{self, Vars};
use crate::json::{self, Json};
use crate::post::{self, Clients, DecideInput, MergeInput, MergeOutcome, RunIds};
use crate::pushskip::{self, Judged};
use crate::releasegate::{self, Verdict};
use crate::review::{self, Running};
use crate::types::{Answer, Count, Message, Name, Sha, Text, Token};
use crate::verdict::PullNumber;

use super::args::Arguments;
use super::{FIRST, SECOND, THIRD};

/// A client for the repository named by `GITHUB_REPOSITORY`, using the token
/// in the environment variable `token_variable`.
fn client(vars: &Vars, token_variable: &Name) -> Result<GhClient, Failure> {
    let token = vars.require(token_variable)?;
    let repo = vars.require(&Name::of("GITHUB_REPOSITORY"))?;
    Ok(GhClient::new(&Token::from_text(&token), &repo))
}

/// `post decide <results-dir> <base-dir>` or `post merge`.
pub(crate) fn post(args: &Arguments, vars: &Vars) -> Result<Report, Failure> {
    match args.word(FIRST)?.as_str() {
        "decide" => decide(args, vars),
        "merge" => merge(vars),
        _ => Err(Failure::usage(&Message::of("post needs decide or merge"))),
    }
}

/// Runs the `decide` command.
fn decide(args: &Arguments, vars: &Vars) -> Result<Report, Failure> {
    let app = client(vars, &Name::of("GH_TOKEN"))?;
    let reader = client(vars, &Name::of("READ_TOKEN"))?;
    let env = post::validated_env(vars)?;
    let attempt_text = match vars.get(&Name::of("GITHUB_RUN_ATTEMPT")) {
        Some(found) => Count::parse(found),
        None => Some(Count::new(1)),
    };
    let id_text = Count::parse(&vars.text(&Name::of("GITHUB_RUN_ID")));
    let (Some(run_id), Some(attempt)) = (id_text, attempt_text) else {
        return Err(Failure::refused(&Message::of(
            "refusing: GITHUB_RUN_ID/GITHUB_RUN_ATTEMPT are not numbers",
        )));
    };
    let input = DecideInput {
        default_branch: Name::of(vars.require(&Name::of("DEFAULT_BRANCH"))?.as_str()),
        env,
        now: io::now_epoch_seconds()?,
        policy_commit: post::policy_commit(&args.word_path(THIRD)?)?,
        results_dir: args.word_path(SECOND)?,
        run: RunIds { attempt, run_id },
    };
    let data = post::decide(
        &Clients {
            app: &app,
            reader: &reader,
        },
        vars,
        &input,
    )?;
    io::write_text(&post::verdict_file(vars), &data.to_json().compact())?;
    Ok(Report::clean())
}

/// Runs the `merge` command.
fn merge(vars: &Vars) -> Result<Report, Failure> {
    let app = client(vars, &Name::of("GH_TOKEN"))?;
    let env = post::validated_env(vars)?;
    let stored = json::parse(&io::read_text(&post::verdict_file(vars))?)
        .map_err(|message| Failure::parse(&message))?;
    let outcome = post::merge(
        &app,
        &MergeInput {
            data: VerdictData::from_json(&stored)?,
            default_branch: Name::of(vars.require(&Name::of("DEFAULT_BRANCH"))?.as_str()),
            env,
        },
    )?;
    let report = Report::saying(&outcome.describe());
    Ok(match outcome {
        MergeOutcome::Refused(_) => report.into_failed(),
        MergeOutcome::NotAuto
        | MergeOutcome::PullChanged
        | MergeOutcome::WrongBase
        | MergeOutcome::Merged => report,
    })
}

/// `review [--head-sha SHA] [PR_NUMBER]`: one pull request; or, when the number
/// is absent or empty, the open pull requests whose head is SHA, or all open
/// ones when there is no SHA or no such pull request. A pull request that
/// cannot be evaluated does not stop the others; the command then fails.
pub(crate) fn review(args: &Arguments, vars: &Vars) -> Result<Report, Failure> {
    let app = client(vars, &Name::of("GH_TOKEN"))?;
    let reader = client(vars, &Name::of("READ_TOKEN"))?;
    let default_branch = Name::of(vars.require(&Name::of("DEFAULT_BRANCH"))?.as_str());
    let base_dir = match args.optional(&Name::of("base-dir")) {
        Some(found) => PathBuf::from(found.as_str()),
        None => PathBuf::from("."),
    };
    let running = Running {
        default_branch: default_branch.clone(),
        policy_commit: post::policy_commit(&base_dir)?,
    };
    let numbers = match args.maybe_word(FIRST) {
        Some(word) if !word.as_str().is_empty() => match PullNumber::parse(&word) {
            Some(found) => vec![found],
            None => {
                return Ok(Report::saying(&Text::of("refusing: bad PR number")).into_failed());
            }
        },
        Some(_) | None => match args.optional(&Name::of("head-sha")) {
            Some(text) if !text.as_str().is_empty() => {
                let Some(head) = Sha::parse(&text) else {
                    return Ok(Report::saying(&Text::of("refusing: bad head sha")).into_failed());
                };
                found_by_head(&app, &default_branch, &head)?
            }
            Some(_) | None => review::open_pull_numbers(&app, &default_branch)?,
        },
    };
    let clients = Clients {
        app: &app,
        reader: &reader,
    };
    let done = review::sweep(&clients, &numbers, &running);
    let report = Report {
        lines: done.lines,
        ..Report::clean()
    };
    Ok(match done.failed {
        Answer::Yes => report.into_failed(),
        Answer::No => report,
    })
}

/// The pull requests whose head is `head`, or every open pull request when
/// there is none or the lookup fails: a review that cannot tell which pull
/// request it is for re-evaluates all of them, never fewer.
fn found_by_head(
    client: &dyn Github,
    default_branch: &Name,
    head: &Sha,
) -> Result<Vec<PullNumber>, Failure> {
    match review::open_pulls_with_head(client, default_branch, head) {
        Ok(found) if !found.is_empty() => Ok(found),
        Ok(_) | Err(_) => review::open_pull_numbers(client, default_branch),
    }
}

/// `push-skip <commit>`: prints `skip=yes|no` and a note for `$GITHUB_OUTPUT`.
/// It always exits 0 once the commit id is valid; a missing token or any API
/// error means "run the checks".
pub(crate) fn push_skip(args: &Arguments, vars: &Vars) -> Result<Report, Failure> {
    let pushed = args.word_commit(FIRST)?;
    let judged = match client(vars, &Name::of("GH_TOKEN")) {
        Ok(found) => pushskip::judge(&found, &pushed),
        Err(_) => Judged::run(&Text::of("no token or repository, running the checks")),
    };
    Ok(Report {
        lines: judged.lines(),
        ..Report::clean()
    })
}

/// `release-gate <commit>`: exits 0 when the tagged commit passed the ordinary
/// checks and 1 with a one-line reason otherwise. A missing token or any API
/// error also refuses.
pub(crate) fn release_gate(args: &Arguments, vars: &Vars) -> Result<Report, Failure> {
    let tagged = args.word_commit(FIRST)?;
    let default_branch = Name::of(vars.require(&Name::of("DEFAULT_BRANCH"))?.as_str());
    let verdict = match client(vars, &Name::of("GH_TOKEN")) {
        Ok(found) => releasegate::judge(&found, &tagged, &default_branch),
        Err(_) => Verdict::closed(&Text::of("no token or repository")),
    };
    Ok(verdict.report())
}

/// The recent commits of the default branch, with their pull request counts.
fn recent_commits(client: &dyn Github, expected: &Json) -> Result<Vec<MergeCommit>, Failure> {
    let branch = expected
        .text_at(&Name::of("default_branch"))
        .cloned()
        .ok_or_else(|| Failure::parse(&Message::of("settings: default_branch")))?;
    let recent = expected
        .dig(&keys!["merge_commits", "recent"])
        .and_then(Json::as_count)
        .ok_or_else(|| Failure::parse(&Message::of("settings: merge_commits.recent")))?;
    let listing = client.get(&client.repo_path(&Text::from_string(format!(
        "/commits?sha={}&per_page={}",
        branch.as_str(),
        recent.get()
    ))))?;
    let mut commits = Vec::new();
    for item in json::items_of(Some(&listing)) {
        let sha = item
            .text_at(&Name::of("sha"))
            .and_then(Sha::parse)
            .ok_or_else(|| Failure::api(&Message::of("commit without a sha")))?;
        let message = item
            .dig(&keys!["commit", "message"])
            .and_then(Json::as_text)
            .cloned()
            .ok_or_else(|| Failure::api(&Message::of("commit without a message")))?;
        let pulls = client.get(&client.repo_path(&Text::from_string(format!(
            "/commits/{}/pulls",
            sha.as_str()
        ))))?;
        commits.push(MergeCommit {
            message,
            pulls: Count::new(json::items_of(Some(&pulls)).len()),
            sha,
        });
    }
    Ok(commits)
}

/// `audit <settings.json> [drift-file]`.
pub(crate) fn audit(args: &Arguments, vars: &Vars) -> Result<Report, Failure> {
    let expected = super::jobs::read_json(&args.word_path(FIRST)?)?;
    let audit_client = client(vars, &Name::of("AUDIT_TOKEN"))?;
    let observed = live::gather(&audit_client, &expected)?;
    let mut drift = audit::compare(&Audited {
        expected: &expected,
        live: &observed,
    });
    let prefix = expected
        .dig(&keys!["merge_commits", "auto_subject_prefix"])
        .and_then(Json::as_text)
        .cloned()
        .ok_or_else(|| Failure::parse(&Message::of("settings: auto_subject_prefix")))?;
    drift.extend(audit::check_merge_commits(
        &recent_commits(&audit_client, &expected)?,
        &prefix,
    ));
    if let Some(file) = args.maybe_word(SECOND) {
        let body: String = drift
            .iter()
            .map(|item| format!("- {}\n", item.as_str()))
            .collect::<Vec<String>>()
            .concat();
        io::write_text(&PathBuf::from(file.as_str()), &Text::from_string(body))?;
    }
    let lines: Vec<Text> = drift
        .iter()
        .map(|item| Text::from_string(format!("DRIFT: {}", item.as_str())))
        .collect();
    Ok(if lines.is_empty() {
        Report::clean()
    } else {
        Report::failed(lines)
    })
}
