use super::{evaluate_pr, open_pull_numbers, open_pulls_with_head, sweep, ReviewStatus, Running};
use crate::checkrun;
use crate::data::{InputResults, VerdictData};
use crate::github::Method;
use crate::json::Json;
use crate::post::Clients;
use crate::reason::{Code, Commits, Counts, Job, Reason};
use crate::report::{self, Approval, Conclusion, Info};
use crate::testkit::{json_of, Fail, FakeGithub};
use crate::types::{Answer, Count, Name, RelPath, Sha, Text};
use crate::verdict::{Approvable, PullNumber, Verdict};

/// The App's check run as GitHub would return it.
struct Run {
    conclusion: Text,
    data: VerdictData,
    slug: Text,
}

impl Run {
    fn json(&self) -> Result<Json, Fail> {
        let info = Info {
            approvers: Vec::new(),
            blockers: Vec::new(),
            commits: self.data.commits.clone(),
            counts: Counts::empty(),
            policy_commit: self.data.policy_commit.clone(),
            reasons: self.data.reasons.clone(),
            verdict: self.data.verdict,
        };
        let check = report::build_check(&info, Approval::NotApproved);
        let payload = checkrun::payload(&check, &self.data);
        let output = match payload.get(&Name::of("output")) {
            Some(found) => found.compact(),
            None => Text::of("{}"),
        };
        json_of(&Text::from_string(format!(
            "{{\"id\":99,\"name\":\"policy\",\"head_sha\":\"{}\",\"app\":{{\"slug\":\"{}\"}},\"conclusion\":\"{}\",\"output\":{}}}",
            head().as_str(),
            self.slug.as_str(),
            self.conclusion.as_str(),
            output.as_str()
        )))
    }

    fn of(data: VerdictData) -> Self {
        Self {
            conclusion: Text::of("failure"),
            data,
            slug: Text::of("unrepresentable-policy"),
        }
    }
}

/// The pull requests of one page of the listing.
struct Page {
    /// How many pull requests the page lists.
    count: Count,
    /// The number of the first one.
    first: Count,
}

/// Everything the fake GitHub holds about pull request 7.
struct World {
    base_ref: Text,
    base_tip: Sha,
    commit_count: Option<Count>,
    commit_entries: Vec<Text>,
    extra_review: Option<Text>,
    failed_job: Option<Job>,
    policy_commit: Sha,
    review_commit: Text,
    review_login: Text,
    review_state: Text,
    run: Option<Run>,
    run_path: Text,
}

impl World {
    /// The commits of the pull request, as the API lists them.
    fn commits_reply(&self) -> Result<Json, Fail> {
        let items: Vec<&str> = self.commit_entries.iter().map(Text::as_str).collect();
        json_of(&Text::from_string(format!("[{}]", items.join(","))))
    }

    fn fake(&self) -> Result<FakeGithub, Fail> {
        let runs = match &self.run {
            Some(found) => format!("[{}]", found.json()?.compact().as_str()),
            None => "[]".to_owned(),
        };
        let mut reviews = format!(
            "{{\"user\":{{\"login\":\"{}\",\"type\":\"User\"}},\"state\":\"{}\",\"commit_id\":\"{}\"}}",
            self.review_login.as_str(),
            self.review_state.as_str(),
            self.review_commit.as_str()
        );
        if let Some(extra) = &self.extra_review {
            reviews = format!(
                "{reviews},{{\"user\":{{\"login\":\"{}\",\"type\":\"User\"}},\"state\":\"{}\",\"commit_id\":\"{}\"}}",
                self.review_login.as_str(),
                extra.as_str(),
                self.review_commit.as_str()
            );
        }
        let jobs = self.job_entries();
        Ok(FakeGithub::new()
            .route(
                &Text::of("/pulls/7"),
                &json_of(&Text::from_string(format!(
                    "{{\"state\":\"open\",\"commits\":{},\"head\":{{\"sha\":\"{}\"}},\"base\":{{\"ref\":\"{}\"}},\"user\":{{\"login\":\"author\"}}}}",
                    self.reported_commits().get(),
                    head().as_str(),
                    self.base_ref.as_str()
                )))?,
            )
            .route(
                &Text::of("/check-runs?"),
                &json_of(&Text::from_string(format!("{{\"check_runs\":{runs}}}")))?,
            )
            .route(
                &Text::of("/branches/main"),
                &json_of(&Text::from_string(format!(
                    "{{\"commit\":{{\"sha\":\"{}\"}}}}",
                    self.base_tip.as_str()
                )))?,
            )
            .route(&Text::of("/pulls/7/commits"), &self.commits_reply()?)
            .route(
                &Text::of("/pulls/7/reviews"),
                &json_of(&Text::from_string(format!("[{reviews}]")))?,
            )
            .route(
                &Text::of("/actions/runs/5/attempts/1/jobs"),
                &json_of(&Text::from_string(format!("{{\"jobs\":[{}]}}", jobs.iter().map(Text::as_str).collect::<Vec<&str>>().join(","))))?,
            )
            .route(
                &Text::of("/actions/runs/5"),
                &json_of(&Text::from_string(format!(
                    "{{\"path\":\"{}\",\"status\":\"completed\"}}",
                    self.run_path.as_str()
                )))?,
            )
            .with_text_reply(&Text::of("* @maint\n")))
    }

