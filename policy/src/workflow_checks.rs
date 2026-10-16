//! Static guarantees about every workflow file (read as text: there is no YAML
//! library in the dependency set).

use core::iter::once;
use std::ffi::OsStr;
use std::path::Path;

use crate::bytes::Bytes;
use crate::headjobs::EXPECTED_CHECKS;
use crate::io::{self, EntryKind};
use crate::process::{self, Args, Invocation, Status};
use crate::reason::Job;
use crate::releasegate::{REQUIRED_JOBS, SKIP_JOB, WORKFLOW_NAME, WORKFLOW_PATH};
use crate::testkit::Fail;
use crate::types::{Answer, Count, Holds as _, Message, Name, Text};

const WORKFLOWS: &str = "../.github/workflows";
const PR_NEEDLES: [&str; 6] = [
    "github.event.",
    "github.head_ref",
    "github.ref_name",
    "github.actor",
    "pull_request",
    "inputs.",
];

/// The jobs of `policy.yml` that run only on pushes and manual runs.
const PUSH_JOBS: [&str; 10] = [
    "audit-workflows",
    "canaries",
    "clippy-main",
    "clippy-policy",
    "deny",
    "fmt",
    "policytest",
    "self-checks",
    "skip-check",
    "test",
];
/// The checks of `ci/checks.sh` that need the network. They run in the push
/// jobs and the scheduled sweep, never in the offline `head-checks`.
const NETWORK_CHECKS: [&str; 1] = ["deny-advisories"];
/// The jobs of `policy.yml` that run on pull requests and use the policy tool
/// built by `build-base`.
const PR_JOBS: [&str; 8] = [
    "classify",
    "self-lint",
    "head-checks",
    "build-head",
    "differential",
    "differential-hidden",
    "mutants",
    "policy-evaluate",
];
/// The jobs of `policy.yml` that handle head code or head data: they run it as
/// an unprivileged user with no network, and they hold no secret and no
/// environment.
const HEAD_JOBS: [&str; 4] = ["build-head", "head-checks", "mutants", "self-lint"];

/// Manifest text, the version `ci/version.sh` prints for it, and its exit status.
static VERSION_SCRIPT_CASES: [(&str, &str, usize); 26] = [
        ("[package]\nname = \"a\"\nversion = \"1.2.3\"\n", "1.2.3\n", 0),
        (
            "[workspace.package]\nversion = \"9.9.9\"\n[package]\nname = \"a\"\nversion = \"0.1.0\" # note\n",
            "0.1.0\n",
            0,
        ),
        (
            "[dependencies]\nx = { version = \"2.0.0\" }\n[dependencies.y]\nversion = \"3.0.0\"\n[package]\nversion = \"0.4.5\"\n[dev-dependencies.z]\nversion = \"7.7.7\"\n",
            "0.4.5\n",
            0,
        ),
        ("[package]\nversion.workspace = true\n", "", 1),
        ("[workspace.package]\nversion = \"1.0.0\"\n", "", 1),
        ("[dependencies.y]\nversion = \"3.0.0\"\n", "", 1),
        ("[package]\nversion = \"1.0\"\n", "", 1),
        ("[package]\nversion = \"1.0.0-rc.1\"\n", "1.0.0-rc.1\n", 0),
        ("[package]\nversion = \"1.2.3-alpha.1\"\n", "1.2.3-alpha.1\n", 0),
        ("[package]\nversion = \"0.0.0\"\n", "0.0.0\n", 0),
        ("[package]\nversion = \"10.20.30\"\n", "10.20.30\n", 0),
        ("[package]\nversion = \"0.1.0-alpha.1\" # note\n", "0.1.0-alpha.1\n", 0),
        ("[package]\nversion = \"1.2.3-x-y.0.7a\"\n", "1.2.3-x-y.0.7a\n", 0),
        ("[package]\nversion = \"01.2.3\"\n", "", 1),
        ("[package]\nversion = \"1.02.3\"\n", "", 1),
        ("[package]\nversion = \"1.2.03\"\n", "", 1),
        ("[package]\nversion = \"1.2.3-rc.01\"\n", "", 1),
        ("[package]\nversion = \"1.2.3-\"\n", "", 1),
        ("[package]\nversion = \"1.2.3+build\"\n", "", 1),
        ("[package]\nversion = \"1.2.3-alpha.1+build\"\n", "", 1),
        ("[package]\nversion = \"1.2.3-alpha..1\"\n", "", 1),
        ("[package]\nversion = \"1.2.3-.alpha\"\n", "", 1),
        ("[package]\nversion = \"1.2.3-alpha.\"\n", "", 1),
        ("[package]\nversion = \"1.2.3-al_pha\"\n", "", 1),
        ("[package]\nversion = \"v1.2.3-alpha.1\"\n", "", 1),
        ("", "", 1),
];

/// One workflow file: its name and text.
struct Workflow {
    name: Name,
    text: Text,
}

/// One job of a workflow: its name and its text up to the next job.
struct WorkflowJob {
    body: Text,
    name: Name,
}

fn workflows() -> Result<Vec<Workflow>, Fail> {
    let mut found = Vec::new();
    for entry in io::list_dir(Path::new(WORKFLOWS))? {
        if entry.kind == EntryKind::File
            && Path::new(entry.name.as_str()).extension() == Some(OsStr::new("yml"))
        {
            let text = io::read_text(&Path::new(WORKFLOWS).join(entry.name.as_str()))?;
            found.push(Workflow {
                name: entry.name,
                text,
            });
        }
    }
    Ok(found)
}

/// The jobs of `item`, each with its text up to the next job head.
fn jobs_of(item: &Workflow) -> Result<Vec<WorkflowJob>, Fail> {
    let Some((_, jobs)) = item.text.as_str().split_once("\njobs:\n") else {
        return Err(Fail::new(&Message::from_string(format!(
            "{} has no jobs",
            item.name.as_str()
        ))));
    };
    let mut found: Vec<(Name, Vec<&str>)> = Vec::new();
    for line in jobs.lines() {
        if is_job_head(&Text::of(line)) == Answer::Yes {
            found.push((Name::of(line.trim().trim_end_matches(':')), vec![line]));
            continue;
        }
        if let Some((_, lines)) = found.last_mut() {
            lines.push(line);
        }
    }
    Ok(found
        .into_iter()
        .map(|(name, lines)| WorkflowJob {
            body: Text::from_string(lines.join("\n")),
            name,
        })
        .collect())
}

/// The job called `wanted` in `policy.yml`.
fn policy_job(wanted: &Name) -> Result<WorkflowJob, Fail> {
    jobs_of(&workflow_named(&Name::of("policy.yml"))?)?
        .into_iter()
        .find(|job| job.name == *wanted)
        .ok_or_else(|| Fail::new(&Message::from_string(format!("no job {}", wanted.as_str()))))
}

fn workflow_named(wanted: &Name) -> Result<Workflow, Fail> {
    workflows()?
        .into_iter()
        .find(|item| item.name == *wanted)
        .ok_or_else(|| {
            Fail::new(&Message::from_string(format!(
                "no workflow {}",
                wanted.as_str()
            )))
        })
}

fn indent_of(line: &Text) -> Count {
    Count::new(
        line.as_str()
            .chars()
            .take_while(|found| found.is_whitespace())
            .count(),
    )
}

/// The text of every `key:` value in `lines`: the inline value and any more
/// deeply indented lines after it.
fn blocks(lines: &[Text], key: &Name) -> Result<Vec<Text>, Fail> {
    let marker = format!("{}:", key.as_str());
    let mut found = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.as_str().trim_start();
        let (dash, after_dash) = match trimmed.strip_prefix("- ") {
            Some(rest) => (Count::new(2), rest),
            None => (Count::new(0), trimmed),
        };
        let Some(inline) = after_dash.strip_prefix(&marker) else {
            continue;
        };
        let limit = indent_of(line).plus(dash)?;
        let mut body = vec![inline.trim_start().to_owned()];
        for next in lines.iter().skip(Count::new(index).next()?.get()) {
            if next.as_str().trim().is_empty() || indent_of(next).get() > limit.get() {
                body.push(next.as_str().to_owned());
            } else {
                break;
            }
        }
        found.push(Text::from_string(body.join("\n")));
    }
    Ok(found)
}

/// Whether a `${{ ... }}` expression in `body` mentions PR-controlled data.
fn has_pr_expression(body: &Text) -> Answer {
    body.as_str()
        .split("${{")
        .skip(1)
        .any(|piece| {
            piece.split_once("}}").is_some_and(|(inside, _)| {
                !inside.contains('}') && PR_NEEDLES.iter().any(|needle| inside.contains(needle))
            })
        })
        .answer()
}

#[test]
fn there_are_workflows() {
    assert_eq!(check_there_are_workflows(), Ok(()), "there_are_workflows");
}

fn check_there_are_workflows() -> Result<(), Fail> {
    let names: Vec<String> = workflows()?
        .iter()
        .map(|item| item.name.as_str().to_owned())
        .collect();
    for wanted in [
        "policy.yml",
        "review-signal.yml",
        "policy-review.yml",
        "policy-sweep.yml",
        "settings-audit.yml",
        "release.yml",
    ] {
        ensure!(
            names.iter().any(|name| name == wanted),
            "{wanted} exists in {names:?}"
        );
    }
    ensure!(
        !names.iter().any(|name| name == "ci.yml"),
        "ci.yml was merged into policy.yml and stays gone: {names:?}"
    );
    Ok(())
}

#[test]
fn every_cargo_call_uses_the_entry_point() {
    assert_eq!(
        check_every_cargo_call_uses_the_entry_point(),
        Ok(()),
        "every_cargo_call_uses_the_entry_point"
    );
}

fn check_every_cargo_call_uses_the_entry_point() -> Result<(), Fail> {
    for item in workflows()? {
        for line in item.text.as_str().lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') || trimmed.starts_with("- name:") {
                continue;
            }
            ensure!(
                !trimmed.split_whitespace().any(|word| word == "cargo"),
                "{}: `{trimmed}` calls cargo directly; use ci/cargo.sh",
                item.name.as_str()
            );
        }
    }
    Ok(())
}

#[test]
fn the_entry_point_bakes_in_release_and_locked() {
    assert_eq!(
        check_the_entry_point_bakes_in_release_and_locked(),
        Ok(()),
        "the_entry_point_bakes_in_release_and_locked"
    );
}

