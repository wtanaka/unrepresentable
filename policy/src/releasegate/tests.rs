use super::{judge, Gate, Verdict, REQUIRED_JOBS, SKIP_JOB, WORKFLOW_NAME, WORKFLOW_PATH};
use crate::exit::ExitStatus;
use crate::testkit::{json_of, Fail, FakeGithub};
use crate::types::{Glyph, Name, Sha, Text};

/// The check suite of the push's workflow run.
const SUITE: &str = "77";

/// What the fake answers for the push-skip lookups.
#[derive(Clone, Copy)]
enum PushSkip {
    /// No pull request lookups answer.
    Absent,
    /// The same tree passed as a pull request head.
    Passes,
    /// The pull request head has another tree.
    WrongTree,
}

/// The conclusions of the nine jobs and of `skip-check`.
struct Outcome {
    jobs: Text,
    skip: Text,
}

impl Outcome {
    fn good() -> Self {
        Self {
            jobs: Text::of("success"),
            skip: Text::of("success"),
        }
    }

    fn skipped() -> Self {
        Self {
            jobs: Text::of("skipped"),
            skip: Text::of("success"),
        }
    }
}

/// One job's check run.
#[derive(Clone)]
struct Job {
    app: Text,
    conclusion: Text,
    name: Text,
    status: Text,
    suite: Text,
}

impl Job {
    fn json(&self) -> Text {
        Text::from_string(format!(
            "{{\"name\":\"{}\",\"head_sha\":\"{}\",\"app\":{{\"slug\":\"{}\"}},\"check_suite\":{{\"id\":{}}},\"status\":\"{}\",\"conclusion\":\"{}\"}}",
            self.name.as_str(),
            tagged().as_str(),
            self.app.as_str(),
            self.suite.as_str(),
            self.status.as_str(),
            self.conclusion.as_str()
        ))
    }

    fn of(name: &Name, conclusion: &Text) -> Self {
        Self {
            app: Text::of("github-actions"),
            conclusion: conclusion.clone(),
            name: Text::of(name.as_str()),
            status: Text::of("completed"),
            suite: Text::of(SUITE),
        }
    }

    fn pending(name: &Name) -> Self {
        Self {
            status: Text::of("in_progress"),
            ..Self::of(name, &Text::of("success"))
        }
    }
}

/// The push run of the workflow, as the API lists it.
#[derive(Clone)]
struct Run {
    branch: Text,
    event: Text,
    name: Text,
    path: Text,
}

impl Run {
    fn json(&self) -> Text {
        Text::from_string(format!(
            "{{\"name\":\"{}\",\"path\":\"{}\",\"event\":\"{}\",\"head_branch\":\"{}\",\"head_sha\":\"{}\",\"check_suite_id\":{SUITE}}}",
            self.name.as_str(),
            self.path.as_str(),
            self.event.as_str(),
            self.branch.as_str(),
            tagged().as_str()
        ))
    }

    fn policy() -> Self {
        Self {
            branch: Text::of("main"),
            event: Text::of("push"),
            name: Text::of(WORKFLOW_NAME),
            path: Text::of(WORKFLOW_PATH),
        }
    }
}

/// What the fake GitHub knows; `None` leaves a call unanswered, which is an
/// API error.
struct World {
    jobs: Option<Vec<Job>>,
    push_skip: PushSkip,
    runs: Option<Vec<Run>>,
}

impl World {
    /// The nine jobs and `skip-check` with the conclusions in `outcome`.
    fn all(outcome: &Outcome) -> Vec<Job> {
        let mut jobs: Vec<Job> = REQUIRED_JOBS
            .iter()
            .map(|name| Job::of(&Name::of(name), &outcome.jobs))
            .collect();
        jobs.push(Job::of(&Name::of(SKIP_JOB), &outcome.skip));
        jobs
    }

