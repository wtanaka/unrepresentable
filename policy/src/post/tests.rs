use super::{
    decide, gather, merge, settings_audit_state, validated_env, AuditState, Clients, DecideInput,
    MergeInput, MergeOutcome, PolicyEnv, RunIds,
};
use crate::artifacts;
use crate::bytes::Bytes;
use crate::data::VerdictData;
use crate::github::Method;
use crate::io::{self, Vars};
use crate::json::Json;
use crate::reason::{BlockerCode, Code, Commits, Counts, Job, Reason};
use crate::testkit::{json_of, Fail, FakeGithub, Scratch};
use crate::types::{Count, Message, Name, RelPath, Sha, Text};
use crate::verdict::compute;
use crate::verdict::{Approvable, PullNumber, Verdict};

macro_rules! audit_runs {
    ($updated:literal, $conclusion:literal) => {
        Ok::<FakeGithub, Fail>(FakeGithub::new().route(
            &Text::of("/runs?branch=main"),
            &json_of(&Text::of(concat!(
                "{\"workflow_runs\":[{\"conclusion\":\"",
                $conclusion,
                "\",\"updated_at\":\"",
                $updated,
                "\"}]}"
            )))?,
        ))
    };
}

/// 2024-02-29T12:00:00Z.
const NOON: usize = 1_709_208_000;

/// A text replacement in one job's artifact.
struct Tweak {
    from: Text,
    job: Job,
    to: Text,
}

fn commits() -> Commits {
    Commits {
        base: sha!('b'),
        head: sha!('a'),
    }
}

fn pull() -> PullNumber {
    PullNumber::from_count(Count::new(7))
}

fn all_success() -> Vars {
    Job::ALL.into_iter().fold(Vars::empty(), |vars, job| {
        vars.with(&job.result_variable(), &Text::of("success"))
    })
}

/// Writes every job's artifact under a scratch directory, applying `target`
/// to the artifact of its job.
fn write_results(scratch: &Scratch, target: Option<&Tweak>) -> Result<(), Fail> {
    for job in Job::ALL {
        let text = artifacts::emit(
            job,
            &commits(),
            &[],
            &Counts::empty().with(&Name::of("files"), Count::new(1)),
        );
        let shown = match target {
            Some(tweak) if tweak.job == job => text
                .as_str()
                .replace(tweak.from.as_str(), tweak.to.as_str()),
            Some(_) | None => text.as_str().to_owned(),
        };
        scratch.write(
            &RelPath::from_string(format!("result-{}/result.txt", job.name().as_str())),
            &Text::from_string(shown),
        )?;
    }
    Ok(())
}

fn blocker_codes(vars: &Vars, scratch: &Scratch) -> Result<Vec<BlockerCode>, Fail> {
    let gathered = gather(scratch.path(), vars, &commits())?;
    Ok(gathered
        .blockers
        .iter()
        .map(|blocker| blocker.code)
        .collect())
}

#[test]
fn gather_clean() {
    assert_eq!(check_gather_clean(), Ok(()), "gather_clean");
}

fn check_gather_clean() -> Result<(), Fail> {
    let scratch = Scratch::repository()?;
    write_results(&scratch, None)?;
    let gathered = gather(scratch.path(), &all_success(), &commits())?;
    ensure_eq!(gathered.reasons.len(), 0, "no reasons");
    ensure_eq!(gathered.blockers.len(), 0, "no blockers");
    ensure_eq!(
        compute(&gathered.reasons, &gathered.blockers).verdict,
        Verdict::Auto,
        "clean means auto"
    );
    Ok(())
}

#[test]
fn failed_job_blocks() {
    assert_eq!(check_failed_job_blocks(), Ok(()), "failed_job_blocks");
}