    fn go(&self) -> Result<(ReviewStatus, FakeGithub), Fail> {
        let fake = self.fake()?;
        let status = evaluate_pr(
            &Clients {
                app: &fake,
                reader: &fake,
            },
            pull(),
            &Running {
                default_branch: Name::of("main"),
                policy_commit: self.policy_commit.clone(),
            },
        )?;
        Ok((status, fake))
    }

    /// The jobs of the policy run, as the API lists them.
    fn job_entries(&self) -> Vec<Text> {
        Job::ALL
            .into_iter()
            .map(|job| {
                let conclusion = if self.failed_job == Some(job) {
                    "failure"
                } else {
                    "success"
                };
                Text::from_string(format!(
                    "{{\"name\":\"{}\",\"conclusion\":\"{conclusion}\"}}",
                    job.name().as_str()
                ))
            })
            .collect()
    }

    fn new(run: Option<Run>) -> Self {
        Self {
            base_ref: Text::of("main"),
            base_tip: sha!('b'),
            commit_count: None,
            commit_entries: vec![normal_commit()],
            extra_review: None,
            failed_job: None,
            policy_commit: sha!('c'),
            review_commit: Text::of(head().as_str()),
            review_login: Text::of("maint"),
            review_state: Text::of("APPROVED"),
            run,
            run_path: Text::of(".github/workflows/policy.yml"),
        }
    }

    /// The number of commits the pull request itself reports.
    fn reported_commits(&self) -> Count {
        match self.commit_count {
            Some(count) => count,
            None => Count::new(self.commit_entries.len()),
        }
    }
}

/// Who a commit names as its author or committer.
enum Account {
    /// An account with this login.
    Linked(Name),
    /// An email that is linked to no account: the API gives `null`.
    Unlinked,
}

impl Account {
    fn json(&self) -> Text {
        match self {
            Self::Unlinked => Text::of("null"),
            Self::Linked(login) => {
                Text::from_string(format!("{{\"login\":\"{}\"}}", login.as_str()))
            }
        }
    }
}

/// A commit as the API lists it.
struct Commit {
    author: Account,
    committer: Account,
}

impl Commit {
    fn listed(&self) -> Text {
        Text::from_string(format!(
            "{{\"author\":{},\"committer\":{}}}",
            self.author.json().as_str(),
            self.committer.json().as_str()
        ))
    }
}

fn account(login: &Name) -> Account {
    Account::Linked(login.clone())
}

fn normal_commit() -> Text {
    Commit {
        author: account(&Name::of("author")),
        committer: account(&Name::of("author")),
    }
    .listed()
}

fn web_editor_commit() -> Text {
    Commit {
        author: account(&Name::of("author")),
        committer: account(&Name::of("web-flow")),
    }
    .listed()
}