fn check_the_entry_point_bakes_in_release_and_locked() -> Result<(), Fail> {
    let script = io::read_text(Path::new("../ci/cargo.sh"))?;
    for wanted in [
        "exec cargo \"$subcommand\" --release --locked \"$@\"",
        "exec cargo \"$subcommand\" --locked \"$@\"",
        "--all | --workspace)",
        "must name a package with -p NAME",
    ] {
        ensure!(
            script.as_str().contains(wanted),
            "ci/cargo.sh contains {wanted}"
        );
    }
    Ok(())
}

#[test]
fn top_level_permissions_empty() {
    assert_eq!(
        check_top_level_permissions_empty(),
        Ok(()),
        "top_level_permissions_empty"
    );
}

fn check_top_level_permissions_empty() -> Result<(), Fail> {
    for item in workflows()? {
        ensure!(
            item.text
                .as_str()
                .lines()
                .any(|line| line.trim_end() == "permissions: {}"),
            "{} has empty top-level permissions",
            item.name.as_str()
        );
    }
    Ok(())
}

#[test]
fn every_job_declares_permissions() {
    assert_eq!(
        check_every_job_declares_permissions(),
        Ok(()),
        "every_job_declares_permissions"
    );
}

fn is_job_head(line: &Text) -> Answer {
    let Some(rest) = line.as_str().strip_prefix("  ") else {
        return Answer::No;
    };
    let Some(name) = rest.trim_end().strip_suffix(':') else {
        return Answer::No;
    };
    (name
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_lowercase())
        && name
            .chars()
            .all(|found| found.is_ascii_lowercase() || found.is_ascii_digit() || found == '-'))
    .answer()
}

fn check_every_job_declares_permissions() -> Result<(), Fail> {
    for item in workflows()? {
        let Some((_, jobs)) = item.text.as_str().split_once("\njobs:\n") else {
            return Err(Fail::new(&Message::from_string(format!(
                "{} has no jobs",
                item.name.as_str()
            ))));
        };
        let heads = jobs
            .lines()
            .filter(|line| is_job_head(&Text::of(line)) == Answer::Yes)
            .count();
        let permissions = jobs
            .lines()
            .filter(|line| line.starts_with("    permissions:"))
            .count();
        ensure_eq!(
            heads,
            permissions,
            "{} declares permissions per job",
            item.name.as_str()
        );
    }
    Ok(())
}

fn is_sha_pinned(reference: &Text) -> Answer {
    let Some((path, sha)) = reference.as_str().split_once('@') else {
        return Answer::No;
    };
    (!path.is_empty()
        && path
            .chars()
            .all(|found| found.is_ascii_alphanumeric() || matches!(found, '_' | '.' | '/' | '-'))
        && sha.len() == 40
        && sha
            .chars()
            .all(|found| found.is_ascii_digit() || matches!(found, 'a'..='f')))
    .answer()
}

#[test]
fn third_party_actions_pinned_by_sha() {
    assert_eq!(
        check_third_party_actions_pinned_by_sha(),
        Ok(()),
        "third_party_actions_pinned_by_sha"
    );
}

fn check_third_party_actions_pinned_by_sha() -> Result<(), Fail> {
    for item in workflows()? {
        for (number, line) in item.text.as_str().lines().enumerate() {
            let trimmed = line.trim_start();
            let after_dash = match trimmed.strip_prefix("- ") {
                Some(rest) => rest.trim_start(),
                None => trimmed,
            };
            if let Some(value) = after_dash.strip_prefix("uses:") {
                let reference = value.split_whitespace().next();
                ensure!(
                    reference.is_some_and(|found| is_sha_pinned(&Text::of(found)) == Answer::Yes),
                    "{}:{} is not pinned by sha",
                    item.name.as_str(),
                    Count::new(number).next()?.get()
                );
            }
        }
    }
    Ok(())
}

#[test]
fn checkout_never_persists_credentials() {
    assert_eq!(
        check_checkout_never_persists_credentials(),
        Ok(()),
        "checkout_never_persists_credentials"
    );
}

fn check_checkout_never_persists_credentials() -> Result<(), Fail> {
    for item in workflows()? {
        let lines: Vec<&str> = item.text.as_str().lines().collect();
        for (index, line) in lines.iter().enumerate() {
            if line.contains("actions/checkout@") {
                let window: Vec<&str> = lines.iter().skip(index).take(8).copied().collect();
                ensure!(
                    window.join("\n").contains("persist-credentials: false"),
                    "{}:{} persists credentials",
                    item.name.as_str(),
                    Count::new(index).next()?.get()
                );
            }
        }
    }
    Ok(())
}

#[test]
fn no_pr_controlled_expression_in_run_script_or_with() {
    assert_eq!(
        check_no_pr_controlled_expression_in_run_script_or_with(),
        Ok(()),
        "no_pr_controlled_expression_in_run_script_or_with"
    );
}

fn check_no_pr_controlled_expression_in_run_script_or_with() -> Result<(), Fail> {
    for item in workflows()? {
        let lines: Vec<Text> = item.text.as_str().lines().map(Text::of).collect();
        for key in ["run", "script", "with"] {
            for body in blocks(&lines, &Name::of(key))? {
                ensure!(
                    has_pr_expression(&body) == Answer::No,
                    "{} {key}: PR-controlled expression in {}",
                    item.name.as_str(),
                    body.as_str()
                );
            }
        }
        for body in blocks(&lines, &Name::of("run"))? {
            ensure!(
                !body.as_str().contains("${{"),
                "{}: expression inside run: {}",
                item.name.as_str(),
                body.as_str()
            );
        }
    }
    Ok(())
}

#[test]
fn no_cache_in_policy_workflows() {
    assert_eq!(
        check_no_cache_in_policy_workflows(),
        Ok(()),
        "no_cache_in_policy_workflows"
    );
}

fn check_no_cache_in_policy_workflows() -> Result<(), Fail> {
    for name in ["policy.yml", "policy-review.yml"] {
        let item = workflow_named(&Name::of(name))?;
        ensure!(
            !item.text.as_str().contains("actions/cache"),
            "{name} uses no cache action"
        );
        ensure!(
            !item.text.as_str().contains("cache:"),
            "{name} sets no cache option"
        );
    }
    Ok(())
}

#[test]
fn policy_yml_trigger_and_environment() {
    assert_eq!(
        check_policy_yml_trigger_and_environment(),
        Ok(()),
        "policy_yml_trigger_and_environment"
    );
}

fn check_policy_yml_trigger_and_environment() -> Result<(), Fail> {
    let item = workflow_named(&Name::of("policy.yml"))?;
    let text = item.text.as_str();
    for trigger in ["  pull_request_target:", "  push:", "  workflow_dispatch:"] {
        ensure!(
            text.lines().any(|line| line == trigger),
            "policy.yml has the trigger {trigger}"
        );
    }
    ensure!(
        !text.lines().any(|line| line.starts_with("  pull_request:")),
        "policy.yml does not run on pull_request"
    );
    ensure_eq!(
        text.matches("environment: policy").count(),
        1,
        "one policy environment"
    );
    for job in [
        "head-checks",
        "build-head",
        "mutants",
        "self-lint",
        "differential",
    ] {
        let body = policy_job(&Name::of(job))?.body;
        ensure!(!body.as_str().contains("secrets."), "{job} sees no secrets");
        ensure!(
            !body.as_str().contains("environment:"),
            "{job} has no environment"
        );
    }
    for job in PUSH_JOBS {
        let body = policy_job(&Name::of(job))?.body;
        ensure!(
            !body.as_str().contains("secrets.") && !body.as_str().contains("environment:"),
            "{job} (push path) sees no secrets and no environment"
        );
        ensure!(
            body.as_str().contains("contents: read"),
            "{job} (push path) reads contents only"
        );
    }
    Ok(())
}

#[test]
fn every_policy_job_is_gated_by_the_event() {
    assert_eq!(
        check_every_policy_job_is_gated_by_the_event(),
        Ok(()),
        "every_policy_job_is_gated_by_the_event"
    );
}

fn check_every_policy_job_is_gated_by_the_event() -> Result<(), Fail> {
    let jobs = jobs_of(&workflow_named(&Name::of("policy.yml"))?)?;
    for job in &jobs {
        let gate = job
            .body
            .as_str()
            .lines()
            .find(|line| line.starts_with("    if:"));
        ensure!(
            gate.is_some_and(|line| line.contains("github.event_name")),
            "{} has an `if:` on github.event_name",
            job.name.as_str()
        );
    }
    let named: Vec<&str> = jobs.iter().map(|job| job.name.as_str()).collect();
    for expected in PR_JOBS.iter().chain(PUSH_JOBS.iter()) {
        ensure!(
            named.contains(expected),
            "policy.yml has the job {expected}: {named:?}"
        );
    }
    ensure!(
        named.contains(&"build-base"),
        "policy.yml has the job build-base: {named:?}"
    );
    Ok(())
}

#[test]
fn push_jobs_wait_for_the_skip_decision_and_fail_open() {
    assert_eq!(
        check_push_jobs_wait_for_the_skip_decision_and_fail_open(),
        Ok(()),
        "push_jobs_wait_for_the_skip_decision_and_fail_open"
    );
}

fn check_push_jobs_wait_for_the_skip_decision_and_fail_open() -> Result<(), Fail> {
    for job in PUSH_JOBS {
        if job == "skip-check" {
            continue;
        }
        let body = policy_job(&Name::of(job))?.body;
        let text = body.as_str();
        ensure!(
            text.contains("needs: [skip-check]"),
            "{job} needs skip-check"
        );
        ensure!(
            text.contains("needs.skip-check.outputs.skip != 'yes'"),
            "{job} honours the skip decision"
        );
        ensure!(
            text.contains("!cancelled()"),
            "{job} still runs when skip-check failed or was skipped"
        );
        ensure!(
            text.contains("github.event_name != 'pull_request_target'"),
            "{job} does not run on pull requests"
        );
    }
    let skip = policy_job(&Name::of("skip-check"))?.body;
    ensure!(
        skip.as_str().contains("github.event_name == 'push'"),
        "skip-check runs on pushes only"
    );
    ensure!(
        skip.as_str().contains("|| out='skip=no'"),
        "skip-check prints skip=no when the tool fails"
    );
    Ok(())
}

#[test]
fn base_binaries_come_from_one_build_and_are_verified() {
    assert_eq!(
        check_base_binaries_come_from_one_build_and_are_verified(),
        Ok(()),
        "base_binaries_come_from_one_build_and_are_verified"
    );
}