    fn fake(&self) -> Result<FakeGithub, Fail> {
        let mut fake = FakeGithub::new();
        if let Some(runs) = &self.runs {
            fake = fake.route(
                &Text::from_string(format!("/actions/runs?head_sha={}", tagged().as_str())),
                &json_of(&Text::from_string(format!(
                    "{{\"workflow_runs\":[{}]}}",
                    runs.iter()
                        .map(|run| run.json().as_str().to_owned())
                        .collect::<Vec<String>>()
                        .join(",")
                )))?,
            );
        }
        if let Some(jobs) = &self.jobs {
            fake = fake.route(
                &Text::from_string(format!("/commits/{}/check-runs", tagged().as_str())),
                &json_of(&Text::from_string(format!(
                    "{{\"check_runs\":[{}]}}",
                    jobs.iter()
                        .map(|job| job.json().as_str().to_owned())
                        .collect::<Vec<String>>()
                        .join(",")
                )))?,
            );
        }
        self.pulls_for(fake)
    }

    /// `fake` plus the pull request lookups that push-skip makes.
    fn pulls_for(&self, fake: FakeGithub) -> Result<FakeGithub, Fail> {
        let head_tree = match self.push_skip {
            PushSkip::Absent => return Ok(fake),
            PushSkip::Passes => tree_e(),
            PushSkip::WrongTree => tree_f(),
        };
        let policy_run = format!(
            "{{\"check_runs\":[{{\"name\":\"policy\",\"head_sha\":\"{}\",\"app\":{{\"slug\":\"unrepresentable-policy\"}},\"status\":\"completed\",\"conclusion\":\"success\"}}]}}",
            head().as_str()
        );
        Ok(fake
            .route(
                &Text::from_string(format!("/git/commits/{}", tagged().as_str())),
                &json_of(&Text::from_string(format!(
                    "{{\"tree\":{{\"sha\":\"{}\"}}}}",
                    tree_e().as_str()
                )))?,
            )
            .route(
                &Text::from_string(format!("/git/commits/{}", head().as_str())),
                &json_of(&Text::from_string(format!(
                    "{{\"tree\":{{\"sha\":\"{}\"}}}}",
                    head_tree.as_str()
                )))?,
            )
            .route(
                &Text::from_string(format!("/commits/{}/pulls", tagged().as_str())),
                &json_of(&Text::from_string(format!(
                    "[{{\"head\":{{\"sha\":\"{}\"}}}}]",
                    head().as_str()
                )))?,
            )
            .route(
                &Text::from_string(format!("/commits/{}/check-runs", head().as_str())),
                &json_of(&Text::from_string(policy_run))?,
            ))
    }

    fn verdict(&self) -> Result<Verdict, Fail> {
        Ok(judge(&self.fake()?, &tagged(), &Name::of("main")))
    }

    /// A world whose jobs are `jobs`, with a good push run.
    fn with(jobs: Vec<Job>, push_skip: PushSkip) -> Self {
        Self {
            jobs: Some(jobs),
            push_skip,
            runs: Some(vec![Run::policy()]),
        }
    }
}

fn head() -> Sha {
    sha!('2')
}

fn tagged() -> Sha {
    sha!('1')
}

fn tree_e() -> Sha {
    Sha::filled(Glyph::new('e'))
}

fn tree_f() -> Sha {
    Sha::filled(Glyph::new('f'))
}

fn gate_of(world: &World) -> Result<Gate, Fail> {
    Ok(world.verdict()?.gate)
}

#[test]
fn all_nine_succeeding_passes() {
    assert_eq!(
        check_all_nine_succeeding_passes(),
        Ok(()),
        "all_nine_succeeding_passes"
    );
}

fn check_all_nine_succeeding_passes() -> Result<(), Fail> {
    let verdict = World::with(World::all(&Outcome::good()), PushSkip::Absent).verdict()?;
    ensure_eq!(verdict.gate, Gate::Open, "open: {}", verdict.note.as_str());
    ensure_eq!(verdict.report().status, ExitStatus::Clean, "exit clean");
    let mut without_skip_check = World::all(&Outcome::good());
    without_skip_check.retain(|job| job.name.as_str() != SKIP_JOB);
    let bare = World::with(without_skip_check, PushSkip::Absent);
    ensure_eq!(gate_of(&bare)?, Gate::Open, "skip-check is not needed");
    Ok(())
}

#[test]
fn one_job_not_succeeding_refuses() {
    assert_eq!(
        check_one_job_not_succeeding_refuses(),
        Ok(()),
        "one_job_not_succeeding_refuses"
    );
}