fn unlinked_author_commit() -> Text {
    Commit {
        author: Account::Unlinked,
        committer: account(&Name::of("author")),
    }
    .listed()
}

fn unlinked_committer_commit() -> Text {
    Commit {
        author: account(&Name::of("author")),
        committer: Account::Unlinked,
    }
    .listed()
}

fn unlinked_both_commit() -> Text {
    Commit {
        author: Account::Unlinked,
        committer: Account::Unlinked,
    }
    .listed()
}

fn head() -> Sha {
    sha!('a')
}

fn pull() -> PullNumber {
    PullNumber::from_count(Count::new(7))
}

fn workflow() -> Reason {
    Reason::at(
        Code::WorkflowChanged,
        &Text::of(".github/workflows/ci.yml"),
        &RelPath::of(".github/workflows/ci.yml"),
        Count::new(1),
    )
}

fn inputs() -> InputResults {
    Job::ALL
        .into_iter()
        .map(|job| (job, Text::of("success")))
        .collect()
}

/// The decision of a policy run for the head `at`.
fn verdict_data(
    reasons: Vec<Reason>,
    verdict: Verdict,
    approvable: Approvable,
    at: &Sha,
) -> VerdictData {
    VerdictData {
        approvable,
        attempt: Count::new(1),
        blockers: Vec::new(),
        commits: Commits {
            base: sha!('b'),
            head: at.clone(),
        },
        counts: Counts::empty(),
        inputs: inputs(),
        policy_commit: sha!('c'),
        pull: pull(),
        reasons,
        reviewed: None,
        run_id: Count::new(5),
        verdict,
    }
}

fn needs_human_run() -> Run {
    Run::of(verdict_data(
        vec![workflow()],
        Verdict::NeedsHuman,
        Approvable::Yes,
        &head(),
    ))
}

fn status_of(world: &World) -> Result<ReviewStatus, Fail> {
    Ok(world.go()?.0)
}

#[test]
fn owner_approval_flips_check_to_success() {
    assert_eq!(
        check_owner_approval_flips_check_to_success(),
        Ok(()),
        "owner_approval_flips_check_to_success"
    );
}

fn check_owner_approval_flips_check_to_success() -> Result<(), Fail> {
    let (status, fake) = World::new(Some(needs_human_run())).go()?;
    ensure_eq!(
        status,
        ReviewStatus::Updated(Conclusion::Success),
        "check flips"
    );
    let sent = fake.sent();
    ensure_eq!(sent.len(), 1, "one write");
    let first = sent.first().map(|write| {
        (
            write.method,
            write.body.text_at(&Name::of("conclusion")).cloned(),
        )
    });
    ensure_eq!(
        first,
        Some((Method::Patch, Some(Text::of("success")))),
        "the existing run is patched to success"
    );
    Ok(())
}

#[test]
fn stale_approval_leaves_failure() {
    assert_eq!(
        check_stale_approval_leaves_failure(),
        Ok(()),
        "stale_approval_leaves_failure"
    );
}

fn check_stale_approval_leaves_failure() -> Result<(), Fail> {
    let mut world = World::new(Some(needs_human_run()));
    world.review_commit = Text::of(sha!('d').as_str());
    let (status, fake) = world.go()?;
    ensure_eq!(
        status,
        ReviewStatus::Unchanged(Conclusion::Failure),
        "stays failed"
    );
    ensure_eq!(fake.sent().len(), 0, "nothing written");
    Ok(())
}

#[test]
fn fake_check_from_another_app_is_ignored() {
    assert_eq!(
        check_fake_check_from_another_app_is_ignored(),
        Ok(()),
        "fake_check_from_another_app_is_ignored"
    );
}

fn check_fake_check_from_another_app_is_ignored() -> Result<(), Fail> {
    let mut run = needs_human_run();
    run.slug = Text::of("github-actions");
    ensure_eq!(
        status_of(&World::new(Some(run)))?,
        ReviewStatus::NoPolicyCheck,
        "only the App's check counts"
    );
    Ok(())
}

