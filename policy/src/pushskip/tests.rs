use super::{judge, Decision, Judged};
use crate::testkit::{json_of, Fail, FakeGithub};
use crate::types::{Sha, Text};

/// A check run as the API reports it.
struct Run {
    app: Text,
    conclusion: Text,
    status: Text,
}

impl Run {
    fn completed(conclusion: &Text) -> Self {
        Self {
            app: Text::of("unrepresentable-policy"),
            conclusion: conclusion.clone(),
            status: Text::of("completed"),
        }
    }

    fn json(&self, head: &Sha) -> Text {
        Text::from_string(format!(
            "{{\"name\":\"policy\",\"head_sha\":\"{}\",\"app\":{{\"slug\":\"{}\"}},\"status\":\"{}\",\"conclusion\":\"{}\"}}",
            head.as_str(),
            self.app.as_str(),
            self.status.as_str(),
            self.conclusion.as_str()
        ))
    }

    fn passing(head: &Sha) -> Text {
        Self::completed(&Text::of("success")).json(head)
    }
}

/// Whether the pushed commit and the pull request head have the same tree.
#[derive(Clone, Copy)]
enum Trees {
    Different,
    Same,
}

/// What the fake GitHub knows. A commit, pull list or check list that is left
/// out answers with an error, like an API failure.
struct World {
    /// The check runs of a head, as JSON objects.
    checks: Vec<(Sha, Vec<Text>)>,
    /// The heads of the pull requests that contain the pushed commit.
    pulls: Option<Vec<Sha>>,
    /// The tree of a commit.
    trees: Vec<(Sha, Sha)>,
}

impl World {
    fn fake(&self) -> Result<FakeGithub, Fail> {
        let mut fake = FakeGithub::new();
        for (commit, tree) in &self.trees {
            fake = fake.route(
                &Text::from_string(format!("/git/commits/{}", commit.as_str())),
                &json_of(&Text::from_string(format!(
                    "{{\"tree\":{{\"sha\":\"{}\"}}}}",
                    tree.as_str()
                )))?,
            );
        }
        if let Some(heads) = &self.pulls {
            let items: Vec<String> = heads
                .iter()
                .map(|head| format!("{{\"head\":{{\"sha\":\"{}\"}}}}", head.as_str()))
                .collect();
            fake = fake.route(
                &Text::from_string(format!("/commits/{}/pulls", pushed().as_str())),
                &json_of(&Text::from_string(format!("[{}]", items.join(","))))?,
            );
        }
        for (head, runs) in &self.checks {
            fake = fake.route(
                &Text::from_string(format!("/commits/{}/check-runs", head.as_str())),
                &json_of(&Text::from_string(format!(
                    "{{\"check_runs\":[{}]}}",
                    runs.iter()
                        .map(Text::as_str)
                        .collect::<Vec<&str>>()
                        .join(",")
                )))?,
            );
        }
        Ok(fake)
    }

    fn judged(&self) -> Result<Judged, Fail> {
        Ok(judge(&self.fake()?, &pushed()))
    }

    /// The pull request head `head_a` with `runs`; its tree is the pushed
    /// commit's tree or another one.
    fn one_pull(trees: Trees, runs: Vec<Text>) -> Self {
        let head_tree = match trees {
            Trees::Same => tree_x(),
            Trees::Different => tree_y(),
        };
        Self {
            checks: vec![(head_a(), runs)],
            pulls: Some(vec![head_a()]),
            trees: vec![(pushed(), tree_x()), (head_a(), head_tree)],
        }
    }
}

fn pushed() -> Sha {
    sha!('1')
}

fn head_a() -> Sha {
    sha!('2')
}

fn head_b() -> Sha {
    sha!('3')
}

fn tree_x() -> Sha {
    sha!('e')
}

fn tree_y() -> Sha {
    sha!('f')
}

fn decision_of(world: &World) -> Result<Decision, Fail> {
    Ok(world.judged()?.decision)
}

#[test]
fn equal_tree_and_success_skips() {
    assert_eq!(
        check_equal_tree_and_success_skips(),
        Ok(()),
        "equal_tree_and_success_skips"
    );
}

fn check_equal_tree_and_success_skips() -> Result<(), Fail> {
    let judged = World::one_pull(Trees::Same, vec![Run::passing(&head_a())]).judged()?;
    ensure_eq!(judged.decision, Decision::Skip, "skips");
    ensure_eq!(
        judged.lines().first().cloned(),
        Some(Text::of("skip=yes")),
        "first output line"
    );
    Ok(())
}

#[test]
fn different_tree_runs() {
    assert_eq!(check_different_tree_runs(), Ok(()), "different_tree_runs");
}

fn check_different_tree_runs() -> Result<(), Fail> {
    let world = World::one_pull(Trees::Different, vec![Run::passing(&head_a())]);
    ensure_eq!(decision_of(&world)?, Decision::Run, "a different tree runs");
    Ok(())
}

#[test]
fn missing_check_run_runs() {
    assert_eq!(
        check_missing_check_run_runs(),
        Ok(()),
        "missing_check_run_runs"
    );
}