fn check_base_binaries_come_from_one_build_and_are_verified() -> Result<(), Fail> {
    let build = policy_job(&Name::of("build-base"))?.body;
    ensure!(
        build.as_str().contains("ref: ${{ github.sha }}")
            && build.as_str().contains("rev-parse HEAD"),
        "build-base builds the base commit and checks it"
    );
    ensure!(
        build.as_str().contains("policy-sha256:") && build.as_str().contains("tool-sha256:"),
        "build-base outputs both digests"
    );
    for job in PR_JOBS {
        let body = policy_job(&Name::of(job))?.body;
        let text = body.as_str();
        ensure!(
            text.contains("ref: ${{ github.sha }}"),
            "{job} checks out the base commit"
        );
        ensure!(
            text.contains("needs.build-base.outputs.policy-sha256")
                && text.contains("sha256sum -c -"),
            "{job} verifies the policy tool's sha256"
        );
        ensure!(
            !text.contains("build -p unrepresentable-policy"),
            "{job} does not rebuild the policy tool"
        );
        let lines: Vec<&str> = text.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            if line.contains("actions/upload-artifact") {
                let window: Vec<&str> = lines.iter().skip(index).take(3).copied().collect();
                ensure!(
                    !window.join("\n").contains("name: base-"),
                    "{job} does not upload a base binary"
                );
            }
        }
    }
    for job in ["self-lint", "differential", "differential-hidden"] {
        ensure!(
            policy_job(&Name::of(job))?
                .body
                .as_str()
                .contains("needs.build-base.outputs.tool-sha256"),
            "{job} verifies the base binary's sha256"
        );
    }
    Ok(())
}

#[test]
fn review_signal_does_nothing() {
    assert_eq!(
        check_review_signal_does_nothing(),
        Ok(()),
        "review_signal_does_nothing"
    );
}

fn check_review_signal_does_nothing() -> Result<(), Fail> {
    let item = workflow_named(&Name::of("review-signal.yml"))?;
    let text = item.text.as_str();
    ensure_eq!(
        text.lines()
            .filter(|line| line.trim() == "- run: exit 0")
            .count(),
        1,
        "one step that exits"
    );
    ensure!(!text.contains("secrets."), "no secrets");
    ensure_eq!(text.matches("steps:").count(), 1, "one steps list");
    Ok(())
}

#[test]
fn no_auto_merge_enablement_anywhere() {
    assert_eq!(
        check_no_auto_merge_enablement_anywhere(),
        Ok(()),
        "no_auto_merge_enablement_anywhere"
    );
}

fn check_no_auto_merge_enablement_anywhere() -> Result<(), Fail> {
    for item in workflows()? {
        let lowered = item.text.as_str().to_lowercase();
        ensure!(
            !lowered.contains("--auto"),
            "{} has no --auto",
            item.name.as_str()
        );
        ensure!(
            !lowered.contains("enablepullrequestautomerge"),
            "{} does not enable auto merge",
            item.name.as_str()
        );
    }
    Ok(())
}

#[test]
fn head_jobs_use_the_offline_wrapper() {
    assert_eq!(
        check_head_jobs_use_the_offline_wrapper(),
        Ok(()),
        "head_jobs_use_the_offline_wrapper"
    );
}

fn check_head_jobs_use_the_offline_wrapper() -> Result<(), Fail> {
    let item = workflow_named(&Name::of("policy.yml"))?;
    let text = item.text.as_str();
    for needle in [
        "\"$OFF\" sh \"$CHECKS\" \"$name\" \"$@\" --offline",
        "check clippy",
        "check test",
        "check owntree --range \"$BASE_SHA...HEAD\"",
        "ci/cargo.sh\" build --offline",
        "cargo-mutants mutants",
    ] {
        ensure!(text.contains(needle), "policy.yml contains {needle}");
    }
    ensure!(
        text.matches("offline.sh").count() >= 4,
        "offline.sh wraps the checks, the build, the mutants and the self-lint"
    );
    Ok(())
}

#[test]
fn workflows_call_the_rust_binary_not_python() {
    assert_eq!(
        check_workflows_call_the_rust_binary_not_python(),
        Ok(()),
        "workflows_call_the_rust_binary_not_python"
    );
}

fn check_workflows_call_the_rust_binary_not_python() -> Result<(), Fail> {
    for item in workflows()? {
        ensure!(
            !item.text.as_str().contains("python3"),
            "{} still calls python",
            item.name.as_str()
        );
    }
    Ok(())
}

/// The check names `text` passes to `checks.sh`: the word after the script,
/// and the word after the `check` helper of the head-checks job.
fn used_check_names(text: &Text) -> Vec<Name> {
    let mut names = Vec::new();
    for line in text.as_str().lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            continue;
        }
        let words: Vec<&str> = trimmed.split_whitespace().collect();
        let mut pairs: Vec<(&str, &str)> = words
            .windows(2)
            .filter_map(|pair| match pair {
                [first, second] => Some((*first, *second)),
                _ => None,
            })
            .collect();
        pairs.retain(|(first, _)| {
            first.trim_end_matches('"').ends_with("checks.sh") || *first == "\"$CHECKS\""
        });
        if let [first, second, ..] = words.as_slice() {
            if *first == "check" {
                pairs.push((first, second));
            }
        }
        for (_, second) in pairs {
            let cleaned = second.trim_end_matches(')');
            if is_check_name(&Text::of(cleaned)) == Answer::Yes {
                names.push(Name::of(cleaned));
            }
        }
    }
    names
}

fn is_check_name(word: &Text) -> Answer {
    (!word.as_str().is_empty()
        && word
            .as_str()
            .chars()
            .all(|found| found.is_ascii_lowercase() || found == '-'))
    .answer()
}

/// The check names `ci/checks.sh` answers to: the labels of its `case`.
fn known_check_names(script: &Text) -> Vec<Name> {
    script
        .as_str()
        .lines()
        .filter_map(|line| {
            let (label, _) = line.trim_start().split_once(')')?;
            (is_check_name(&Text::of(label)) == Answer::Yes).then(|| Name::of(label))
        })
        .collect()
}

#[test]
fn workflows_use_only_checks_that_exist() {
    assert_eq!(
        check_workflows_use_only_checks_that_exist(),
        Ok(()),
        "workflows_use_only_checks_that_exist"
    );
}

fn check_workflows_use_only_checks_that_exist() -> Result<(), Fail> {
    let known = known_check_names(&io::read_text(Path::new("../ci/checks.sh"))?);
    for wanted in [
        "audit-workflows",
        "canaries",
        "clippy",
        "clippy-main",
        "clippy-policy",
        "deny",
        "deny-advisories",
        "fmt",
        "owntree",
        "package",
        "policytest",
        "selftest",
        "test",
    ] {
        ensure!(
            known.iter().any(|name| name.as_str() == wanted),
            "ci/checks.sh has the check {wanted}: {known:?}"
        );
    }
    let mut used_anywhere = Count::new(0);
    for item in workflows()? {
        for name in used_check_names(&item.text) {
            used_anywhere = used_anywhere.next()?;
            ensure!(
                known.contains(&name),
                "{} runs the check {}, which ci/checks.sh does not have: {known:?}",
                item.name.as_str(),
                name.as_str()
            );
        }
    }
    ensure!(used_anywhere.get() > 0, "some workflow calls ci/checks.sh");
    Ok(())
}

#[test]
fn push_jobs_and_head_checks_run_every_check() {
    assert_eq!(
        check_push_jobs_and_head_checks_run_every_check(),
        Ok(()),
        "push_jobs_and_head_checks_run_every_check"
    );
}

fn check_push_jobs_and_head_checks_run_every_check() -> Result<(), Fail> {
    let known = known_check_names(&io::read_text(Path::new("../ci/checks.sh"))?);
    let mut on_push = Vec::new();
    for job in PUSH_JOBS {
        on_push.extend(used_check_names(&policy_job(&Name::of(job))?.body));
    }
    let mut on_pull_request = used_check_names(&policy_job(&Name::of("head-checks"))?.body);
    if on_pull_request.iter().any(|name| name.as_str() == "clippy") {
        on_pull_request.push(Name::of("clippy-main"));
        on_pull_request.push(Name::of("clippy-policy"));
    }
    for name in known.iter().filter(|name| name.as_str() != "clippy") {
        ensure!(on_push.contains(name), "a push job runs {}", name.as_str());
        if NETWORK_CHECKS.contains(&name.as_str()) {
            ensure!(
                !on_pull_request.contains(name),
                "head-checks runs offline and must not run {}",
                name.as_str()
            );
        } else {
            ensure!(
                on_pull_request.contains(name),
                "head-checks runs {}",
                name.as_str()
            );
        }
    }
    Ok(())
}

#[test]
fn checks_script_keeps_its_rules() {
    assert_eq!(
        check_checks_script_keeps_its_rules(),
        Ok(()),
        "checks_script_keeps_its_rules"
    );
}

fn check_checks_script_keeps_its_rules() -> Result<(), Fail> {
    let script = io::read_text(Path::new("../ci/checks.sh"))?;
    for wanted in [
        "set -eu",
        "sh \"$here/cargo.sh\" \"$@\"",
        "--no-verify",
        "--target-dir \"$scratch/target\"",
        "RUNNER_TEMP",
        "unknown check",
    ] {
        ensure!(
            script.as_str().contains(wanted),
            "ci/checks.sh contains {wanted}"
        );
    }
    for line in script.as_str().lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            continue;
        }
        ensure!(
            !trimmed.split_whitespace().any(|word| word == "cargo"),
            "ci/checks.sh: `{trimmed}` calls cargo directly; use ci/cargo.sh"
        );
        ensure!(
            !trimmed.contains("CARGO_TARGET_DIR") && !trimmed.contains("CARGO_HOME"),
            "ci/checks.sh: `{trimmed}` names a cargo environment variable"
        );
    }
    for item in workflows()? {
        ensure!(
            !item.text.as_str().contains("CARGO_TARGET_DIR"),
            "{} sets no CARGO_TARGET_DIR",
            item.name.as_str()
        );
    }
    Ok(())
}

#[test]
fn checks_script_refuses_what_it_does_not_know() {
    assert_eq!(
        check_checks_script_refuses_what_it_does_not_know(),
        Ok(()),
        "checks_script_refuses_what_it_does_not_know"
    );
}