fn check_one_job_not_succeeding_refuses() -> Result<(), Fail> {
    for target in REQUIRED_JOBS {
        for conclusion in [
            "failure",
            "cancelled",
            "neutral",
            "timed_out",
            "action_required",
        ] {
            let jobs: Vec<Job> = World::all(&Outcome::good())
                .into_iter()
                .map(|job| {
                    if job.name.as_str() == target {
                        Job::of(&Name::of(target), &Text::of(conclusion))
                    } else {
                        job
                    }
                })
                .collect();
            let verdict = World::with(jobs, PushSkip::Absent).verdict()?;
            ensure_eq!(verdict.gate, Gate::Closed, "{target} {conclusion}");
            ensure!(
                verdict.note.as_str().contains(target)
                    && verdict
                        .note
                        .as_str()
                        .contains("re-run the release workflow"),
                "note names the job and the remedy: {}",
                verdict.note.as_str()
            );
            ensure!(!verdict.note.as_str().contains('\n'), "one line");
            ensure_eq!(verdict.report().status, ExitStatus::Failed, "exit failed");
        }
        let pending: Vec<Job> = World::all(&Outcome::good())
            .into_iter()
            .map(|job| {
                if job.name.as_str() == target {
                    Job::pending(&Name::of(target))
                } else {
                    job
                }
            })
            .collect();
        ensure_eq!(
            gate_of(&World::with(pending, PushSkip::Absent))?,
            Gate::Closed,
            "{target} pending"
        );
        let missing: Vec<Job> = World::all(&Outcome::good())
            .into_iter()
            .filter(|job| job.name.as_str() != target)
            .collect();
        ensure_eq!(
            gate_of(&World::with(missing, PushSkip::Absent))?,
            Gate::Closed,
            "{target} missing"
        );
    }
    Ok(())
}

#[test]
fn skipped_jobs_pass_with_a_push_skip_yes() {
    assert_eq!(
        check_skipped_jobs_pass_with_a_push_skip_yes(),
        Ok(()),
        "skipped_jobs_pass_with_a_push_skip_yes"
    );
}

fn check_skipped_jobs_pass_with_a_push_skip_yes() -> Result<(), Fail> {
    let skipped = World::all(&Outcome::skipped());
    let verdict = World::with(skipped.clone(), PushSkip::Passes).verdict()?;
    ensure_eq!(verdict.gate, Gate::Open, "open: {}", verdict.note.as_str());
    let mut absent = skipped;
    absent.retain(|job| job.name.as_str() == SKIP_JOB);
    ensure_eq!(
        gate_of(&World::with(absent, PushSkip::Passes))?,
        Gate::Open,
        "jobs without check runs are skipped jobs"
    );
    Ok(())
}

#[test]
fn skipped_jobs_refuse_without_a_push_skip_yes() {
    assert_eq!(
        check_skipped_jobs_refuse_without_a_push_skip_yes(),
        Ok(()),
        "skipped_jobs_refuse_without_a_push_skip_yes"
    );
}

fn check_skipped_jobs_refuse_without_a_push_skip_yes() -> Result<(), Fail> {
    for (label, push_skip) in [
        ("no lookups answer", PushSkip::Absent),
        ("another tree", PushSkip::WrongTree),
    ] {
        let world = World::with(World::all(&Outcome::skipped()), push_skip);
        ensure_eq!(gate_of(&world)?, Gate::Closed, "{label}");
    }
    let no_skip_check = World::with(
        World::all(&Outcome::skipped())
            .into_iter()
            .filter(|job| job.name.as_str() != SKIP_JOB)
            .collect(),
        PushSkip::Passes,
    );
    ensure_eq!(gate_of(&no_skip_check)?, Gate::Closed, "no skip-check job");
    for state in ["failure", "skipped", "cancelled"] {
        let world = World::with(
            World::all(&Outcome {
                jobs: Text::of("skipped"),
                skip: Text::of(state),
            }),
            PushSkip::Passes,
        );
        ensure_eq!(gate_of(&world)?, Gate::Closed, "skip-check {state}");
    }
    let mut one_failed = World::all(&Outcome::skipped());
    one_failed.push(Job::of(&Name::of("fmt"), &Text::of("failure")));
    ensure_eq!(
        gate_of(&World::with(one_failed, PushSkip::Passes))?,
        Gate::Closed,
        "a failed job beside skipped ones"
    );
    let mut one_pending = World::all(&Outcome::skipped());
    one_pending.push(Job::pending(&Name::of("test")));
    ensure_eq!(
        gate_of(&World::with(one_pending, PushSkip::Passes))?,
        Gate::Closed,
        "a pending job beside skipped ones"
    );
    Ok(())
}