fn check_failed_job_blocks() -> Result<(), Fail> {
    let scratch = Scratch::repository()?;
    write_results(&scratch, None)?;
    let vars = all_success().with(&Job::Mutants.result_variable(), &Text::of("failure"));
    ensure_eq!(
        blocker_codes(&vars, &scratch)?,
        vec![BlockerCode::InputJobFailed],
        "a failed job blocks"
    );
    Ok(())
}

#[test]
fn unparsable_artifact_blocks() {
    assert_eq!(
        check_unparsable_artifact_blocks(),
        Ok(()),
        "unparsable_artifact_blocks"
    );
}

fn check_unparsable_artifact_blocks() -> Result<(), Fail> {
    let scratch = Scratch::repository()?;
    write_results(
        &scratch,
        Some(&Tweak {
            from: Text::of("end\n"),
            job: Job::HeadChecks,
            to: Text::of("end\ngarbage\n"),
        }),
    )?;
    ensure_eq!(
        blocker_codes(&all_success(), &scratch)?,
        vec![BlockerCode::ResultUnparsable],
        "garbage is a blocker"
    );
    Ok(())
}

#[test]
fn missing_artifact_blocks() {
    assert_eq!(
        check_missing_artifact_blocks(),
        Ok(()),
        "missing_artifact_blocks"
    );
}

fn check_missing_artifact_blocks() -> Result<(), Fail> {
    let scratch = Scratch::repository()?;
    ensure_eq!(
        blocker_codes(&all_success(), &scratch)?.len(),
        Job::ALL.len(),
        "every missing artifact blocks"
    );
    Ok(())
}

#[test]
fn artifact_for_another_head_blocks() {
    assert_eq!(
        check_artifact_for_another_head_blocks(),
        Ok(()),
        "artifact_for_another_head_blocks"
    );
}

fn check_artifact_for_another_head_blocks() -> Result<(), Fail> {
    let scratch = Scratch::repository()?;
    write_results(
        &scratch,
        Some(&Tweak {
            from: Text::of(sha!('a').as_str()),
            job: Job::Classify,
            to: Text::of(sha!('f').as_str()),
        }),
    )?;
    ensure_eq!(
        blocker_codes(&all_success(), &scratch)?,
        vec![BlockerCode::ResultUnparsable],
        "another head is refused"
    );
    Ok(())
}

fn auto_data() -> VerdictData {
    VerdictData {
        approvable: Approvable::Yes,
        attempt: Count::new(1),
        blockers: Vec::new(),
        commits: commits(),
        counts: Counts::empty(),
        inputs: Vec::new(),
        policy_commit: sha!('c'),
        pull: pull(),
        reasons: Vec::new(),
        reviewed: None,
        run_id: Count::new(5),
        verdict: Verdict::Auto,
    }
}

fn merge_input(data: VerdictData) -> MergeInput {
    MergeInput {
        data,
        default_branch: Name::of("main"),
        env: PolicyEnv {
            commits: commits(),
            pull: pull(),
        },
    }
}

fn open_pull(head: &Sha) -> Result<FakeGithub, Fail> {
    Ok(FakeGithub::new().route(
        &Text::of("/pulls/7"),
        &json_of(&Text::from_string(format!(
            "{{\"state\":\"open\",\"draft\":false,\"head\":{{\"sha\":\"{}\"}},\"base\":{{\"ref\":\"main\"}}}}",
            head.as_str()
        )))?,
    ))
}

#[test]
fn merge_refuses_non_auto() {
    assert_eq!(
        check_merge_refuses_non_auto(),
        Ok(()),
        "merge_refuses_non_auto"
    );
}

fn check_merge_refuses_non_auto() -> Result<(), Fail> {
    let fake = open_pull(&sha!('a'))?;
    let data = VerdictData {
        reasons: vec![Reason::general(Code::OtherPathChanged, &Text::of("x"))],
        verdict: Verdict::NeedsHuman,
        ..auto_data()
    };
    ensure_eq!(
        merge(&fake, &merge_input(data))?,
        MergeOutcome::NotAuto,
        "not auto"
    );
    ensure_eq!(fake.sent().len(), 0, "nothing was merged");
    Ok(())
}