fn check_checks_script_refuses_what_it_does_not_know() -> Result<(), Fail> {
    let program = Name::of("sh");
    for args in [
        Args::of(&["../ci/checks.sh", "no-such-check"]),
        Args::of(&["../ci/checks.sh"]),
    ] {
        let captured = process::run(&Invocation::new(&program, &args))?;
        ensure_eq!(
            captured.status,
            Status::Code(Count::new(2)),
            "an unknown or missing name exits 2"
        );
    }
    for script in ["../ci/checks.sh", "../ci/cargo.sh"] {
        let captured = process::run(&Invocation::new(&program, &Args::of(&[script, "--help"])))?;
        ensure_eq!(
            captured.status,
            Status::Code(Count::new(0)),
            "--help prints the usage and exits 0"
        );
    }
    Ok(())
}

#[test]
fn the_version_script_reads_only_the_package_table() {
    assert_eq!(
        check_the_version_script_reads_only_the_package_table(),
        Ok(()),
        "the_version_script_reads_only_the_package_table"
    );
}

fn check_the_version_script_reads_only_the_package_table() -> Result<(), Fail> {
    let program = Name::of("sh");
    let args = Args::of(&["../ci/version.sh"]);
    for (manifest, expected, code) in VERSION_SCRIPT_CASES {
        let captured = process::run(
            &Invocation::new(&program, &args).feeding(&Bytes::of_text(&Text::of(manifest))),
        )?;
        ensure_eq!(
            captured.status,
            Status::Code(Count::new(code)),
            "the exit status for {manifest:?}"
        );
        ensure_eq!(
            captured.stdout.lossy_text(),
            Text::of(expected),
            "the version printed for {manifest:?}"
        );
    }
    Ok(())
}

#[test]
fn the_package_check_passes_allow_dirty_only_to_cargo_package() {
    assert_eq!(
        check_the_package_check_passes_allow_dirty_only_to_cargo_package(),
        Ok(()),
        "the_package_check_passes_allow_dirty_only_to_cargo_package"
    );
}

fn check_the_package_check_passes_allow_dirty_only_to_cargo_package() -> Result<(), Fail> {
    let script = io::read_text(Path::new("../ci/checks.sh"))?;
    let lines = code_lines(&script);
    let flagged: Vec<&Text> = lines
        .iter()
        .filter(|line| line.as_str().contains("--allow-dirty"))
        .collect();
    ensure_eq!(
        flagged.len(),
        2,
        "two code lines name --allow-dirty: the package listing and the package"
    );
    ensure!(
        flagged
            .iter()
            .all(|line| line.as_str().contains("cargo_sh package ")),
        "--allow-dirty goes to cargo package alone"
    );
    ensure!(
        script.as_str().contains("git status --porcelain"),
        "--allow-dirty is passed only when the tree is dirty"
    );
    Ok(())
}

#[test]
fn the_package_check_refuses_stray_files() {
    assert_eq!(
        check_the_package_check_refuses_stray_files(),
        Ok(()),
        "the_package_check_refuses_stray_files"
    );
}

fn check_the_package_check_refuses_stray_files() -> Result<(), Fail> {
    let script = io::read_text(Path::new("../ci/checks.sh"))?;
    let text: String = code_lines(&script)
        .iter()
        .map(|line| line.as_str().to_owned())
        .collect::<Vec<String>>()
        .join("\n");
    for wanted in [
        "refuse_junk_in_package() {",
        "\\.bak|\\.orig|\\.rej|\\.swp|~)$",
        "(^|/)\\.DS_Store$",
        "^src/.*[^/]$",
        "grep -Ev '^src/.*\\.rs$|^Cargo\\.toml\\.orig$'",
        "cargo_sh package -p unrepresentable --list --allow-dirty \"$@\" | refuse_junk_in_package",
        "cargo_sh package -p unrepresentable --list \"$@\" | refuse_junk_in_package",
    ] {
        ensure!(text.contains(wanted), "checks.sh contains {wanted}");
    }
    Ok(())
}

#[test]
fn release_verify_gates_on_green_ci() {
    assert_eq!(
        check_release_verify_gates_on_green_ci(),
        Ok(()),
        "release_verify_gates_on_green_ci"
    );
}

fn check_release_verify_gates_on_green_ci() -> Result<(), Fail> {
    let release = workflow_named(&Name::of("release.yml"))?;
    let verify = jobs_of(&release)?
        .into_iter()
        .find(|job| job.name.as_str() == "verify")
        .ok_or_else(|| Fail::new(&Message::of("release.yml has no verify job")))?;
    let text = verify.body.as_str();
    for wanted in [
        "      actions: read\n",
        "      checks: read\n",
        "      contents: read\n",
        "      pull-requests: read\n",
        "run: sh ci/cargo.sh build -p unrepresentable-policy",
        "GH_TOKEN: ${{ github.token }}",
        "target/release/unrepresentable-policy release-gate \"$GITHUB_SHA\"",
        "is trusted code",
    ] {
        ensure!(
            text.contains(wanted),
            "release.yml verify contains {wanted}"
        );
    }
    ensure!(
        !text.contains("write"),
        "release.yml verify has no write permission"
    );
    let position = |needle: &str| text.find(needle);
    let ancestry = position("merge-base --is-ancestor");
    let build = position("ci/cargo.sh build -p unrepresentable-policy");
    let gate = position("release-gate");
    ensure!(
        ancestry.is_some() && ancestry < build && build < gate,
        "the gate runs after the ancestry check and the build"
    );
    Ok(())
}

#[test]
fn the_release_gate_names_jobs_that_exist() {
    assert_eq!(
        check_the_release_gate_names_jobs_that_exist(),
        Ok(()),
        "the_release_gate_names_jobs_that_exist"
    );
}

fn check_the_release_gate_names_jobs_that_exist() -> Result<(), Fail> {
    let policy = workflow_named(&Name::of("policy.yml"))?;
    ensure!(
        policy
            .text
            .as_str()
            .lines()
            .any(|line| line == format!("name: {WORKFLOW_NAME}")),
        "policy.yml is named {WORKFLOW_NAME}"
    );
    ensure!(
        WORKFLOW_PATH == ".github/workflows/policy.yml",
        "the gate looks for policy.yml"
    );
    let jobs = jobs_of(&policy)?;
    for wanted in REQUIRED_JOBS.iter().chain(once(&SKIP_JOB)) {
        let job = jobs
            .iter()
            .find(|found| found.name.as_str() == *wanted)
            .ok_or_else(|| Fail::new(&Message::from_string(format!("no job {wanted}"))))?;
        ensure!(
            job.body.as_str().contains(&format!("name: {wanted}\n")),
            "job {wanted} has the display name {wanted}, which its check run carries"
        );
        ensure!(
            job.body
                .as_str()
                .contains("github.event_name != 'pull_request_target'")
                || job.body.as_str().contains("github.event_name == 'push'"),
            "job {wanted} runs on pushes"
        );
    }
    for job in REQUIRED_JOBS {
        ensure!(PUSH_JOBS.contains(&job), "{job} is a push job");
    }
    for job in PUSH_JOBS {
        ensure!(
            job == SKIP_JOB || REQUIRED_JOBS.contains(&job),
            "the gate requires the push job {job}"
        );
    }
    Ok(())
}

#[test]
fn every_installed_tool_is_run_with_its_directory_on_the_path() {
    assert_eq!(
        check_installed_tools_are_on_the_path(),
        Ok(()),
        "every_installed_tool_is_run_with_its_directory_on_the_path"
    );
}

fn check_installed_tools_are_on_the_path() -> Result<(), Fail> {
    for item in workflows()? {
        for line in item.text.as_str().lines() {
            let runs_tool = line.contains("checks.sh deny")
                || line.contains("checks.sh audit-workflows")
                || line.contains("checks.sh\" audit-workflows");
            if runs_tool {
                ensure!(
                    line.contains("PATH=\"$RUNNER_TEMP/bin:$PATH\""),
                    "{}: a check that needs an installed tool sets PATH inline: {}",
                    item.name.as_str(),
                    line.trim()
                );
            }
        }
    }
    Ok(())
}

#[test]
fn advisories_run_with_the_network_and_every_deny_job_installs_the_pinned_tool() {
    assert_eq!(
        check_advisories_and_pinned_cargo_deny(),
        Ok(()),
        "advisories_run_with_the_network_and_every_deny_job_installs_the_pinned_tool"
    );
}

fn check_advisories_and_pinned_cargo_deny() -> Result<(), Fail> {
    let deny = policy_job(&Name::of("deny"))?.body;
    ensure!(
        deny.as_str().contains("checks.sh deny\n")
            && deny.as_str().contains("checks.sh deny-advisories"),
        "the push job `deny` runs both deny checks"
    );
    let sweep = jobs_of(&workflow_named(&Name::of("policy-sweep.yml"))?)?;
    let advisories = sweep
        .iter()
        .find(|job| job.name.as_str() == "advisories")
        .ok_or_else(|| Fail::new(&Message::of("policy-sweep.yml has no advisories job")))?;
    ensure!(
        advisories
            .body
            .as_str()
            .contains("checks.sh deny-advisories"),
        "the sweep checks advisories"
    );
    ensure!(
        !advisories.body.as_str().contains("secrets.")
            && !advisories.body.as_str().contains("environment:"),
        "the advisories job holds no secrets and no environment"
    );
    let head_checks = policy_job(&Name::of("head-checks"))?.body;
    ensure!(
        !head_checks.as_str().contains("deny-advisories"),
        "the offline head-checks never run the network-only advisories check"
    );
    for item in workflows()? {
        ensure!(
            !item
                .text
                .as_str()
                .lines()
                .any(|line| line.contains("deny-advisories") && line.contains("--offline")),
            "{} runs advisories with --offline, which cannot work",
            item.name.as_str()
        );
        for job in jobs_of(&item)? {
            let runs_deny = job.body.as_str().lines().any(|line| {
                line.contains("checks.sh deny")
                    || line.contains("CHECKS\" deny")
                    || line.trim() == "check deny"
            });
            if runs_deny {
                ensure!(
                    job.body.as_str().contains("install-tool")
                        && job.body.as_str().contains("tools.lock")
                        && job.body.as_str().contains("cargo-deny"),
                    "{}: job {} runs cargo-deny but does not install the pinned one",
                    item.name.as_str(),
                    job.name.as_str()
                );
            }
        }
    }
    Ok(())
}

