//! Fetch a PR head into a directory as files only (never built or run here).
//!
//! The head is fetched into a fresh repository with the token passed through
//! git's environment configuration (never argv, never a stored credential),
//! then the base objects come from the local base checkout, not the network.

use crate::bytes::{self, Bytes};
use crate::failure::Failure;
use crate::io::{self, Vars};
use crate::post;
use crate::process::{self, Args, EnvPair, Invocation};
use crate::types::{Message, Name, Sha, Text};
use std::path::PathBuf;

/// An environment variable for a git child process.
macro_rules! env_pair {
    ($name:literal, $value:expr_2021) => {
        EnvPair {
            name: Name::of($name),
            value: Text::of($value),
        }
    };
}

/// Where to put the head and where the base checkout is.
pub(crate) struct FetchHead {
    /// The directory of the base checkout.
    pub base_dir: PathBuf,
    /// The directory to put the head in.
    pub dest: PathBuf,
}

/// `owner/name` made of the characters a repository path may hold.
fn repository_of(vars: &Vars) -> Result<Text, Failure> {
    let repo = vars.require(&Name::of("GITHUB_REPOSITORY"))?;
    let fine = repo.as_str().contains('/')
        && repo
            .as_str()
            .chars()
            .all(|found| found.is_ascii_alphanumeric() || matches!(found, '.' | '_' | '-' | '/'));
    if fine {
        Ok(repo)
    } else {
        Err(Failure::refused(&Message::of(
            "refusing: GITHUB_REPOSITORY is not owner/name",
        )))
    }
}

/// A git invocation that runs in the base checkout.
fn git_in(request: &FetchHead, args: &Args) -> Invocation {
    Invocation::new(&Name::of("git"), args)
        .in_dir(&request.dest)
        .without_env(&Name::of("GH_TOKEN"))
        .with_env(&env_pair!("GIT_CONFIG_GLOBAL", "/dev/null"))
        .with_env(&env_pair!("GIT_CONFIG_SYSTEM", "/dev/null"))
}

/// Runs a git invocation and checks that it succeeded.
fn run_git(invocation: &Invocation) -> Result<(), Failure> {
    process::run(invocation)?
        .stdout_or_fail(&Name::of("git"))
        .map(|_output| ())
}

/// Fetches the pull request head and the base objects, and checks the head
/// out detached.
pub(crate) fn run(request: &FetchHead, vars: &Vars) -> Result<(), Failure> {
    let env = post::validated_env(vars)?;
    let repo = repository_of(vars)?;
    let token = vars.text(&Name::of("GH_TOKEN"));
    let credentials = bytes::encode(&Bytes::of_text(&Text::from_string(format!(
        "x-access-token:{}",
        token.as_str()
    ))));
    let auth = format!("AUTHORIZATION: basic {}", credentials.as_str());
    io::create_dirs(&request.dest)?;
    process::git(&request.dest, &Args::of(&["init", "-q"])).map(|_output| ())?;
    let url = format!("https://github.com/{}.git", repo.as_str());
    let refspec = format!(
        "+refs/pull/{}/head:refs/remotes/pr/head",
        env.pull.count().get()
    );
    run_git(
        &git_in(
            request,
            &Args::of(&["fetch", "--no-tags", "--quiet", &url, &refspec]),
        )
        .with_env(&env_pair!("GIT_TERMINAL_PROMPT", "0"))
        .with_env(&env_pair!("GIT_CONFIG_COUNT", "1"))
        .with_env(&env_pair!(
            "GIT_CONFIG_KEY_0",
            "http.https://github.com/.extraheader"
        ))
        .with_env(&env_pair!("GIT_CONFIG_VALUE_0", &auth)),
    )?;
    let got = process::git(
        &request.dest,
        &Args::of(&["rev-parse", "refs/remotes/pr/head"]),
    )?
    .lossy_text();
    if Sha::parse(&Text::of(got.as_str().trim())) != Some(env.commits.head.clone()) {
        return Err(Failure::refused(&Message::of(
            "refusing: PR head moved after the event (a newer run will evaluate it)",
        )));
    }
    let base_path = io::absolute(&request.base_dir)?;
    run_git(&git_in(
        request,
        &Args::of(&[
            "fetch",
            "--no-tags",
            "--quiet",
            &base_path.to_string_lossy(),
            env.commits.base.as_str(),
        ]),
    ))?;
    run_git(&git_in(
        request,
        &Args::of(&[
            "-c",
            "core.hooksPath=/dev/null",
            "checkout",
            "-q",
            "--detach",
            env.commits.head.as_str(),
        ]),
    ))
}
