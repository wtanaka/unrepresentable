//! Approval evaluation. Pure functions; the callers fetch everything from the
//! GitHub API for one exact head commit.
//!
//! Conditions (all):
//!  1. the reviewer's latest effective review is APPROVED on commit == head;
//!  2. the reviewer owns (per the BASE-branch CODEOWNERS) the path of every
//!     path-based reason; non-path reasons need an owner of `policy/**`;
//!  3. the reviewer is not the PR author, did not author or commit any commit
//!     of the PR (a code owner who pushes to someone else's PR cannot approve
//!     the result), and is not a bot;
//!  4. every commit of the PR names a login for its author and for its
//!     committer, and the commits listed are all the PR has: otherwise the
//!     contributors cannot be known and nothing is approvable;
//!  5. every input job of the latest policy run for that head succeeded.
//!
//! `/approve` comments are never read. Team owners are never resolved: a rule
//! that names only teams approves nothing.

#[cfg(test)]
mod tests;

use crate::codeowners::{owners_for, Rule};
use crate::json::Json;
use crate::reason::Reason;
use crate::types::{Answer, Count, Holds as _, Name, RelPath, Sha, Text};
use alloc::collections::{BTreeMap, BTreeSet};

/// A path every non-path reason is judged against: `policy/**` owners decide.
const POLICY_PROBE: &str = "policy/x";

/// The state of a pull request review.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReviewState {
    /// The review approves the pull request.
    Approved,
    /// The review is a comment without a verdict.
    Commented,
    /// Any other review state, such as dismissed or changes requested.
    Other,
    /// The review has not been submitted yet.
    Pending,
}

/// One review of a pull request, as the API reports it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Review {
    /// The commit the review was written against.
    pub commit: Text,
    /// The login of the reviewer.
    pub login: Name,
    /// The state of the review.
    pub state: ReviewState,
    /// The account type the API reports, such as `User` or `Bot`.
    pub user_type: Text,
}

impl Review {
    /// Reads a review object; `None` when it names no user.
    #[must_use]
    pub(crate) fn from_json(item: &Json) -> Option<Self> {
        let user = item.get(&Name::of("user"))?;
        let login = user.text_at(&Name::of("login"))?;
        if login.as_str().is_empty() {
            return None;
        }
        let state = match item.text_at(&Name::of("state")).map(Text::as_str) {
            Some("APPROVED") => ReviewState::Approved,
            Some("COMMENTED") => ReviewState::Commented,
            Some("PENDING") => ReviewState::Pending,
            Some(_) | None => ReviewState::Other,
        };
        let text_of = |key: &str| match item.text_at(&Name::of(key)) {
            Some(found) => found.clone(),
            None => Text::of(""),
        };
        Some(Self {
            commit: text_of("commit_id"),
            login: Name::of(login.as_str()),
            state,
            user_type: match user.text_at(&Name::of("type")) {
                Some(found) => found.clone(),
                None => Text::of(""),
            },
        })
    }

    /// Whether the reviewer is a person rather than a bot.
    fn is_human(&self) -> Answer {
        (self.user_type.as_str() == "User"
            && !self.login.as_str().to_lowercase().ends_with("[bot]"))
        .answer()
    }

    /// The reviewer's login folded to lower case.
    fn lowercase_login(&self) -> Name {
        Name::from_string(self.login.as_str().to_lowercase())
    }
}

/// Whether the logins behind every commit of a pull request are known.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Attribution {
    /// Every commit names a login for its author and its committer, and the
    /// listing holds every commit of the pull request.
    Complete,
    /// A commit has an author or committer without a login (its email is
    /// linked to no account), or the listing is not the whole pull request.
    Incomplete,
}

/// The logins behind the commits of a pull request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Contributors {
    /// Whether `logins` is the whole truth.
    pub attribution: Attribution,
    /// The logins that authored or committed a commit.
    pub logins: Vec<Name>,
}

impl Contributors {
    /// Reads the commits the API lists for a pull request. `expected` is the
    /// number of commits the pull request itself reports. The attribution is
    /// `Incomplete` when a commit has no login for its author or committer,
    /// when `expected` is unknown, or when it differs from the number listed
    /// (the API lists at most 250 commits and stops there without saying so).
    #[must_use]
    pub(crate) fn from_commits(items: &[Json], expected: Option<Count>) -> Self {
        let mut attribution = if expected.is_some_and(|count| count.get() == items.len()) {
            Attribution::Complete
        } else {
            Attribution::Incomplete
        };
        let mut logins = Vec::new();
        for item in items {
            for role in ["author", "committer"] {
                let login = item
                    .get(&Name::of(role))
                    .and_then(|user| user.text_at(&Name::of("login")))
                    .filter(|found| !found.as_str().is_empty());
                match login {
                    Some(found) => logins.push(Name::of(found.as_str())),
                    None => attribution = Attribution::Incomplete,
                }
            }
        }
        Self {
            attribution,
            logins,
        }
    }
}