/// The text of the job `job` in the workflow `item`.
fn job_text(item: &Workflow, job: &Name) -> Result<Text, Fail> {
    jobs_of(item)?
        .into_iter()
        .find(|found| found.name == *job)
        .map(|found| found.body)
        .ok_or_else(|| {
            Fail::new(&Message::from_string(format!(
                "{} has no job {}",
                item.name.as_str(),
                job.as_str()
            )))
        })
}

/// The lines of `body` that are not comments.
fn code_lines(body: &Text) -> Vec<Text> {
    body.as_str()
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .map(Text::of)
        .collect()
}

#[test]
fn head_code_runs_only_as_the_unprivileged_user() {
    assert_eq!(
        check_head_code_runs_only_as_the_unprivileged_user(),
        Ok(()),
        "head_code_runs_only_as_the_unprivileged_user"
    );
}

fn check_head_code_runs_only_as_the_unprivileged_user() -> Result<(), Fail> {
    for job in HEAD_JOBS {
        let body = policy_job(&Name::of(job))?.body;
        ensure!(
            body.as_str()
                .contains("base/policy/stage-sandbox.sh /opt/sandbox"),
            "{job} stages the sandbox from the base checkout"
        );
        ensure!(
            body.as_str().contains("base/policy/offline.sh"),
            "{job} runs head code through offline.sh"
        );
        for line in code_lines(&body) {
            let shown = line.as_str();
            if line.as_str().contains("ci/cargo.sh fetch") {
                ensure!(
                    line.as_str().contains("cd base &&"),
                    "{job}: `{shown}` fetches crates with the head as the working directory, where its cargo configuration would be read"
                );
                ensure!(
                    !line.as_str().contains("cd head"),
                    "{job}: `{shown}` runs cargo in head/"
                );
            }
            if line.as_str().contains("cd head") {
                ensure!(
                    job == "head-checks" && line.as_str().contains("audit-workflows"),
                    "{job}: `{shown}` runs something in the head checkout; only zizmor does, as data"
                );
            }
            ensure!(
                !line.as_str().contains("RUSTUP_HOME")
                    && !line
                        .as_str()
                        .contains("CARGO_HOME=\"$RUNNER_TEMP/cargo\" >> "),
                "{job}: `{shown}` hands the runner's rustup or cargo home to later steps"
            );
        }
    }
    Ok(())
}

#[test]
fn the_sandbox_scripts_drop_privileges_and_the_network() {
    assert_eq!(
        check_the_sandbox_scripts_drop_privileges_and_the_network(),
        Ok(()),
        "the_sandbox_scripts_drop_privileges_and_the_network"
    );
}

fn check_the_sandbox_scripts_drop_privileges_and_the_network() -> Result<(), Fail> {
    let sandbox = io::read_text(Path::new("../policy/sandbox.sh"))?;
    for wanted in [
        "unshare --net --pid --fork --kill-child --mount-proc",
        "ip link set lo up",
        "--reuid=\"$uid\"",
        "--regid=\"$uid\"",
        "--clear-groups",
        "--no-new-privs",
        "env -i",
        "$dir/toolchain/bin",
    ] {
        ensure!(
            sandbox.as_str().contains(wanted),
            "sandbox.sh contains {wanted}"
        );
    }
    for line in code_lines(&sandbox) {
        let shown = line.as_str();
        ensure!(
            !line.as_str().contains("id -u") && !line.as_str().contains("RUSTUP"),
            "sandbox.sh: `{shown}` runs as the caller or hands rustup to the code"
        );
    }
    let offline = io::read_text(Path::new("../policy/offline.sh"))?;
    ensure!(
        offline.as_str().contains("SANDBOX_DIR") && offline.as_str().contains("sandbox.sh"),
        "offline.sh is sandbox.sh with a staged directory required"
    );
    ensure!(
        !offline.as_str().contains("id -u") && !offline.as_str().contains("--reuid=\"$"),
        "offline.sh does not run head code as the runner user"
    );
    let stage = io::read_text(Path::new("../policy/stage-sandbox.sh"))?;
    for wanted in [
        "rustc --print sysroot",
        "chmod -R go+rX,go-w \"$dir/toolchain\"",
        "\"$ci/checks.sh\" \"$ci/cargo.sh\" \"$ci/version.sh\"",
        "chown -R 65534:65534",
        "--private-bin)",
    ] {
        ensure!(
            stage.as_str().contains(wanted),
            "stage-sandbox.sh contains {wanted}"
        );
    }
    let take = io::read_text(Path::new("../policy/take-file.sh"))?;
    ensure!(
        take.as_str().contains("outside $root"),
        "take-file.sh refuses a link out of the sandbox"
    );
    Ok(())
}

#[test]
fn every_sandboxed_command_has_its_own_pid_namespace_and_is_reaped() {
    assert_eq!(
        check_every_sandboxed_command_has_its_own_pid_namespace_and_is_reaped(),
        Ok(()),
        "every_sandboxed_command_has_its_own_pid_namespace_and_is_reaped"
    );
}

fn check_every_sandboxed_command_has_its_own_pid_namespace_and_is_reaped() -> Result<(), Fail> {
    let sandbox = io::read_text(Path::new("../policy/sandbox.sh"))?;
    let code: Vec<String> = code_lines(&sandbox)
        .iter()
        .map(|line| line.as_str().to_owned())
        .collect();
    let text = code.join("\n");
    for wanted in [
        "unshare --net --pid --fork --kill-child --mount-proc",
        "/usr/sbin/ip link set lo up",
        "sudo -n /usr/bin/pkill -KILL -u \"$uid\"",
        "pgrep -u \"$uid\"",
        "exit \"$status\"",
    ] {
        ensure!(text.contains(wanted), "sandbox.sh contains {wanted}");
    }
    ensure!(
        !text.contains("exec sudo"),
        "sandbox.sh does not exec the sandbox: it must stay to reap what is left"
    );
    let at = |needle: &str| {
        text.find(needle)
            .ok_or_else(|| Fail::new(&Message::from_string(format!("no {needle}"))))
    };
    ensure!(
        at("ip link set lo up")? < at("--reuid=")?,
        "the loopback interface is brought up before privileges are dropped"
    );
    ensure!(
        at("pkill -KILL")? > at("unshare")?,
        "processes are killed after the command, before the script returns"
    );
    // every head job goes through offline.sh, which is sandbox.sh
    for job in HEAD_JOBS {
        ensure!(
            policy_job(&Name::of(job))?
                .body
                .as_str()
                .contains("base/policy/offline.sh"),
            "{job} runs head code only through offline.sh, so every call is reaped"
        );
    }
    Ok(())
}

#[test]
fn the_head_binary_runs_as_another_user_than_the_base_binary() {
    assert_eq!(
        check_the_head_binary_runs_as_another_user_than_the_base_binary(),
        Ok(()),
        "the_head_binary_runs_as_another_user_than_the_base_binary"
    );
}

fn check_the_head_binary_runs_as_another_user_than_the_base_binary() -> Result<(), Fail> {
    let sandbox = io::read_text(Path::new("../policy/sandbox.sh"))?;
    ensure!(
        sandbox.as_str().contains("65534 | 65533) ;;")
            && sandbox.as_str().contains("SANDBOX_UID:-65534")
            && sandbox.as_str().contains("SANDBOX_SCRATCH"),
        "sandbox.sh runs as 65534 or 65533 and gives the scratch directory to that user"
    );
    ensure!(
        sandbox.as_str().contains("chmod 0700 \"$scratch\"")
            && sandbox
                .as_str()
                .contains("chown -R \"$uid:$uid\" \"$scratch\""),
        "the scratch directory is given to the user with mode 0700"
    );
    let stage = io::read_text(Path::new("../policy/stage-sandbox.sh"))?;
    ensure!(
        stage
            .as_str()
            .contains("install -d -m 0750 -o root -g \"$uid\" \"$dir/private-$uid\"")
            && stage.as_str().contains(
                "install -m 0750 -o root -g \"$uid\" \"$src\" \"$dir/private-$uid/$name\""
            ),
        "a private binary is readable and executable by its own user only"
    );
    ensure!(
        process::BASE_UID != process::HEAD_UID,
        "the two binaries have two users"
    );
    for job in ["differential", "differential-hidden"] {
        let body = policy_job(&Name::of(job))?.body;
        ensure!(
            !body.as_str().contains("/opt/sandbox/bin/"),
            "{job} puts no binary on the common PATH"
        );
    }
    Ok(())
}

#[test]
fn take_file_opens_the_file_once_and_checks_the_open_file() {
    assert_eq!(
        check_take_file_opens_the_file_once_and_checks_the_open_file(),
        Ok(()),
        "take_file_opens_the_file_once_and_checks_the_open_file"
    );
}

fn check_take_file_opens_the_file_once_and_checks_the_open_file() -> Result<(), Fail> {
    let take = io::read_text(Path::new("../policy/take-file.sh"))?;
    let text: String = code_lines(&take)
        .iter()
        .map(|line| line.as_str().to_owned())
        .collect::<Vec<String>>()
        .join("\n");
    for wanted in [
        "exec 3<\"$src\"",
        "readlink -- \"$open\"",
        "[ -f \"$open\" ]",
        "head -c \"$((limit + 1))\" <&3",
        "limit=268435456",
        "outside $root",
        "[ -f \"$src\" ] && [ ! -L \"$src\" ] && [ ! -p \"$src\" ]",
        "exec timeout 120 env TAKE_FILE_BOUNDED=1 sh \"$0\" \"$@\"",
    ] {
        ensure!(text.contains(wanted), "take-file.sh contains {wanted}");
    }
    let check = text.find("[ ! -p \"$src\" ]");
    let open = text.find("exec 3<\"$src\"");
    ensure!(
        matches!((check, open), (Some(before), Some(after)) if before < after),
        "the type check comes before the open"
    );
    ensure!(
        text.matches("<\"$src\"").count() == 1
            && !text.contains("cp ")
            && !text.contains("realpath -e -- \"$src\""),
        "the source path is opened once and never copied by path"
    );
    Ok(())
}

#[test]
fn every_zizmor_invocation_disables_configuration_discovery() {
    assert_eq!(
        check_every_zizmor_invocation_disables_configuration_discovery(),
        Ok(()),
        "every_zizmor_invocation_disables_configuration_discovery"
    );
}