#[test]
fn verdict_data_for_older_head_is_ignored() {
    assert_eq!(
        check_verdict_data_for_older_head_is_ignored(),
        Ok(()),
        "verdict_data_for_older_head_is_ignored"
    );
}

fn check_verdict_data_for_older_head_is_ignored() -> Result<(), Fail> {
    let old = Run::of(verdict_data(
        vec![workflow()],
        Verdict::NeedsHuman,
        Approvable::Yes,
        &sha!('e'),
    ));
    ensure_eq!(
        status_of(&World::new(Some(old)))?,
        ReviewStatus::NoVerdictData,
        "data for another head is not trusted"
    );
    Ok(())
}

#[test]
fn failed_input_job_is_not_approvable() {
    assert_eq!(
        check_failed_input_job_is_not_approvable(),
        Ok(()),
        "failed_input_job_is_not_approvable"
    );
}

fn check_failed_input_job_is_not_approvable() -> Result<(), Fail> {
    let mut world = World::new(Some(needs_human_run()));
    world.failed_job = Some(Job::Mutants);
    ensure_eq!(
        status_of(&world)?,
        ReviewStatus::Unchanged(Conclusion::Failure),
        "a failed input blocks approval"
    );
    Ok(())
}

#[test]
fn run_of_another_workflow_file_is_rejected() {
    assert_eq!(
        check_run_of_another_workflow_file_is_rejected(),
        Ok(()),
        "run_of_another_workflow_file_is_rejected"
    );
}

fn check_run_of_another_workflow_file_is_rejected() -> Result<(), Fail> {
    let mut world = World::new(Some(needs_human_run()));
    world.run_path = Text::of(".github/workflows/evil.yml");
    ensure_eq!(
        status_of(&world)?,
        ReviewStatus::Unchanged(Conclusion::Failure),
        "only policy.yml runs count"
    );
    Ok(())
}

#[test]
fn unapprovable_verdict_is_not_touched() {
    assert_eq!(
        check_unapprovable_verdict_is_not_touched(),
        Ok(()),
        "unapprovable_verdict_is_not_touched"
    );
}

fn check_unapprovable_verdict_is_not_touched() -> Result<(), Fail> {
    let run = Run::of(verdict_data(
        vec![workflow()],
        Verdict::NeedsHuman,
        Approvable::No,
        &head(),
    ));
    ensure_eq!(
        status_of(&World::new(Some(run)))?,
        ReviewStatus::NothingToApprove,
        "blockers cannot be approved"
    );
    Ok(())
}

#[test]
fn auto_verdict_is_not_touched() {
    assert_eq!(
        check_auto_verdict_is_not_touched(),
        Ok(()),
        "auto_verdict_is_not_touched"
    );
}

fn check_auto_verdict_is_not_touched() -> Result<(), Fail> {
    let run = Run::of(verdict_data(
        Vec::new(),
        Verdict::Auto,
        Approvable::Yes,
        &head(),
    ));
    ensure_eq!(
        status_of(&World::new(Some(run)))?,
        ReviewStatus::NothingToApprove,
        "auto needs no approval"
    );
    Ok(())
}

#[test]
fn other_base_branch_skipped() {
    assert_eq!(
        check_other_base_branch_skipped(),
        Ok(()),
        "other_base_branch_skipped"
    );
}

fn check_other_base_branch_skipped() -> Result<(), Fail> {
    let mut world = World::new(Some(needs_human_run()));
    world.base_ref = Text::of("attacker-branch");
    ensure_eq!(
        status_of(&world)?,
        ReviewStatus::Skipped,
        "other bases are skipped"
    );
    Ok(())
}

#[test]
fn revoked_approval_flips_back() {
    assert_eq!(
        check_revoked_approval_flips_back(),
        Ok(()),
        "revoked_approval_flips_back"
    );
}

fn check_revoked_approval_flips_back() -> Result<(), Fail> {
    let mut run = needs_human_run();
    run.conclusion = Text::of("success");
    let mut world = World::new(Some(run));
    world.extra_review = Some(Text::of("DISMISSED"));
    ensure_eq!(
        status_of(&world)?,
        ReviewStatus::Updated(Conclusion::Failure),
        "a dismissal flips the check back"
    );
    Ok(())
}