/// Why a pull request is not approved.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Problem {
    /// A commit has no login for its author or committer, or the commit list
    /// is not the whole pull request.
    CommitsNotAttributable,
    /// A required input job of the policy run did not succeed.
    InputJobsNotAllSuccessful,
    /// No code owner approved a path that needs one.
    NoCodeOwnerApprovalForRequiredPath,
    /// No valid approval exists for the current head commit.
    NoValidApprovalOnCurrentHead,
}

/// Whether every input job of the policy run succeeded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Inputs {
    /// Every input job succeeded.
    AllSucceeded,
    /// At least one input job did not succeed.
    SomeFailed,
}

/// The result of evaluating approval.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// The pull request is approved by these reviewers.
    Approved {
        /// The reviewers who approved.
        approvers: Vec<Name>,
    },
    /// The pull request is not approved, for these reasons.
    NotApproved {
        /// Why the change is not approved.
        problems: Vec<Problem>,
    },
}

/// Everything approval depends on.
pub(crate) struct Evaluation<'case> {
    /// The pull request author, who cannot approve their own change.
    pub author: &'case Name,
    /// The logins that authored or committed a commit of the pull request,
    /// who cannot approve it either, and whether they are all known.
    pub contributors: &'case Contributors,
    /// The head commit the approval must name.
    pub head: &'case Sha,
    /// The state of the input jobs of the policy run.
    pub inputs: Inputs,
    /// The reasons that sent the change to a human.
    pub reasons: &'case [Reason],
    /// Every review of the pull request.
    pub reviews: &'case [Review],
    /// The code owner rules from the base branch.
    pub rules: &'case [Rule],
}

/// The latest review per user that changes state. COMMENTED and PENDING never
/// override an approval; anything else (including a dismissal) does.
fn effective_reviews(reviews: &[Review]) -> BTreeMap<Name, &Review> {
    let mut latest = BTreeMap::new();
    for review in reviews {
        match review.state {
            ReviewState::Commented | ReviewState::Pending => {}
            ReviewState::Approved | ReviewState::Other => {
                match latest.insert(review.lowercase_login(), review) {
                    Some(_) | None => {}
                }
            }
        }
    }
    latest
}

/// The distinct (path, lowercase owners) pairs the reasons require.
fn required_owners(reasons: &[Reason], rules: &[Rule]) -> Vec<(RelPath, BTreeSet<Name>)> {
    let mut seen: Vec<(RelPath, BTreeSet<Name>)> = Vec::new();
    for reason in reasons {
        let path = match &reason.path {
            Some(found) if reason.code.is_path_based() == Answer::Yes => found.clone(),
            Some(_) | None => RelPath::of(POLICY_PROBE),
        };
        let owners: BTreeSet<Name> = owners_for(rules, &path)
            .iter()
            .map(|owner| Name::from_string(owner.as_str().to_lowercase()))
            .collect();
        let pair = (path, owners);
        if !seen.contains(&pair) {
            seen.push(pair);
        }
    }
    seen
}

/// The qualified approvers: a human, not the author and not a contributor of
/// a commit, approving `head`.
fn qualified_approvers(case: &Evaluation<'_>) -> BTreeSet<Name> {
    let author = case.author.as_str().to_lowercase();
    let contributors: BTreeSet<Name> = case
        .contributors
        .logins
        .iter()
        .map(|login| Name::from_string(login.as_str().to_lowercase()))
        .collect();
    effective_reviews(case.reviews)
        .into_iter()
        .filter(|(login, review)| {
            review.state == ReviewState::Approved
                && review.commit.as_str() == case.head.as_str()
                && login.as_str() != author
                && !contributors.contains(login)
                && review.is_human() == Answer::Yes
        })
        .map(|(login, _)| login)
        .collect()
}

/// Evaluates approval of `case`. Only individual users are resolved as code
/// owners; teams are never expanded, so a team owner approves nothing (fail
/// closed).
pub(crate) fn evaluate(case: &Evaluation<'_>) -> Outcome {
    let mut problems = Vec::new();
    match case.inputs {
        Inputs::SomeFailed => problems.push(Problem::InputJobsNotAllSuccessful),
        Inputs::AllSucceeded => {}
    }
    match case.contributors.attribution {
        Attribution::Incomplete => problems.push(Problem::CommitsNotAttributable),
        Attribution::Complete => {}
    }
    let qualified = qualified_approvers(case);
    if qualified.is_empty() {
        problems.push(Problem::NoValidApprovalOnCurrentHead);
    }
    let mut used: BTreeSet<Name> = BTreeSet::new();
    for (_path, owners) in required_owners(case.reasons, case.rules) {
        let covering: Vec<&Name> = qualified
            .iter()
            .filter(|login| owners.contains(&Name::from_string(format!("@{}", login.as_str()))))
            .collect();
        if covering.is_empty() {
            problems.push(Problem::NoCodeOwnerApprovalForRequiredPath);
        }
        used.extend(covering.into_iter().cloned());
    }
    if problems.is_empty() && !case.reasons.is_empty() {
        Outcome::Approved {
            approvers: used.into_iter().collect(),
        }
    } else {
        Outcome::NotApproved { problems }
    }
}