fn check_missing_check_run_runs() -> Result<(), Fail> {
    let world = World::one_pull(Trees::Same, Vec::new());
    ensure_eq!(decision_of(&world)?, Decision::Run, "no check run runs");
    Ok(())
}

#[test]
fn failed_or_unfinished_check_runs() {
    assert_eq!(
        check_failed_or_unfinished_check_runs(),
        Ok(()),
        "failed_or_unfinished_check_runs"
    );
}

fn check_failed_or_unfinished_check_runs() -> Result<(), Fail> {
    let head = head_a();
    let other_app = Run {
        app: Text::of("someone-else"),
        conclusion: Text::of("success"),
        status: Text::of("completed"),
    };
    let cases = [
        (
            "failed",
            vec![Run::completed(&Text::of("failure")).json(&head)],
        ),
        (
            "neutral",
            vec![Run::completed(&Text::of("neutral")).json(&head)],
        ),
        (
            "unfinished",
            vec![Run {
                status: Text::of("in_progress"),
                ..Run::completed(&Text::of("success"))
            }
            .json(&head)],
        ),
        ("another app", vec![other_app.json(&head)]),
        (
            "one failure among runs",
            vec![
                Run::passing(&head),
                Run::completed(&Text::of("failure")).json(&head),
            ],
        ),
    ];
    for (label, runs) in cases {
        let world = World::one_pull(Trees::Same, runs);
        ensure_eq!(decision_of(&world)?, Decision::Run, "{label} runs");
    }
    Ok(())
}

#[test]
fn api_errors_run() {
    assert_eq!(check_api_errors_run(), Ok(()), "api_errors_run");
}

fn check_api_errors_run() -> Result<(), Fail> {
    let empty = World {
        checks: Vec::new(),
        pulls: None,
        trees: Vec::new(),
    };
    ensure_eq!(decision_of(&empty)?, Decision::Run, "no reply at all");
    let pushed_only = World {
        checks: Vec::new(),
        pulls: None,
        trees: vec![(pushed(), tree_x())],
    };
    ensure_eq!(decision_of(&pushed_only)?, Decision::Run, "pull list fails");
    let no_head_commit = World {
        checks: vec![(head_a(), vec![Run::passing(&head_a())])],
        pulls: Some(vec![head_a()]),
        trees: vec![(pushed(), tree_x())],
    };
    ensure_eq!(
        decision_of(&no_head_commit)?,
        Decision::Run,
        "head commit lookup fails"
    );
    let no_checks = World {
        checks: Vec::new(),
        pulls: Some(vec![head_a()]),
        trees: vec![(pushed(), tree_x()), (head_a(), tree_x())],
    };
    ensure_eq!(
        decision_of(&no_checks)?,
        Decision::Run,
        "check run lookup fails"
    );
    let damaged = FakeGithub::new().route(
        &Text::from_string(format!("/git/commits/{}", pushed().as_str())),
        &json_of(&Text::of("{\"tree\":{\"sha\":\"nope\"}}"))?,
    );
    ensure_eq!(
        judge(&damaged, &pushed()).decision,
        Decision::Run,
        "damaged tree id"
    );
    Ok(())
}

#[test]
fn no_pull_request_runs() {
    assert_eq!(check_no_pull_request_runs(), Ok(()), "no_pull_request_runs");
}

fn check_no_pull_request_runs() -> Result<(), Fail> {
    let world = World {
        checks: Vec::new(),
        pulls: Some(Vec::new()),
        trees: vec![(pushed(), tree_x())],
    };
    ensure_eq!(
        decision_of(&world)?,
        Decision::Run,
        "no associated pull request"
    );
    Ok(())
}

#[test]
fn a_later_pull_request_can_match() {
    assert_eq!(
        check_a_later_pull_request_can_match(),
        Ok(()),
        "a_later_pull_request_can_match"
    );
}

fn check_a_later_pull_request_can_match() -> Result<(), Fail> {
    let world = World {
        checks: vec![(head_b(), vec![Run::passing(&head_b())])],
        pulls: Some(vec![head_a(), head_b()]),
        trees: vec![
            (pushed(), tree_x()),
            (head_a(), tree_y()),
            (head_b(), tree_x()),
        ],
    };
    ensure_eq!(
        decision_of(&world)?,
        Decision::Skip,
        "the second head matches"
    );
    Ok(())
}

#[test]
fn a_run_note_is_one_line() {
    assert_eq!(
        check_a_run_note_is_one_line(),
        Ok(()),
        "a_run_note_is_one_line"
    );
}

fn check_a_run_note_is_one_line() -> Result<(), Fail> {
    let judged = judge(&FakeGithub::new(), &pushed());
    ensure!(!judged.note.as_str().contains('\n'), "one line");
    ensure_eq!(judged.lines().len(), 2, "a skip line and a note line");
    ensure_eq!(
        Judged::run(&Text::of("x")).lines().first().cloned(),
        Some(Text::of("skip=no")),
        "run says no"
    );
    Ok(())
}