#[test]
fn a_verdict_for_another_base_or_policy_is_stale() {
    assert_eq!(
        check_a_verdict_for_another_base_or_policy_is_stale(),
        Ok(()),
        "a_verdict_for_another_base_or_policy_is_stale"
    );
}

fn check_a_verdict_for_another_base_or_policy_is_stale() -> Result<(), Fail> {
    let mut moved = World::new(Some(needs_human_run()));
    moved.base_tip = sha!('e');
    let (status, fake) = moved.go()?;
    ensure_eq!(
        status,
        ReviewStatus::Stale,
        "the base branch moved since the verdict, so the approval is not applied"
    );
    ensure_eq!(
        fake.sent().len(),
        0,
        "nothing is written for a stale verdict"
    );
    let mut changed = World::new(Some(needs_human_run()));
    changed.policy_commit = sha!('f');
    let (changed_status, changed_fake) = changed.go()?;
    ensure_eq!(
        changed_status,
        ReviewStatus::Stale,
        "another policy commit than the one that decided is stale too"
    );
    ensure_eq!(changed_fake.sent().len(), 0, "nothing is written");
    ensure_eq!(
        status_of(&World::new(Some(needs_human_run())))?,
        ReviewStatus::Updated(Conclusion::Success),
        "the same base and policy still flip the check"
    );
    Ok(())
}

#[test]
fn a_stale_success_is_turned_back_to_failure() {
    assert_eq!(
        check_a_stale_success_is_turned_back_to_failure(),
        Ok(()),
        "a_stale_success_is_turned_back_to_failure"
    );
}

fn check_a_stale_success_is_turned_back_to_failure() -> Result<(), Fail> {
    let mut run = needs_human_run();
    run.conclusion = Text::of("success");
    let mut world = World::new(Some(run));
    world.base_tip = sha!('e');
    let (status, fake) = world.go()?;
    ensure_eq!(
        status,
        ReviewStatus::Updated(Conclusion::Failure),
        "an old success does not stand on a moved base"
    );
    let sent = fake.sent();
    let first = sent
        .first()
        .map(|write| write.body.text_at(&Name::of("conclusion")).cloned());
    ensure_eq!(
        first,
        Some(Some(Text::of("failure"))),
        "the run is patched to failure"
    );
    Ok(())
}

#[test]
fn the_sweep_lists_every_open_pull_request_or_fails() {
    assert_eq!(
        check_the_sweep_lists_every_open_pull_request_or_fails(),
        Ok(()),
        "the_sweep_lists_every_open_pull_request_or_fails"
    );
}

/// A page of `page.count` pull requests numbered from `page.first`.
fn pull_page(page: &Page) -> Text {
    let items: Vec<String> = (page.first.get()..)
        .take(page.count.get())
        .map(|number| format!("{{\"number\":{number}}}"))
        .collect();
    Text::from_string(format!("[{}]", items.join(",")))
}

fn check_the_sweep_lists_every_open_pull_request_or_fails() -> Result<(), Fail> {
    let branch = Name::of("main");
    let fake = FakeGithub::new()
        .route(
            &Text::of("&page=1"),
            &json_of(
                &(pull_page(&Page {
                    count: Count::new(100),
                    first: Count::new(1),
                })),
            )?,
        )
        .route(
            &Text::of("&page=2"),
            &json_of(
                &(pull_page(&Page {
                    count: Count::new(100),
                    first: Count::new(101),
                })),
            )?,
        )
        .route(
            &Text::of("&page=3"),
            &json_of(
                &(pull_page(&Page {
                    count: Count::new(7),
                    first: Count::new(201),
                })),
            )?,
        );
    let numbers = open_pull_numbers(&fake, &branch)?;
    ensure_eq!(
        numbers.len(),
        207,
        "the sweep reads past the first hundred pull requests"
    );
    let endless = FakeGithub::new().route(
        &Text::of("/pulls?"),
        &json_of(
            &(pull_page(&Page {
                count: Count::new(100),
                first: Count::new(1),
            })),
        )?,
    );
    ensure!(
        open_pull_numbers(&endless, &branch).is_err(),
        "a listing that never ends is an error, not the first two thousand"
    );
    let malformed = FakeGithub::new().route(
        &Text::of("/pulls?"),
        &json_of(&Text::of("{\"message\":\"oops\"}"))?,
    );
    ensure!(
        open_pull_numbers(&malformed, &branch).is_err(),
        "a page that is not a list is an error, not an empty sweep"
    );
    Ok(())
}