#[test]
fn api_errors_refuse() {
    assert_eq!(check_api_errors_refuse(), Ok(()), "api_errors_refuse");
}

fn check_api_errors_refuse() -> Result<(), Fail> {
    let no_runs = World {
        runs: None,
        ..World::with(World::all(&Outcome::good()), PushSkip::Absent)
    };
    let no_checks = World {
        jobs: None,
        ..World::with(World::all(&Outcome::good()), PushSkip::Absent)
    };
    let nothing = World {
        jobs: None,
        push_skip: PushSkip::Absent,
        runs: None,
    };
    for (label, world) in [
        ("workflow run lookup fails", no_runs),
        ("check run lookup fails", no_checks),
        ("nothing answers", nothing),
    ] {
        let verdict = world.verdict()?;
        ensure_eq!(verdict.gate, Gate::Closed, "{label}");
        ensure!(
            verdict.note.as_str().contains("could not confirm CI"),
            "{label}: {}",
            verdict.note.as_str()
        );
        ensure!(!verdict.note.as_str().contains('\n'), "{label}: one line");
    }
    let no_tree = World {
        push_skip: PushSkip::Absent,
        ..World::with(World::all(&Outcome::skipped()), PushSkip::Absent)
    };
    ensure_eq!(
        gate_of(&no_tree)?,
        Gate::Closed,
        "push-skip lookups fail after skipped jobs"
    );
    Ok(())
}

#[test]
fn another_app_or_workflow_refuses() {
    assert_eq!(
        check_another_app_or_workflow_refuses(),
        Ok(()),
        "check_another_app_or_workflow_refuses"
    );
}

fn check_another_app_or_workflow_refuses() -> Result<(), Fail> {
    let wrong_app: Vec<Job> = World::all(&Outcome::good())
        .into_iter()
        .map(|job| Job {
            app: Text::of("someone-else"),
            ..job
        })
        .collect();
    ensure_eq!(
        gate_of(&World::with(wrong_app, PushSkip::Absent))?,
        Gate::Closed,
        "another app"
    );
    let mut one_foreign = World::all(&Outcome::good());
    if let Some(first) = one_foreign.first_mut() {
        first.app = Text::of("someone-else");
    }
    ensure_eq!(
        gate_of(&World::with(one_foreign, PushSkip::Absent))?,
        Gate::Closed,
        "one job from another app"
    );
    let other_suite: Vec<Job> = World::all(&Outcome::good())
        .into_iter()
        .map(|job| Job {
            suite: Text::of("78"),
            ..job
        })
        .collect();
    ensure_eq!(
        gate_of(&World::with(other_suite, PushSkip::Absent))?,
        Gate::Closed,
        "jobs of another workflow run"
    );
    Ok(())
}

#[test]
fn another_workflow_run_refuses() {
    assert_eq!(
        check_another_workflow_run_refuses(),
        Ok(()),
        "another_workflow_run_refuses"
    );
}

fn check_another_workflow_run_refuses() -> Result<(), Fail> {
    let mut renamed = Run::policy();
    renamed.name = Text::of("something-else");
    let mut elsewhere = Run::policy();
    elsewhere.path = Text::of(".github/workflows/other.yml");
    let mut pull_request = Run::policy();
    pull_request.event = Text::of("pull_request_target");
    for (label, run) in [
        ("another workflow name", renamed),
        ("another workflow file", elsewhere),
        ("another event", pull_request),
    ] {
        let world = World {
            runs: Some(vec![run]),
            ..World::with(World::all(&Outcome::good()), PushSkip::Absent)
        };
        ensure_eq!(gate_of(&world)?, Gate::Closed, "{label}");
    }
    let none = World {
        runs: Some(Vec::new()),
        ..World::with(World::all(&Outcome::good()), PushSkip::Absent)
    };
    ensure_eq!(gate_of(&none)?, Gate::Closed, "no push run");
    Ok(())
}

