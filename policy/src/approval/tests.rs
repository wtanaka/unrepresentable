use super::{
    evaluate, Attribution, Contributors, Evaluation, Inputs, Outcome, Problem, Review, ReviewState,
};
use crate::codeowners::{owners_for, parse, Rule};
use crate::json;
use crate::reason::{Code, Reason};
use crate::types::{Answer, Count, Name, RelPath, Text};

macro_rules! login {
    ($text:expr_2021) => {
        Name::of($text)
    };
}

const RULES: &str = "* @maint\n/policy/ @polowner\n/.github/ @polowner @maint\n";

/// A change under review, with everything approval depends on.
struct Scenario {
    author: Name,
    contributors: Contributors,
    inputs: Inputs,
    reasons: Vec<Reason>,
    reviews: Vec<Review>,
    rules: Vec<Rule>,
}

impl Scenario {
    fn authored_by(self, author: &Name) -> Self {
        Self {
            author: author.clone(),
            ..self
        }
    }

    fn is_approved(&self) -> Answer {
        match self.outcome() {
            Outcome::Approved {
                approvers: _approvers,
            } => Answer::Yes,
            Outcome::NotApproved {
                problems: _problems,
            } => Answer::No,
        }
    }

    fn new(reasons: Vec<Reason>, reviews: Vec<Review>) -> Self {
        Self {
            author: Name::of("author"),
            contributors: Contributors {
                attribution: Attribution::Complete,
                logins: Vec::new(),
            },
            inputs: Inputs::AllSucceeded,
            reasons,
            reviews,
            rules: rules(),
        }
    }

    fn outcome(&self) -> Outcome {
        evaluate(&Evaluation {
            author: &self.author,
            contributors: &self.contributors,
            head: &sha!('a'),
            inputs: self.inputs,
            reasons: &self.reasons,
            reviews: &self.reviews,
            rules: &self.rules,
        })
    }

    fn pushed_by(self, login: &Name) -> Self {
        Self {
            contributors: Contributors {
                attribution: Attribution::Complete,
                logins: vec![login.clone()],
            },
            ..self
        }
    }

    fn unattributable(self) -> Self {
        Self {
            contributors: Contributors {
                attribution: Attribution::Incomplete,
                logins: Vec::new(),
            },
            ..self
        }
    }

    fn with_failed_inputs(self) -> Self {
        Self {
            inputs: Inputs::SomeFailed,
            ..self
        }
    }

    fn with_rules(self, rules: Vec<Rule>) -> Self {
        Self { rules, ..self }
    }
}

/// The rules of `text`; a parse failure leaves no rules, which fails the test.
fn parsed_rules(text: &Text) -> Vec<Rule> {
    let Ok(found) = parse(text) else {
        return Vec::new();
    };
    found
}

fn rules() -> Vec<Rule> {
    parsed_rules(&Text::of(RULES))
}

/// An approval of the current head by `login`, a human.
fn approval(login: &Name) -> Review {
    Review {
        commit: Text::of(sha!('a').as_str()),
        login: login.clone(),
        state: ReviewState::Approved,
        user_type: Text::of("User"),
    }
}

fn workflow() -> Reason {
    Reason::at(
        Code::WorkflowChanged,
        &Text::of(".github/workflows/ci.yml"),
        &RelPath::of(".github/workflows/ci.yml"),
        Count::new(1),
    )
}

fn lost() -> Reason {
    Reason::at(
        Code::LostFinding,
        &Text::of("rule x"),
        &RelPath::of("src/rules.rs"),
        Count::new(3),
    )
}

#[test]
fn last_match_wins() {
    let rules = rules();
    let owners = |path: &str| owners_for(&rules, &RelPath::of(path));
    assert_eq!(
        owners("policy/a.py"),
        vec![login!("@polowner")],
        "policy owner"
    );
    assert_eq!(owners("src/a.rs"), vec![login!("@maint")], "default owner");
    assert_eq!(
        owners(".github/workflows/x.yml"),
        vec![login!("@polowner"), login!("@maint")],
        "two owners"
    );
}

#[test]
fn owner_approval_on_head() {
    let scenario = Scenario::new(vec![workflow()], vec![approval(&login!("maint"))]);
    assert_eq!(
        scenario.outcome(),
        Outcome::Approved {
            approvers: vec![login!("maint")]
        },
        "the owner approves the head"
    );
}