#[test]
fn a_reviewer_who_committed_to_the_pull_request_does_not_approve_it() {
    assert_eq!(
        check_a_reviewer_who_committed_to_the_pull_request_does_not_approve_it(),
        Ok(()),
        "a_reviewer_who_committed_to_the_pull_request_does_not_approve_it"
    );
}

fn check_a_reviewer_who_committed_to_the_pull_request_does_not_approve_it() -> Result<(), Fail> {
    let mut run = needs_human_run();
    run.conclusion = Text::of("success");
    let mut world = World::new(Some(run));
    world.commit_entries = vec![Commit {
        author: account(&Name::of("author")),
        committer: account(&Name::of("Maint")),
    }
    .listed()];
    ensure_eq!(
        status_of(&world)?,
        ReviewStatus::Updated(Conclusion::Failure),
        "an approval by someone who pushed to the pull request is not counted"
    );
    let fresh = World::new(Some(needs_human_run()));
    ensure_eq!(
        status_of(&fresh)?,
        ReviewStatus::Updated(Conclusion::Success),
        "the same approval counts when the owner pushed nothing"
    );
    Ok(())
}

#[test]
fn one_failing_pull_request_does_not_stop_the_sweep() {
    assert_eq!(
        check_one_failing_pull_request_does_not_stop_the_sweep(),
        Ok(()),
        "one_failing_pull_request_does_not_stop_the_sweep"
    );
}

fn check_one_failing_pull_request_does_not_stop_the_sweep() -> Result<(), Fail> {
    // pull request 7 has a stale success that must be downgraded; the fake has
    // no pull request 8, so evaluating it is an API error, and it comes first
    let mut run = needs_human_run();
    run.conclusion = Text::of("success");
    let mut world = World::new(Some(run));
    world.extra_review = Some(Text::of("DISMISSED"));
    let fake = world.fake()?;
    let failing = PullNumber::from_count(Count::new(8));
    let done = sweep(
        &Clients {
            app: &fake,
            reader: &fake,
        },
        &[failing, pull()],
        &Running {
            default_branch: Name::of("main"),
            policy_commit: world.policy_commit.clone(),
        },
    );
    ensure_eq!(done.failed, Answer::Yes, "the sweep fails at the end");
    ensure_eq!(done.lines.len(), 2, "both pull requests are reported");
    ensure!(
        done.lines.first().is_some_and(|line| line
            .as_str()
            .starts_with("PR #8: could not be re-evaluated")),
        "the failure is listed: {:?}",
        done.lines
    );
    ensure_eq!(
        done.lines.get(1).cloned(),
        Some(Text::of("PR #7: updated to failure")),
        "the later pull request was still downgraded"
    );
    ensure_eq!(fake.sent().len(), 1, "its check run was written");
    let clean = sweep(
        &Clients {
            app: &fake,
            reader: &fake,
        },
        &[],
        &Running {
            default_branch: Name::of("main"),
            policy_commit: world.policy_commit,
        },
    );
    ensure_eq!(clean.failed, Answer::No, "nothing to do is not a failure");
    Ok(())
}

#[test]
fn a_fork_review_finds_its_pull_request_by_the_head_commit() {
    assert_eq!(
        check_a_fork_review_finds_its_pull_request_by_the_head_commit(),
        Ok(()),
        "a_fork_review_finds_its_pull_request_by_the_head_commit"
    );
}