fn check_every_zizmor_invocation_disables_configuration_discovery() -> Result<(), Fail> {
    let mut texts: Vec<(String, Text)> = Vec::new();
    for item in workflows()? {
        texts.push((item.name.as_str().to_owned(), item.text));
    }
    for script in ["../ci/checks.sh", "../ci/unrepresentable.sh"] {
        texts.push((script.to_owned(), io::read_text(Path::new(script))?));
    }
    let mut seen = Count::new(0);
    for (name, text) in texts {
        for line in code_lines(&text) {
            let shown = line.as_str();
            let invoked = shown.contains("zizmor -") || shown.contains("zizmor\" -");
            if invoked {
                seen = seen.next()?;
                ensure!(
                    !shown.contains("--no-config"),
                    "{name}: `{shown}` runs zizmor with --no-config, which would report the two designed triggers"
                );
                ensure!(
                    shown.contains("--config ") && shown.contains("policy/zizmor.yml"),
                    "{name}: `{shown}` runs zizmor without --config of policy/zizmor.yml, so the checked tree could switch audits off with its own configuration"
                );
                ensure!(
                    !shown.contains("head/policy") && !shown.contains("head/.github/zizmor"),
                    "{name}: `{shown}` takes the zizmor config from the head"
                );
            }
        }
    }
    ensure!(
        seen.get() >= 2,
        "found the zizmor invocations ({})",
        seen.get()
    );
    Ok(())
}

#[test]
fn the_zizmor_config_ignores_only_the_two_designed_triggers() {
    assert_eq!(
        check_the_zizmor_config_ignores_only_the_two_designed_triggers(),
        Ok(()),
        "the_zizmor_config_ignores_only_the_two_designed_triggers"
    );
}

fn check_the_zizmor_config_ignores_only_the_two_designed_triggers() -> Result<(), Fail> {
    let config = io::read_text(Path::new("zizmor.yml"))?;
    let entries: Vec<String> = code_lines(&config)
        .iter()
        .map(|line| line.as_str().trim().to_owned())
        .filter(|line| !line.is_empty())
        .collect();
    let expected = [
        "rules:",
        "dangerous-triggers:",
        "ignore:",
        "- policy-review.yml:4:1",
        "- policy.yml:28:1",
    ];
    ensure_eq!(
        entries,
        expected
            .iter()
            .map(|line| (*line).to_owned())
            .collect::<Vec<_>>(),
        "policy/zizmor.yml ignores exactly two findings, each at file:line:column"
    );
    // the two ignored positions are the `on:` keys of the two workflows
    for (file, skipped) in [("policy-review.yml", 3), ("policy.yml", 27)] {
        let path = format!("../.github/workflows/{file}");
        let text = io::read_text(Path::new(&path))?;
        let at = text.as_str().lines().nth(skipped);
        ensure_eq!(at, Some("on:"), "{file}: the ignored line is the `on:` key");
    }
    Ok(())
}

#[test]
fn no_workflow_writes_to_github_env_or_github_path() {
    assert_eq!(
        check_no_workflow_writes_to_github_env_or_github_path(),
        Ok(()),
        "no_workflow_writes_to_github_env_or_github_path"
    );
}

fn check_no_workflow_writes_to_github_env_or_github_path() -> Result<(), Fail> {
    let mut outputs = Count::new(0);
    for item in workflows()? {
        let name = item.name.as_str().to_owned();
        for line in code_lines(&item.text) {
            let shown = line.as_str();
            ensure!(
                !shown.contains("GITHUB_ENV") && !shown.contains("GITHUB_PATH"),
                "{name}: `{shown}` writes to the environment or PATH of later steps (zizmor github-env)"
            );
            if shown.contains("channel=$CHANNEL") {
                outputs = outputs.next()?;
            }
        }
        // every toolchain step validates the channel before the output is written
        let text = item.text.as_str();
        let writes = text.matches("channel=$CHANNEL").count();
        let checks = text
            .matches("grep -Eq '^[0-9]+\\.[0-9]+\\.[0-9]+$'")
            .count();
        ensure_eq!(writes, checks, "{name}: every channel output is validated");
        let uses = text
            .matches("RUSTUP_TOOLCHAIN: ${{ steps.toolchain.outputs.channel }}")
            .count();
        ensure!(
            writes == 0 || uses >= 1,
            "{name}: the channel output is passed to a later step through env"
        );
    }
    ensure!(
        outputs.get() >= 6,
        "found the channel outputs ({})",
        outputs.get()
    );
    Ok(())
}

#[test]
fn owntree_fails_on_exit_1_and_2_and_passes_on_exit_3() {
    assert_eq!(
        check_owntree_fails_on_exit_1_and_2_and_passes_on_exit_3(),
        Ok(()),
        "owntree_fails_on_exit_1_and_2_and_passes_on_exit_3"
    );
}

fn check_owntree_fails_on_exit_1_and_2_and_passes_on_exit_3() -> Result<(), Fail> {
    let script = io::read_text(Path::new("../ci/checks.sh"))?;
    let text = script.as_str();
    let Some((_, rest)) = text.split_once("check_owntree() {") else {
        return Err(Fail::new(&Message::of("ci/checks.sh has check_owntree")));
    };
    let body = match rest.split("\n}\n").next() {
        Some(found) => found,
        None => rest,
    };
    for wanted in [
        "status=0",
        "|| status=$?",
        "case \"$status\" in",
        "  0) ;;",
        "  3) printf",
        "  *) exit \"$status\" ;;",
    ] {
        ensure!(body.contains(wanted), "check_owntree contains {wanted}");
    }
    // 3 is the only nonzero status that is let through
    ensure!(
        !body.contains("  1)")
            && !body.contains("  2)")
            && !body.contains("1 | 3")
            && !body.contains("| 3)"),
        "exit 1 and 2 are not matched by a passing arm"
    );
    ensure!(
        body.matches("exit \"$status\"").count() == 1 && !body.contains("exit 0"),
        "every status but 0 and 3 is the check's own exit status"
    );
    Ok(())
}

#[test]
fn differential_inputs_from_the_pull_request_never_meet_the_secret() {
    assert_eq!(
        check_differential_inputs_from_the_pull_request_never_meet_the_secret(),
        Ok(()),
        "differential_inputs_from_the_pull_request_never_meet_the_secret"
    );
}

fn check_differential_inputs_from_the_pull_request_never_meet_the_secret() -> Result<(), Fail> {
    let open = policy_job(&Name::of("differential"))?.body;
    let hidden = policy_job(&Name::of("differential-hidden"))?.body;
    ensure!(
        !open.as_str().contains("environment:") && !open.as_str().contains("secrets."),
        "the job that evaluates the head's inputs has no environment and no secret"
    );
    ensure!(
        open.as_str().contains("--head-input fixtures=")
            && !open.as_str().contains("--hidden-input")
            && !open.as_str().contains("fetch-hidden"),
        "the head's fixtures are a --head-input, and the hidden corpus is not here"
    );
    ensure_eq!(
        hidden
            .as_str()
            .matches("environment: hidden-corpus")
            .count(),
        1,
        "the hidden corpus environment belongs to differential-hidden"
    );
    ensure_eq!(
        workflow_named(&Name::of("policy.yml"))?
            .text
            .as_str()
            .matches("environment: hidden-corpus")
            .count(),
        1,
        "no other job has the hidden corpus environment"
    );
    check_hidden_job_never_touches_the_head(&hidden)?;
    for (job, body) in [("differential", &open), ("differential-hidden", &hidden)] {
        ensure!(
            body.as_str()
                .contains("--prefix \"$GITHUB_WORKSPACE/base/policy/sandbox.sh\"")
                && body.as_str().contains("export SANDBOX_DIR=/opt/sandbox"),
            "{job} runs both binaries through sandbox.sh"
        );
        check_differential_binaries_stay_apart(&Name::of(job), body)?;
    }
    Ok(())
}

/// The two binaries of a differential job are staged where only their own
/// user can reach them, and the runner's copy is hidden.
fn check_differential_binaries_stay_apart(which: &Name, body: &Text) -> Result<(), Fail> {
    let job = which.as_str();
    ensure!(
        body.as_str()
            .contains("--private-bin 65534:base/target/release/unrepresentable=unrepresentable")
            && body
                .as_str()
                .contains("--private-bin 65533:head-binary/unrepresentable=unrepresentable")
            && !body.as_str().contains("--bin ")
            && !body.as_str().contains("/usr/local/bin"),
        "{job} stages each binary in a directory only its own user can search"
    );
    ensure!(
        body.as_str()
            .contains("--base-bin /opt/sandbox/private-65534/unrepresentable")
            && body
                .as_str()
                .contains("--head-bin /opt/sandbox/private-65533/unrepresentable"),
        "{job} runs each binary from its own directory"
    );
    ensure!(
        body.as_str()
            .contains("run: chmod 0700 \"$GITHUB_WORKSPACE\""),
        "{job} hides the runner's copy of the base binary from the sandbox users"
    );
    Ok(())
}

fn check_hidden_job_never_touches_the_head(hidden: &Text) -> Result<(), Fail> {
    for forbidden in [
        "fetch-head",
        "head/",
        "git -C head",
        "--head-input",
        "--input ",
        "HEAD_SHA\" tests",
    ] {
        ensure!(
            !code_lines(hidden)
                .iter()
                .any(|line| line.as_str().contains(forbidden)),
            "differential-hidden never touches the head: it contains {forbidden}"
        );
    }
    ensure!(
        hidden.as_str().contains("--hidden-input hidden="),
        "the hidden corpus is a --hidden-input"
    );
    let secrets: Vec<&str> = hidden
        .as_str()
        .split("secrets.")
        .skip(1)
        .filter_map(|rest| rest.split_whitespace().next())
        .collect();
    ensure!(
        secrets
            .iter()
            .all(|name| name.starts_with("HIDDEN_CORPUS_URL")
                || name.starts_with("HIDDEN_CORPUS_TOKEN")),
        "differential-hidden reads only the corpus secrets: {secrets:?}"
    );
    Ok(())
}

#[test]
fn self_lint_runs_the_base_binary_in_the_sandbox() {
    assert_eq!(
        check_self_lint_runs_the_base_binary_in_the_sandbox(),
        Ok(()),
        "self_lint_runs_the_base_binary_in_the_sandbox"
    );
}

