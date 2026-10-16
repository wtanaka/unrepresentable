//! Whether a push to the default branch can skip its checks.
//!
//! A pull request's head commit already went through the `policy` check. When
//! the pushed commit has the same git tree as the head of a pull request it
//! belongs to, and the App's `policy` check run for that head succeeded, the
//! same files were already checked and the expensive jobs can be skipped. Any
//! doubt means the checks run: a missing, failed or unfinished check run, a
//! different tree, no pull request, or an API error. The decision never skips
//! on an error.

#[cfg(test)]
mod tests;

use crate::checkrun;
use crate::failure::Failure;
use crate::github::Github;
use crate::json::Json;
use crate::types::{Answer, Holds as _, Message, Name, Sha, Text};

/// The most characters of an error message kept in a note.
const NOTE_LIMIT: usize = 160;

/// What the push workflow does with its checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Decision {
    /// Run the checks.
    Run,
    /// The same tree already passed; skip the checks.
    Skip,
}

/// A decision and the one-line note that explains it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Judged {
    /// What to do.
    pub decision: Decision,
    /// Why, on one line.
    pub note: Text,
}

impl Judged {
    /// The `key=value` lines for `$GITHUB_OUTPUT`.
    pub(crate) fn lines(&self) -> Vec<Text> {
        let word = match self.decision {
            Decision::Run => "no",
            Decision::Skip => "yes",
        };
        vec![
            Text::from_string(format!("skip={word}")),
            Text::from_string(format!("note={}", self.note.as_str())),
        ]
    }

    /// Run the checks, for `why`.
    pub(crate) fn run(why: &Text) -> Self {
        Self {
            decision: Decision::Run,
            note: why.clone(),
        }
    }
}

/// The message of `failure` on one line, cut to a note's length.
pub(crate) fn one_line(failure: &Failure) -> Text {
    Text::from_string(
        failure
            .text()
            .as_str()
            .chars()
            .map(|found| if found.is_control() { ' ' } else { found })
            .take(NOTE_LIMIT)
            .collect(),
    )
}

/// The tree of the commit `commit`, as the API reports it.
fn tree_of(client: &dyn Github, commit: &Sha) -> Result<Sha, Failure> {
    let reply = client.get(&client.repo_path(&Text::from_string(format!(
        "/git/commits/{}",
        commit.as_str()
    ))))?;
    reply
        .dig(&keys!["tree", "sha"])
        .and_then(Json::as_text)
        .and_then(Sha::parse)
        .ok_or_else(|| Failure::api(&Message::of("commit without a tree")))
}

/// Whether the App's `policy` check run for `head` exists and every such run
/// finished with success.
fn passed(client: &dyn Github, head: &Sha) -> Result<Answer, Failure> {
    let runs = checkrun::app_check_runs(client, head)?;
    let succeeded = |run: &Json| {
        run.text_at(&Name::of("status")).map(Text::as_str) == Some("completed")
            && run.text_at(&Name::of("conclusion")).map(Text::as_str) == Some("success")
    };
    Ok((!runs.is_empty() && runs.iter().all(succeeded)).answer())
}

/// The head of a pull request that contains `pushed`, has the same tree and
/// has a successful `policy` check run; none when there is no such head.
fn matching_head(client: &dyn Github, pushed: &Sha) -> Result<Option<Sha>, Failure> {
    let tree = tree_of(client, pushed)?;
    let pulls = client.paged(
        &client.repo_path(&Text::from_string(format!(
            "/commits/{}/pulls",
            pushed.as_str()
        ))),
        None,
    )?;
    for pull in &pulls {
        let Some(head) = pull
            .dig(&keys!["head", "sha"])
            .and_then(Json::as_text)
            .and_then(Sha::parse)
        else {
            continue;
        };
        if tree_of(client, &head)? == tree && passed(client, &head)? == Answer::Yes {
            return Ok(Some(head));
        }
    }
    Ok(None)
}

/// The decision for the pushed commit `pushed`. It skips only on positive
/// evidence and runs the checks on every error.
pub(crate) fn judge(client: &dyn Github, pushed: &Sha) -> Judged {
    match matching_head(client, pushed) {
        Ok(Some(head)) => Judged {
            decision: Decision::Skip,
            note: Text::from_string(format!(
                "same tree as pull request head {} with a successful policy check",
                head.short().as_str()
            )),
        },
        Ok(None) => Judged::run(&Text::of(
            "no pull request head with the same tree and a successful policy check",
        )),
        Err(failure) => Judged::run(&Text::from_string(format!(
            "could not confirm, running the checks: {}",
            one_line(&failure).as_str()
        ))),
    }
}