#[test]
fn merge_is_squash_pinned_to_head_with_verdict_block() {
    assert_eq!(
        check_merge_is_squash_pinned_to_head_with_verdict_block(),
        Ok(()),
        "merge_is_squash_pinned_to_head_with_verdict_block"
    );
}

fn check_merge_is_squash_pinned_to_head_with_verdict_block() -> Result<(), Fail> {
    let fake = open_pull(&sha!('a'))?;
    ensure_eq!(
        merge(&fake, &merge_input(auto_data()))?,
        MergeOutcome::Merged,
        "merged"
    );
    let sent = fake.sent();
    ensure_eq!(sent.len(), 1, "one merge call");
    let Some(call) = sent.first() else {
        return Err(Fail::new(&Message::of("no merge call")));
    };
    ensure_eq!(call.method, Method::Put, "PUT");
    ensure!(
        call.path.as_str().ends_with("/7/merge"),
        "merge endpoint: {}",
        call.path.as_str()
    );
    let pinned = sha!('a');
    ensure_eq!(
        call.body.text_at(&Name::of("sha")).map(Text::as_str),
        Some(pinned.as_str()),
        "pinned to the evaluated head"
    );
    ensure_eq!(
        call.body
            .text_at(&Name::of("merge_method"))
            .map(Text::as_str),
        Some("squash"),
        "squash"
    );
    let message = call.body.text_at(&Name::of("commit_message"));
    ensure!(
        message.is_some_and(|text| text.as_str().trim_end().ends_with("approver: none")),
        "the message ends with the verdict block"
    );
    Ok(())
}

#[test]
fn merge_refuses_when_head_moved() {
    assert_eq!(
        check_merge_refuses_when_head_moved(),
        Ok(()),
        "merge_refuses_when_head_moved"
    );
}

fn check_merge_refuses_when_head_moved() -> Result<(), Fail> {
    let fake = open_pull(&sha!('9'))?;
    ensure_eq!(
        merge(&fake, &merge_input(auto_data()))?,
        MergeOutcome::PullChanged,
        "a newer head is not merged"
    );
    ensure_eq!(fake.sent().len(), 0, "nothing was merged");
    Ok(())
}

#[test]
fn merge_reports_a_refused_call() {
    assert_eq!(
        check_merge_reports_a_refused_call(),
        Ok(()),
        "merge_reports_a_refused_call"
    );
}

fn check_merge_reports_a_refused_call() -> Result<(), Fail> {
    let fake = open_pull(&sha!('a'))?.refusing_writes();
    let outcome = merge(&fake, &merge_input(auto_data()))?;
    match outcome {
        MergeOutcome::Refused(_) => {}
        MergeOutcome::NotAuto
        | MergeOutcome::PullChanged
        | MergeOutcome::WrongBase
        | MergeOutcome::Merged => {
            return Err(Fail::new(&Message::of("a refused merge must be reported")));
        }
    }
    Ok(())
}

#[test]
fn invalid_env_is_refused() {
    let good = Vars::empty()
        .with(&Name::of("PR_NUMBER"), &Text::of("7"))
        .with(&Name::of("HEAD_SHA"), &Text::of(sha!('a').as_str()))
        .with(&Name::of("BASE_SHA"), &Text::of(sha!('b').as_str()));
    assert_eq!(
        validated_env(&good),
        Ok(PolicyEnv {
            commits: commits(),
            pull: pull()
        }),
        "valid input passes"
    );
    let base_with_newline = format!("{}\n", sha!('b').as_str());
    for (key, value) in [
        ("PR_NUMBER", "7; id"),
        ("HEAD_SHA", "xyz"),
        ("BASE_SHA", base_with_newline.as_str()),
    ] {
        let vars = good.clone().with(&Name::of(key), &Text::of(value));
        assert!(
            validated_env(&vars).is_err(),
            "{key}={value:?} must be refused"
        );
    }
}