#[test]
fn a_manual_run_of_the_default_branch_counts_when_the_push_run_was_dropped() {
    assert_eq!(
        check_a_manual_run_of_the_default_branch_counts_when_the_push_run_was_dropped(),
        Ok(()),
        "a_manual_run_of_the_default_branch_counts_when_the_push_run_was_dropped"
    );
}

fn check_a_manual_run_of_the_default_branch_counts_when_the_push_run_was_dropped(
) -> Result<(), Fail> {
    let gate_for = |run: Run, skip: PushSkip| -> Result<Gate, Fail> {
        gate_of(&World {
            runs: Some(vec![run]),
            ..World::with(World::all(&Outcome::good()), skip)
        })
    };
    let mut dispatched = Run::policy();
    dispatched.event = Text::of("workflow_dispatch");
    ensure_eq!(
        gate_for(dispatched.clone(), PushSkip::Absent)?,
        Gate::Open,
        "a manual run of the default branch whose nine jobs passed opens the gate"
    );
    let mut dispatched_ref = dispatched.clone();
    dispatched_ref.path = Text::from_string(format!("{WORKFLOW_PATH}@refs/heads/main"));
    ensure_eq!(
        gate_for(dispatched_ref, PushSkip::Absent)?,
        Gate::Open,
        "as the API spells the path of a run of the default branch"
    );
    let mut other_branch = dispatched.clone();
    other_branch.branch = Text::of("evil");
    ensure_eq!(
        gate_for(other_branch, PushSkip::Absent)?,
        Gate::Closed,
        "a manual run of another branch does not count"
    );
    let mut other_file = dispatched;
    other_file.path = Text::of(".github/workflows/other.yml");
    ensure_eq!(
        gate_for(other_file, PushSkip::Absent)?,
        Gate::Closed,
        "nor does a manual run of another workflow"
    );
    Ok(())
}

#[test]
fn only_a_run_of_the_default_branch_counts() {
    assert_eq!(
        check_only_a_run_of_the_default_branch_counts(),
        Ok(()),
        "only_a_run_of_the_default_branch_counts"
    );
}

fn check_only_a_run_of_the_default_branch_counts() -> Result<(), Fail> {
    let gate_for = |run: Run| -> Result<Gate, Fail> {
        gate_of(&World {
            runs: Some(vec![run]),
            ..World::with(World::all(&Outcome::good()), PushSkip::Absent)
        })
    };
    let mut default_ref = Run::policy();
    default_ref.path = Text::from_string(format!("{WORKFLOW_PATH}@refs/heads/main"));
    ensure_eq!(
        gate_for(default_ref)?,
        Gate::Open,
        "the path the API gives a run of the default branch"
    );
    let mut other_ref = Run::policy();
    other_ref.path = Text::from_string(format!("{WORKFLOW_PATH}@refs/heads/evil"));
    ensure_eq!(
        gate_for(other_ref)?,
        Gate::Closed,
        "a path with another ref is not the default branch's workflow"
    );
    let mut tag_ref = Run::policy();
    tag_ref.path = Text::from_string(format!("{WORKFLOW_PATH}@refs/tags/v1.0.0"));
    ensure_eq!(gate_for(tag_ref)?, Gate::Closed, "nor is a tag's");
    let mut copy = Run::policy();
    copy.branch = Text::of("evil");
    ensure_eq!(
        gate_for(copy)?,
        Gate::Closed,
        "a run of another branch with the same commit is not the default branch's run"
    );
    let mut main_prefix = Run::policy();
    main_prefix.branch = Text::of("main-evil");
    ensure_eq!(
        gate_for(main_prefix)?,
        Gate::Closed,
        "the name must be exact"
    );
    Ok(())
}

#[test]
fn a_closed_gate_prints_one_line() {
    assert_eq!(
        check_a_closed_gate_prints_one_line(),
        Ok(()),
        "a_closed_gate_prints_one_line"
    );
}

fn check_a_closed_gate_prints_one_line() -> Result<(), Fail> {
    let verdict = judge(&FakeGithub::new(), &tagged(), &Name::of("main"));
    ensure_eq!(verdict.gate, Gate::Closed, "an empty GitHub refuses");
    ensure_eq!(verdict.report().lines.len(), 1, "one printed line");
    let closed = Verdict::closed(&Text::of("no token or repository"));
    ensure_eq!(closed.report().status, ExitStatus::Failed, "exit failed");
    Ok(())
}