fn check_a_fork_review_finds_its_pull_request_by_the_head_commit() -> Result<(), Fail> {
    let listing = |state: &str, base: &str, head: &Sha, number: usize| {
        format!(
            "{{\"number\":{number},\"state\":\"{state}\",\"head\":{{\"sha\":\"{}\"}},\"base\":{{\"ref\":\"{base}\"}},\"user\":{{\"login\":\"u\"}}}}",
            head.as_str()
        )
    };
    let body = [
        listing("open", "main", &head(), 7),
        listing("open", "other", &head(), 9),
        listing("closed", "main", &head(), 11),
        listing("open", "main", &sha!('d'), 12),
    ]
    .join(",");
    let fake = FakeGithub::new().route(
        &Text::of("/commits/"),
        &json_of(&Text::from_string(format!("[{body}]")))?,
    );
    ensure_eq!(
        open_pulls_with_head(&fake, &Name::of("main"), &head())?,
        vec![pull()],
        "only the open pull request into the default branch whose head is that commit"
    );
    Ok(())
}

#[test]
fn commits_that_cannot_be_attributed_are_never_approvable() {
    assert_eq!(
        check_commits_that_cannot_be_attributed_are_never_approvable(),
        Ok(()),
        "commits_that_cannot_be_attributed_are_never_approvable"
    );
}

fn check_commits_that_cannot_be_attributed_are_never_approvable() -> Result<(), Fail> {
    let judged = |entries: Vec<Text>, count: Option<Count>| -> Result<ReviewStatus, Fail> {
        let mut world = World::new(Some(needs_human_run()));
        world.commit_entries = entries;
        world.commit_count = count;
        status_of(&world)
    };
    ensure_eq!(
        judged(vec![normal_commit()], None)?,
        ReviewStatus::Updated(Conclusion::Success),
        "every commit has an author and a committer login"
    );
    ensure_eq!(
        judged(vec![web_editor_commit()], None)?,
        ReviewStatus::Updated(Conclusion::Success),
        "a commit made in the GitHub web editor (committer web-flow) is attributable"
    );
    ensure_eq!(
        judged(vec![unlinked_author_commit()], None)?,
        ReviewStatus::Updated(Conclusion::Failure),
        "an author with no account is not attributable"
    );
    ensure_eq!(
        judged(vec![unlinked_committer_commit()], None)?,
        ReviewStatus::Updated(Conclusion::Failure),
        "a committer with no account is not attributable"
    );
    ensure_eq!(
        judged(vec![normal_commit(), unlinked_both_commit()], None)?,
        ReviewStatus::Updated(Conclusion::Failure),
        "one unattributable commit among attributable ones is enough"
    );
    ensure_eq!(
        judged(vec![normal_commit()], Some(Count::new(251)))?,
        ReviewStatus::Updated(Conclusion::Failure),
        "a listing shorter than the commit count of the pull request (the 250 cap) is refused"
    );
    ensure_eq!(
        judged(vec![normal_commit(), normal_commit()], Some(Count::new(1)))?,
        ReviewStatus::Updated(Conclusion::Failure),
        "a listing longer than the commit count is refused too"
    );
    Ok(())
}

#[test]
fn the_check_run_says_why_commits_cannot_be_attributed() {
    assert_eq!(
        check_the_check_run_says_why_commits_cannot_be_attributed(),
        Ok(()),
        "the_check_run_says_why_commits_cannot_be_attributed"
    );
}

fn check_the_check_run_says_why_commits_cannot_be_attributed() -> Result<(), Fail> {
    let mut stated = World::new(Some(needs_human_run()));
    stated.commit_entries = vec![unlinked_author_commit()];
    let (status, fake) = stated.go()?;
    ensure_eq!(
        status,
        ReviewStatus::Updated(Conclusion::Failure),
        "the check run is rewritten to say why"
    );
    let sent = fake.sent();
    let says_what_to_do = sent
        .first()
        .and_then(|write| write.body.dig(&keys!["output", "summary"]))
        .and_then(Json::as_text)
        .is_some_and(|text| {
            text.as_str().contains("linked to no GitHub account")
                && text.as_str().contains("merge by hand")
        });
    ensure!(says_what_to_do, "the summary tells the author what to do");
    Ok(())
}