#[test]
fn settings_audit_must_be_recent_and_green() {
    assert_eq!(
        check_settings_audit_must_be_recent_and_green(),
        Ok(()),
        "settings_audit_must_be_recent_and_green"
    );
}

fn check_settings_audit_must_be_recent_and_green() -> Result<(), Fail> {
    let branch = Name::of("main");
    let now = Count::new(NOON);
    ensure_eq!(
        settings_audit_state(
            &audit_runs!("2024-02-29T01:00:00Z", "success")?,
            &branch,
            now
        )?,
        AuditState::Fresh,
        "eleven hours old is fresh"
    );
    ensure_eq!(
        settings_audit_state(
            &audit_runs!("2024-02-28T11:00:00Z", "success")?,
            &branch,
            now
        )?,
        AuditState::Problem(Text::of("latest settings-audit run is older than 24 hours")),
        "twenty-five hours old is stale"
    );
    ensure_eq!(
        settings_audit_state(
            &audit_runs!("2024-02-29T11:00:00Z", "failure")?,
            &branch,
            now
        )?,
        AuditState::Problem(Text::of("latest settings-audit run did not succeed")),
        "a failed audit blocks"
    );
    let none = FakeGithub::new().route(
        &Text::of("/runs?branch=main"),
        &json_of(&Text::of("{\"workflow_runs\":[]}"))?,
    );
    ensure_eq!(
        settings_audit_state(&none, &branch, now)?,
        AuditState::Problem(Text::of("no completed settings-audit run")),
        "no audit at all blocks"
    );
    Ok(())
}

#[test]
fn decide_posts_the_check_and_one_comment() {
    assert_eq!(
        check_decide_posts_the_check_and_one_comment(),
        Ok(()),
        "decide_posts_the_check_and_one_comment"
    );
}

fn check_decide_posts_the_check_and_one_comment() -> Result<(), Fail> {
    let scratch = Scratch::repository()?;
    write_results(&scratch, None)?;
    let app = FakeGithub::new()
        .route(
            &Text::of("/check-runs?"),
            &json_of(&Text::of("{\"check_runs\":[]}"))?,
        )
        .route(&Text::of("/issues/7/comments"), &json_of(&Text::of("[]"))?);
    let reader = audit_runs!("2024-02-29T11:00:00Z", "success")?;
    let data = decide(
        &Clients {
            app: &app,
            reader: &reader,
        },
        &all_success(),
        &DecideInput {
            default_branch: Name::of("main"),
            env: PolicyEnv {
                commits: commits(),
                pull: pull(),
            },
            now: Count::new(NOON),
            policy_commit: sha!('c'),
            results_dir: scratch.path().to_path_buf(),
            run: RunIds {
                attempt: Count::new(1),
                run_id: Count::new(5),
            },
        },
    )?;
    ensure_eq!(data.verdict, Verdict::Auto, "a clean run is auto");
    let sent = app.sent();
    let paths: Vec<&str> = sent.iter().map(|write| write.path.as_str()).collect();
    ensure!(
        paths.len() == 2
            && paths
                .first()
                .is_some_and(|path| path.ends_with("/check-runs"))
            && paths
                .get(1)
                .is_some_and(|path| path.ends_with("/issues/7/comments")),
        "a check run then a comment: {paths:?}"
    );
    let body: Option<&Json> = sent.first().map(|write| &write.body);
    ensure_eq!(
        body.and_then(|found| found.text_at(&Name::of("conclusion")))
            .map(Text::as_str),
        Some("success"),
        "the check passes"
    );
    ensure_eq!(
        io::read_bytes(&scratch.path().join("result-classify/result.txt"))
            .map(|bytes: Bytes| bytes.len().get() > 0),
        Ok(true),
        "inputs were left alone"
    );
    Ok(())
}