#[test]
fn case_insensitive_login() {
    let scenario = Scenario::new(vec![workflow()], vec![approval(&login!("Maint"))]);
    assert_eq!(
        scenario.is_approved(),
        Answer::Yes,
        "logins compare without case"
    );
}

#[test]
fn non_owner_cannot_approve() {
    let scenario = Scenario::new(vec![workflow()], vec![approval(&login!("stranger"))]);
    assert_eq!(
        scenario.is_approved(),
        Answer::No,
        "a stranger owns nothing"
    );
}

#[test]
fn author_cannot_self_approve() {
    for author in ["maint", "MAINT"] {
        let scenario = Scenario::new(vec![workflow()], vec![approval(&login!("maint"))])
            .authored_by(&login!(author));
        assert_eq!(
            scenario.is_approved(),
            Answer::No,
            "{author} cannot approve their own PR"
        );
    }
}

#[test]
fn bot_cannot_approve() {
    let bot_type = Review {
        user_type: Text::of("Bot"),
        ..approval(&login!("maint"))
    };
    assert_eq!(
        Scenario::new(vec![workflow()], vec![bot_type]).is_approved(),
        Answer::No,
        "a Bot account cannot approve"
    );
    let bot_name = approval(&login!("maint[bot]"));
    assert_eq!(
        Scenario::new(vec![workflow()], vec![bot_name]).is_approved(),
        Answer::No,
        "a [bot] login cannot approve"
    );
}

#[test]
fn approval_on_older_commit_does_not_count() {
    let stale = Review {
        commit: Text::of(sha!('b').as_str()),
        ..approval(&login!("maint"))
    };
    assert_eq!(
        Scenario::new(vec![workflow()], vec![stale]).is_approved(),
        Answer::No,
        "an approval of another commit is stale"
    );
}

#[test]
fn later_changes_requested_or_dismissed_wins() {
    let reversal = Review {
        state: ReviewState::Other,
        ..approval(&login!("maint"))
    };
    let scenario = Scenario::new(vec![workflow()], vec![approval(&login!("maint")), reversal]);
    assert_eq!(
        scenario.is_approved(),
        Answer::No,
        "a later review replaces the approval"
    );
}

#[test]
fn later_comment_does_not_erase_approval() {
    let comment = Review {
        state: ReviewState::Commented,
        ..approval(&login!("maint"))
    };
    let scenario = Scenario::new(vec![workflow()], vec![approval(&login!("maint")), comment]);
    assert_eq!(
        scenario.is_approved(),
        Answer::Yes,
        "a comment is not a verdict"
    );
}

#[test]
fn non_path_reasons_need_policy_owner() {
    assert_eq!(
        Scenario::new(vec![lost()], vec![approval(&login!("maint"))]).is_approved(),
        Answer::No,
        "the default owner does not own policy"
    );
    assert_eq!(
        Scenario::new(vec![lost()], vec![approval(&login!("polowner"))]).is_approved(),
        Answer::Yes,
        "the policy owner does"
    );
}

#[test]
fn unsafe_path_reason_needs_policy_owner() {
    let reason = Reason::general(Code::UnsafePathChars, &Text::of("x"));
    assert_eq!(
        Scenario::new(vec![reason.clone()], vec![approval(&login!("maint"))]).is_approved(),
        Answer::No,
        "the default owner does not own policy"
    );
    assert_eq!(
        Scenario::new(vec![reason], vec![approval(&login!("polowner"))]).is_approved(),
        Answer::Yes,
        "the policy owner does"
    );
}

#[test]
fn every_requirement_needs_cover() {
    let both = vec![
        Reason::at(
            Code::TestCodeChanged,
            &Text::of("tests/a.rs"),
            &RelPath::of("tests/a.rs"),
            Count::new(1),
        ),
        lost(),
    ];
    assert_eq!(
        Scenario::new(both.clone(), vec![approval(&login!("maint"))]).is_approved(),
        Answer::No,
        "one owner covers only one requirement"
    );
    let reviews = vec![approval(&login!("maint")), approval(&login!("polowner"))];
    assert_eq!(
        Scenario::new(both, reviews).is_approved(),
        Answer::Yes,
        "two owners cover both"
    );
}