fn check_self_lint_runs_the_base_binary_in_the_sandbox() -> Result<(), Fail> {
    let body = policy_job(&Name::of("self-lint"))?.body;
    for wanted in [
        "self-lint --prefix \"$GITHUB_WORKSPACE/base/policy/offline.sh\"",
        "/opt/sandbox/bin/unrepresentable /opt/sandbox/work/head base",
        "--head-as-caller head",
        "export SANDBOX_DIR=/opt/sandbox",
        "(cd base && sh ci/cargo.sh fetch --manifest-path",
    ] {
        ensure!(
            body.as_str().contains(wanted),
            "self-lint contains {wanted}"
        );
    }
    for line in code_lines(&body) {
        let shown = line.as_str();
        ensure!(
            !line
                .as_str()
                .contains("self-lint base/target/release/unrepresentable head"),
            "self-lint: `{shown}` runs the base binary on the head outside the sandbox"
        );
    }
    Ok(())
}

#[test]
fn the_expected_checks_are_the_checks_the_workflow_runs() {
    assert_eq!(
        check_the_expected_checks_are_the_checks_the_workflow_runs(),
        Ok(()),
        "the_expected_checks_are_the_checks_the_workflow_runs"
    );
}

fn check_the_expected_checks_are_the_checks_the_workflow_runs() -> Result<(), Fail> {
    let mut run: Vec<String> = used_check_names(&policy_job(&Name::of("head-checks"))?.body)
        .iter()
        .map(|name| name.as_str().to_owned())
        .collect();
    run.sort();
    let mut expected: Vec<String> = EXPECTED_CHECKS
        .iter()
        .map(|name| (*name).to_owned())
        .collect();
    expected.sort();
    ensure_eq!(
        run,
        expected.clone(),
        "head-checks runs each expected check once and nothing else"
    );
    let known = known_check_names(&io::read_text(Path::new("../ci/checks.sh"))?);
    let mut from_script: Vec<String> = known
        .iter()
        .map(|name| name.as_str().to_owned())
        .filter(|name| {
            !NETWORK_CHECKS.contains(&name.as_str())
                && name != "clippy-main"
                && name != "clippy-policy"
        })
        .collect();
    from_script.sort();
    ensure_eq!(
        from_script,
        expected,
        "the expected checks are ci/checks.sh less the network check, with clippy for both clippy checks"
    );
    Ok(())
}

#[test]
fn owntree_is_given_an_explicit_validated_range() {
    assert_eq!(
        check_owntree_is_given_an_explicit_validated_range(),
        Ok(()),
        "owntree_is_given_an_explicit_validated_range"
    );
}

fn check_owntree_is_given_an_explicit_validated_range() -> Result<(), Fail> {
    let script = io::read_text(Path::new("../ci/checks.sh"))?;
    for wanted in [
        "cargo_sh run \"$@\" --quiet -- check --range \"$range\" .",
        "valid_range \"$range\" || die",
        "[0-9a-f]{40}",
        "HEAD~1..HEAD",
    ] {
        ensure!(
            script.as_str().contains(wanted),
            "ci/checks.sh contains {wanted}"
        );
    }
    ensure!(
        !script.as_str().contains("-- .\n}") || script.as_str().contains("--range"),
        "the tool is never run without a range"
    );
    let checks = policy_job(&Name::of("head-checks"))?.body;
    ensure!(
        checks
            .as_str()
            .contains("check owntree --range \"$BASE_SHA...HEAD\""),
        "a pull request's owntree runs over its own range"
    );
    let program = Name::of("sh");
    for range in [
        "main...HEAD",
        "HEAD; echo x",
        "$(id)..HEAD",
        "abc..HEAD",
        "",
    ] {
        let args = Args::of(&["../ci/checks.sh", "owntree", "--range", range]);
        let captured = process::run(&Invocation::new(&program, &args))?;
        ensure_eq!(
            captured.status,
            Status::Code(Count::new(2)),
            "the range '{range}' is refused with exit 2 before cargo runs"
        );
    }
    let bare = process::run(&Invocation::new(
        &program,
        &Args::of(&["../ci/checks.sh", "owntree", "--range"]),
    ))?;
    ensure_eq!(
        bare.status,
        Status::Code(Count::new(2)),
        "--range without a value exits 2"
    );
    Ok(())
}

#[test]
fn only_the_merge_step_has_a_token_that_can_write_contents() {
    assert_eq!(
        check_only_the_merge_step_has_a_token_that_can_write_contents(),
        Ok(()),
        "only_the_merge_step_has_a_token_that_can_write_contents"
    );
}

fn check_only_the_merge_step_has_a_token_that_can_write_contents() -> Result<(), Fail> {
    for item in workflows()? {
        let writes = item
            .text
            .as_str()
            .matches("permission-contents: write")
            .count();
        ensure_eq!(
            writes,
            usize::from(item.name.as_str() == "policy.yml"),
            "{}: tokens that can write contents",
            item.name.as_str()
        );
    }
    let evaluate = policy_job(&Name::of("policy-evaluate"))?.body;
    let text = evaluate.as_str();
    let (decide, merge) = text
        .split_once("Mint merge token")
        .ok_or_else(|| Fail::new(&Message::of("policy-evaluate mints no merge token")))?;
    ensure!(
        !decide.contains("permission-contents: write") && !decide.contains("post merge"),
        "the token that decides cannot write contents and does not merge"
    );
    ensure!(
        decide.contains("steps.app.outputs.token") && !decide.contains("steps.merge"),
        "the decision uses the first token"
    );
    ensure!(
        merge.contains("steps.merge.outputs.token")
            && merge.contains("permission-contents: write")
            && !merge.contains("steps.app.outputs.token")
            && merge.contains("post merge"),
        "the merge step uses the second token, which is minted just before it"
    );
    for file in ["policy-review.yml", "policy-sweep.yml"] {
        let body = workflow_named(&Name::of(file))?.text;
        ensure!(
            body.as_str().contains("permission-checks: write")
                && body.as_str().contains("permission-pull-requests: write")
                && body.as_str().contains("permission-contents: read"),
            "{file} mints checks, pull request and read-only contents access"
        );
    }
    Ok(())
}

#[test]
fn nothing_is_compiled_where_a_secret_or_a_write_token_is() {
    assert_eq!(
        check_nothing_is_compiled_where_a_secret_or_a_write_token_is(),
        Ok(()),
        "nothing_is_compiled_where_a_secret_or_a_write_token_is"
    );
}

fn check_nothing_is_compiled_where_a_secret_or_a_write_token_is() -> Result<(), Fail> {
    for item in workflows()? {
        for job in jobs_of(&item)? {
            let body = job.body.as_str();
            let guarded = body.contains("secrets.")
                || body.contains("environment:")
                || body.contains("contents: write")
                || body.contains("id-token: write");
            if !guarded {
                continue;
            }
            for line in code_lines(&job.body) {
                let shown = line.as_str();
                if line.as_str().contains("ci/cargo.sh") || line.as_str().contains("ci/checks.sh") {
                    ensure!(
                        line.as_str().contains("publish --no-verify"),
                        "{}: job {} holds a secret, an environment or a write token, and compiles: `{shown}`",
                        item.name.as_str(),
                        job.name.as_str()
                    );
                }
            }
        }
    }
    for (file, build, user) in [
        ("policy-review.yml", "build", "review"),
        ("policy-sweep.yml", "build", "sweep"),
        ("settings-audit.yml", "build", "audit"),
    ] {
        let item = workflow_named(&Name::of(file))?;
        let built = job_text(&item, &Name::of(build))?;
        ensure!(
            !built.as_str().contains("secrets.") && !built.as_str().contains("environment:"),
            "{file}: the build job has no secret and no environment"
        );
        let used = job_text(&item, &Name::of(user))?;
        ensure!(
            used.as_str().contains("needs: [build]")
                && used.as_str().contains("needs.build.outputs.policy-sha256")
                && used.as_str().contains("sha256sum -c -")
                && used.as_str().contains("actions/download-artifact@"),
            "{file}: {user} uses the built tool after checking its sha256"
        );
    }
    Ok(())
}

#[test]
fn every_policy_input_job_has_its_result_variable_and_artifact() {
    assert_eq!(
        check_every_policy_input_job_has_its_result_variable_and_artifact(),
        Ok(()),
        "every_policy_input_job_has_its_result_variable_and_artifact"
    );
}

fn check_every_policy_input_job_has_its_result_variable_and_artifact() -> Result<(), Fail> {
    let evaluate = policy_job(&Name::of("policy-evaluate"))?.body;
    for job in Job::ALL {
        let name = job.name();
        let variable = job.result_variable();
        ensure!(
            evaluate.as_str().contains(&format!(
                "{}: ${{{{ needs.{}.result }}}}",
                variable.as_str(),
                name.as_str()
            )),
            "policy-evaluate reads the result of {}",
            name.as_str()
        );
        ensure!(
            evaluate
                .as_str()
                .lines()
                .any(|line| line.trim_start().starts_with("needs: [")
                    && line.contains(name.as_str())),
            "policy-evaluate waits for {}",
            name.as_str()
        );
        let body = policy_job(&Name::of(name.as_str()))?.body;
        ensure!(
            body.as_str()
                .contains(&format!("name: result-{}\n", name.as_str())),
            "{} uploads result-{}",
            name.as_str(),
            name.as_str()
        );
    }
    Ok(())
}

#[test]
fn release_checks_out_the_tagged_commit_and_ships_the_verified_files() {
    assert_eq!(
        check_release_checks_out_the_tagged_commit_and_ships_the_verified_files(),
        Ok(()),
        "release_checks_out_the_tagged_commit_and_ships_the_verified_files"
    );
}

fn check_release_checks_out_the_tagged_commit_and_ships_the_verified_files() -> Result<(), Fail> {
    let release = workflow_named(&Name::of("release.yml"))?;
    for job in jobs_of(&release)? {
        let lines: Vec<&str> = job.body.as_str().lines().collect();
        for (index, line) in lines.iter().enumerate() {
            if line.contains("actions/checkout@") {
                let window: Vec<&str> = lines.iter().skip(index).take(8).copied().collect();
                ensure!(
                    window.join("\n").contains("ref: ${{ github.sha }}"),
                    "release.yml job {} checks out by tag name, not by commit",
                    job.name.as_str()
                );
            }
        }
        if job.body.as_str().contains("actions/checkout@") {
            ensure!(
                job.body
                    .as_str()
                    .contains("test \"$(git rev-parse HEAD)\" = \"$GITHUB_SHA\""),
                "release.yml job {} asserts HEAD is the tagged commit",
                job.name.as_str()
            );
        }
    }
    check_release_jobs()
}

fn check_release_jobs() -> Result<(), Fail> {
    let prepare = job_text(
        &workflow_named(&Name::of("release.yml"))?,
        &Name::of("prepare"),
    )?;
    for wanted in [
        "sha256sum \"dist/unrepresentable-$t\"",
        "sha256sum \"$second\"",
        "cmp -s \"dist/unrepresentable-$t\" \"$second\"",
        "artifacts/bin-$t-2/unrepresentable-$t",
        "the hash reported for $t is not the hash of its binary",
        "sha256sum unrepresentable-* > SHA256SUMS",
    ] {
        ensure!(
            prepare.as_str().contains(wanted),
            "prepare contains {wanted}"
        );
    }
    ensure!(
        !prepare.as_str().contains("environment:") && prepare.as_str().contains("contents: read"),
        "the job that compiles in the release has no environment and read-only contents"
    );
    let assemble = job_text(
        &workflow_named(&Name::of("release.yml"))?,
        &Name::of("assemble"),
    )?;
    ensure!(
        assemble.as_str().contains("sha256sum -c SHA256SUMS")
            && assemble.as_str().contains("release-dist")
            && !code_lines(&assemble)
                .iter()
                .any(|line| line.as_str().contains("cargo")),
        "assemble ships the files prepare made, after checking them again, and compiles nothing"
    );
    let publish = job_text(
        &workflow_named(&Name::of("release.yml"))?,
        &Name::of("publish-crate"),
    )?;
    ensure!(
        publish.as_str().contains("ci/cargo.sh publish --no-verify")
            && !publish.as_str().contains("ci/cargo.sh build")
            && !publish.as_str().contains("ci/cargo.sh package"),
        "the job with the OIDC token publishes without building"
    );
    ensure!(
        assemble.as_str().contains("environment: release")
            && publish.as_str().contains("environment: release"),
        "both jobs that publish are behind the release environment"
    );
    Ok(())
}

#[test]
fn release_accepts_pre_release_tags_and_marks_them() {
    assert_eq!(
        check_release_accepts_pre_release_tags_and_marks_them(),
        Ok(()),
        "release_accepts_pre_release_tags_and_marks_them"
    );
}

fn check_release_accepts_pre_release_tags_and_marks_them() -> Result<(), Fail> {
    let release = workflow_named(&Name::of("release.yml"))?;
    let tag_regex = "grep -Eq '^v(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)(-(0|[1-9][0-9]*|[0-9A-Za-z-]*[A-Za-z-][0-9A-Za-z-]*)(\\.(0|[1-9][0-9]*|[0-9A-Za-z-]*[A-Za-z-][0-9A-Za-z-]*))*)?$'";
    for name in ["verify", "prepare", "assemble"] {
        let job = job_text(&release, &Name::of(name))?;
        ensure!(
            job.as_str().contains(tag_regex),
            "release.yml job {name} validates the tag with the SemVer pre-release regex"
        );
    }
    let assemble = job_text(&release, &Name::of("assemble"))?;
    let text = assemble.as_str();
    ensure!(
        text.matches("--prerelease").count() == 1,
        "--prerelease appears once in assemble"
    );
    let (before, after) = text.split_once("case \"${TAG#v}\" in").ok_or_else(|| {
        Fail::new(&Message::of(
            "assemble decides pre-release from the validated tag",
        ))
    })?;
    ensure!(
        before.contains(tag_regex) && before.contains("sha256sum -c SHA256SUMS"),
        "the pre-release decision comes after the tag and hash checks"
    );
    let (pre_branch, final_branch) = after
        .rsplit_once("*)")
        .ok_or_else(|| Fail::new(&Message::of("assemble has a branch for final versions")))?;
    ensure!(
        pre_branch.contains("*-*)")
            && pre_branch.contains("--prerelease")
            && pre_branch.contains("Alpha release")
            && pre_branch.contains("--verify-tag"),
        "a version with a suffix is released with --prerelease and alpha notes"
    );
    ensure!(
        !final_branch.contains("--prerelease") && final_branch.contains("--verify-tag"),
        "a final version is not marked as a pre-release"
    );
    Ok(())
}

#[test]
fn the_release_gate_gets_the_default_branch() {
    assert_eq!(
        check_the_release_gate_gets_the_default_branch(),
        Ok(()),
        "the_release_gate_gets_the_default_branch"
    );
}

fn check_the_release_gate_gets_the_default_branch() -> Result<(), Fail> {
    let verify = job_text(
        &workflow_named(&Name::of("release.yml"))?,
        &Name::of("verify"),
    )?;
    ensure!(
        verify
            .as_str()
            .contains("DEFAULT_BRANCH: ${{ github.event.repository.default_branch }}\n          GH_TOKEN: ${{ github.token }}"),
        "the release gate step gets DEFAULT_BRANCH, to accept only a run of that branch"
    );
    Ok(())
}

#[test]
fn every_release_runs_a_fresh_advisory_check_in_prepare() {
    assert_eq!(
        check_every_release_runs_a_fresh_advisory_check_in_prepare(),
        Ok(()),
        "every_release_runs_a_fresh_advisory_check_in_prepare"
    );
}

fn check_every_release_runs_a_fresh_advisory_check_in_prepare() -> Result<(), Fail> {
    let release = workflow_named(&Name::of("release.yml"))?;
    let prepare = job_text(&release, &Name::of("prepare"))?;
    ensure!(
        prepare.as_str().contains("checks.sh deny-advisories")
            && prepare
                .as_str()
                .contains("install-tool policy/tools.lock cargo-deny"),
        "prepare installs the pinned cargo-deny and checks advisories"
    );
    ensure!(
        !prepare.as_str().contains("secrets.")
            && !prepare.as_str().contains("environment:")
            && !prepare.as_str().contains("id-token")
            && prepare.as_str().contains("contents: read"),
        "the advisory check runs where there is no secret, no environment and no write token"
    );
    ensure!(
        !code_lines(&prepare)
            .iter()
            .any(|line| line.as_str().contains("deny-advisories")
                && line.as_str().contains("--offline")),
        "advisories run with the network"
    );
    ensure!(
        !code_lines(&prepare)
            .iter()
            .any(|line| line.as_str().contains("continue-on-error")),
        "the advisory check is not allowed to fail quietly"
    );
    Ok(())
}

#[test]
fn the_first_crate_publish_is_documented_and_publish_crate_stays_unconditional() {
    assert_eq!(
        check_the_first_crate_publish_is_documented_and_publish_crate_stays_unconditional(),
        Ok(()),
        "the_first_crate_publish_is_documented_and_publish_crate_stays_unconditional"
    );
}

/// `text` with every run of whitespace replaced by one space, so that a phrase
/// still matches after a formatter rewraps the prose around it.
fn single_spaced(text: &Text) -> Text {
    Text::from_string(
        text.as_str()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" "),
    )
}

fn check_the_first_crate_publish_is_documented_and_publish_crate_stays_unconditional(
) -> Result<(), Fail> {
    for (path, needles) in [(
        "../policy/README.md",
        vec![
            "First publish of the crate (once, by hand)",
            "cargo publish",
            "BEFORE tagging",
            "trusted publisher",
            "re-run only the `publish-crate` job",
        ],
    )] {
        let text = io::read_text(Path::new(path))?;
        for needle in needles {
            ensure!(
                single_spaced(&text).as_str().contains(needle),
                "{path} says `{needle}`"
            );
        }
    }
    let publish = job_text(
        &workflow_named(&Name::of("release.yml"))?,
        &Name::of("publish-crate"),
    )?;
    ensure!(
        !code_lines(&publish)
            .iter()
            .any(|line| line.as_str().trim_start().starts_with("if:")),
        "publish-crate has no condition that could skip it: a skip would accept a version that was published from somewhere else as this release's"
    );
    Ok(())
}

#[test]
fn default_range_is_documented_as_local_only_and_ci_passes_its_own_base() {
    assert_eq!(
        check_default_range_is_documented_as_local_only_and_ci_passes_its_own_base(),
        Ok(()),
        "default_range_is_documented_as_local_only_and_ci_passes_its_own_base"
    );
}

fn check_default_range_is_documented_as_local_only_and_ci_passes_its_own_base() -> Result<(), Fail>
{
    let readme = io::read_text(Path::new("../README.md"))?;
    ensure!(
        single_spaced(&readme)
            .as_str()
            .contains("`default-range` is a convenience for local use, not a trust anchor"),
        "the README says default-range is only a local convenience"
    );
    let configuration = io::read_text(Path::new("../docs/configuration.md"))?;
    ensure!(
        single_spaced(&configuration)
            .as_str()
            .contains("`default-range` is a convenience for local use, not a trust anchor")
            && single_spaced(&configuration)
                .as_str()
                .contains("CI must therefore always pass `--range` or `--against`"),
        "docs/configuration.md says default-range is only a local convenience"
    );
    let installer = io::read_text(Path::new("../ci/unrepresentable.sh"))?;
    ensure!(
        installer.as_str().contains("check --against \"$base_ref\""),
        "the consumer installer passes check --against"
    );
    let checks = io::read_text(Path::new("../ci/checks.sh"))?;
    ensure!(
        checks.as_str().contains("-- check --range \"$range\" ."),
        "owntree passes --range"
    );
    for item in workflows()? {
        ensure!(
            !item.text.as_str().contains("default-range"),
            "{} does not rely on default-range",
            item.name.as_str()
        );
    }
    Ok(())
}

#[test]
fn the_review_workflow_resolves_a_fork_pull_request_by_its_head_commit() {
    assert_eq!(
        check_the_review_workflow_resolves_a_fork_pull_request_by_its_head_commit(),
        Ok(()),
        "the_review_workflow_resolves_a_fork_pull_request_by_its_head_commit"
    );
}

fn check_the_review_workflow_resolves_a_fork_pull_request_by_its_head_commit() -> Result<(), Fail> {
    let review = job_text(
        &workflow_named(&Name::of("policy-review.yml"))?,
        &Name::of("review"),
    )?;
    ensure!(
        review
            .as_str()
            .contains("HEAD_SHA: ${{ github.event.workflow_run.head_sha }}")
            && review
                .as_str()
                .contains("unrepresentable-policy review --head-sha \"$HEAD_SHA\" \"$PR_NUMBER\""),
        "the review passes the head commit next to the pull request number, both as environment values"
    );
    Ok(())
}