#[test]
fn failed_input_jobs_are_not_approvable() {
    let scenario =
        Scenario::new(vec![workflow()], vec![approval(&login!("maint"))]).with_failed_inputs();
    match scenario.outcome() {
        Outcome::NotApproved { problems } => assert!(
            problems.contains(&Problem::InputJobsNotAllSuccessful),
            "the failed inputs are named"
        ),
        Outcome::Approved {
            approvers: _approvers,
        } => {
            assert_eq!(scenario.is_approved(), Answer::No, "must not approve");
        }
    }
}

#[test]
fn team_owner_fails_closed() {
    let scenario = Scenario::new(vec![workflow()], vec![approval(&login!("maint"))])
        .with_rules(parsed_rules(&Text::of("* @org/team\n")));
    assert_eq!(
        scenario.is_approved(),
        Answer::No,
        "teams are never resolved"
    );
}

#[test]
fn no_owner_rule_fails_closed() {
    let scenario =
        Scenario::new(vec![workflow()], vec![approval(&login!("maint"))]).with_rules(Vec::new());
    assert_eq!(scenario.is_approved(), Answer::No, "no rule means no owner");
}

#[test]
fn no_reviews() {
    assert_eq!(
        Scenario::new(vec![workflow()], Vec::new()).is_approved(),
        Answer::No,
        "nobody approved"
    );
}

#[test]
fn reviews_are_read_from_api_json() {
    let text = Text::of(
        "{\"user\":{\"login\":\"Maint\",\"type\":\"User\"},\"state\":\"APPROVED\",\"commit_id\":\"abc\"}",
    );
    let parsed = json::parse(&text).map(|value| Review::from_json(&value));
    assert_eq!(
        parsed,
        Ok(Some(Review {
            commit: Text::of("abc"),
            login: login!("Maint"),
            state: ReviewState::Approved,
            user_type: Text::of("User"),
        })),
        "fields are read"
    );
    let anonymous =
        json::parse(&Text::of("{\"state\":\"APPROVED\"}")).map(|value| Review::from_json(&value));
    assert_eq!(anonymous, Ok(None), "a review without a user is skipped");
}

#[test]
fn a_rules_code_change_needs_an_owner_of_that_path() {
    let reason = Reason::at(
        Code::RulesCodeChanged,
        &Text::of("src/rules.rs"),
        &RelPath::of("src/rules.rs"),
        Count::new(1),
    );
    assert_eq!(
        Code::RulesCodeChanged.is_path_based(),
        Answer::Yes,
        "the reason names a path"
    );
    assert_eq!(
        Scenario::new(vec![reason.clone()], Vec::new()).is_approved(),
        Answer::No,
        "no approval, no merge"
    );
    assert_eq!(
        Scenario::new(vec![reason.clone()], vec![approval(&login!("stranger"))]).is_approved(),
        Answer::No,
        "a reviewer who owns nothing does not count"
    );
    assert_eq!(
        Scenario::new(vec![reason], vec![approval(&login!("maint"))]).is_approved(),
        Answer::Yes,
        "the code owner of src/ approves it"
    );
}

#[test]
fn a_code_owner_who_pushed_a_commit_cannot_approve_it() {
    let owner = approval(&login!("maint"));
    assert_eq!(
        Scenario::new(vec![workflow()], vec![owner.clone()])
            .pushed_by(&login!("Maint"))
            .is_approved(),
        Answer::No,
        "the approver committed to the pull request (case does not matter)"
    );
    assert_eq!(
        Scenario::new(vec![workflow()], vec![owner])
            .pushed_by(&login!("someone-else"))
            .is_approved(),
        Answer::Yes,
        "another contributor does not stop an owner who did not push"
    );
}

#[test]
fn commits_without_a_known_login_are_not_approvable() {
    let scenario =
        Scenario::new(vec![workflow()], vec![approval(&login!("maint"))]).unattributable();
    match scenario.outcome() {
        Outcome::NotApproved { problems } => assert!(
            problems.contains(&Problem::CommitsNotAttributable),
            "the unattributable commits are named"
        ),
        Outcome::Approved {
            approvers: _approvers,
        } => {
            assert_eq!(scenario.is_approved(), Answer::No, "must not approve");
        }
    }
}
