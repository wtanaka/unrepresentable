//! Verdict computation and the verdict block appended to merge commits.

#[cfg(test)]
mod tests;

use crate::failure::Failure;
use crate::reason::{Blocker, Code, Commits, Reason};
use crate::types::{Answer, Count, Holds as _, Message, Name, Sha, Text};
use alloc::collections::BTreeSet;
use core::str::Split;

/// The first line of a verdict block.
const BLOCK_HEADER: &str = "unrepresentable-policy-verdict: v1";
/// The longest login name.
const MAX_LOGIN: usize = 39;
/// The pull request number limit.
const PULL_NUMBER_LIMIT: usize = 1_000_000_000;

/// A pull request number: digits only, positive, below a billion.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PullNumber(Count);

impl PullNumber {
    /// The number.
    pub(crate) const fn count(self) -> Count {
        self.0
    }

    /// A pull request number without validation, for tests.
    #[cfg(test)]
    pub(crate) const fn from_count(count: Count) -> Self {
        Self(count)
    }

    /// The pull request number in `text`, when it is one.
    #[must_use]
    pub(crate) fn parse(text: &Text) -> Option<Self> {
        let count = Count::parse(text)?;
        (count.get() > 0 && count.get() < PULL_NUMBER_LIMIT).then_some(Self(count))
    }
}

/// What the policy decided for a change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// Every path merges automatically.
    Auto,
    /// A person has to decide.
    NeedsHuman,
}

impl Verdict {
    /// The verdict as it is written.
    pub(crate) fn name(self) -> Text {
        Text::of(match self {
            Self::Auto => "auto",
            Self::NeedsHuman => "needs-human",
        })
    }

    /// The verdict named `text`, when there is one.
    #[must_use]
    pub(crate) fn parse(text: &Text) -> Option<Self> {
        match text.as_str() {
            "auto" => Some(Self::Auto),
            "needs-human" => Some(Self::NeedsHuman),
            _ => None,
        }
    }
}

/// Whether a human approval can still turn the verdict into a pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Approvable {
    /// A person cannot approve the change.
    No,
    /// A person can approve the change.
    Yes,
}

/// A verdict and whether a person can approve it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Decision {
    /// Whether a person can approve.
    pub approvable: Approvable,
    /// The verdict.
    pub verdict: Verdict,
}

/// The verdict context of a merge: the commits, the policy commit and who
/// approved for which reasons.
#[derive(Clone, Debug)]
pub(crate) struct BlockFacts {
    /// The reviewers who approved.
    pub approvers: Vec<Name>,
    /// The reason codes.
    pub codes: Vec<Code>,
    /// The base and head commits.
    pub commits: Commits,
    /// The commit of the policy that ran.
    pub policy_commit: Sha,
}

/// A merge commit subject and body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MergeMessage {
    /// The body of the commit message.
    pub body: Text,
    /// The subject of the commit message.
    pub subject: Text,
}

/// The fields of a verdict block found in a commit message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ParsedBlock {
    /// The approver.
    pub approver: Text,
    /// The base commit.
    pub base: Sha,
    /// The head commit.
    pub head: Sha,
    /// The commit of the policy.
    pub policy_commit: Sha,
    /// The reason codes.
    pub reasons: Text,
}

/// The lines a verdict block carries after its header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Label {
    /// The approver line.
    Approver,
    /// The base line.
    Base,
    /// The head line.
    Head,
    /// The policy commit line.
    PolicyCommit,
    /// The reasons line.
    Reasons,
}

impl Label {
    /// The text that starts the line.
    fn prefix(self) -> Text {
        Text::of(match self {
            Self::Base => "base: ",
            Self::Head => "head: ",
            Self::PolicyCommit => "policy-commit: ",
            Self::Reasons => "reasons: ",
            Self::Approver => "approver: ",
        })
    }
}

/// The remaining lines of a block, front to back.
struct Lines<'text>(Split<'text, char>);

impl Lines<'_> {
    /// The value of the next line, which must start with the label's prefix.
    #[must_use]
    fn field(&mut self, label: Label) -> Option<Text> {
        let line = self.0.next()?;
        line.strip_prefix(label.prefix().as_str()).map(Text::of)
    }

    /// Reads the commit id on the line for `label`.
    #[must_use]
    fn sha(&mut self, label: Label) -> Option<Sha> {
        Sha::parse(&self.field(label)?)
    }
}

/// `auto` only with no reasons and no blockers (a blocker is a failed or
/// unreadable input job, and cannot be approved away).
pub(crate) const fn compute(reasons: &[Reason], blockers: &[Blocker]) -> Decision {
    if reasons.is_empty() && blockers.is_empty() {
        return Decision {
            approvable: Approvable::Yes,
            verdict: Verdict::Auto,
        };
    }
    Decision {
        approvable: if blockers.is_empty() {
            Approvable::Yes
        } else {
            Approvable::No
        },
        verdict: Verdict::NeedsHuman,
    }
}

/// Whether `login` is a valid login name.
fn is_login(login: &Name) -> Answer {
    let text = login.as_str();
    let mut chars = text.chars();
    (chars
        .next()
        .is_some_and(|first| first.is_ascii_alphanumeric())
        && chars.all(|found| found.is_ascii_alphanumeric() || found == '-')
        && text.len() <= MAX_LOGIN)
        .answer()
}

/// The verdict block appended to a merge commit message.
pub(crate) fn verdict_block(facts: &BlockFacts) -> Result<Text, Failure> {
    if !facts
        .approvers
        .iter()
        .all(|login| is_login(login) == Answer::Yes)
    {
        return Err(Failure::refused(&Message::of("bad approver login")));
    }
    let codes: BTreeSet<Text> = facts.codes.iter().map(|code| code.name()).collect();
    let reasons = codes
        .iter()
        .map(Text::as_str)
        .collect::<Vec<&str>>()
        .join(",");
    let approvers = if facts.approvers.is_empty() {
        "none".to_owned()
    } else {
        facts
            .approvers
            .iter()
            .map(Name::as_str)
            .collect::<Vec<&str>>()
            .join(",")
    };
    Ok(Text::from_string(format!(
        "{BLOCK_HEADER}\nbase: {}\nhead: {}\npolicy-commit: {}\nreasons: {reasons}\napprover: {approvers}\n",
        facts.commits.base.as_str(),
        facts.commits.head.as_str(),
        facts.policy_commit.as_str(),
    )))
}

/// A neutral subject (no PR text) plus the verdict block at the end.
pub(crate) fn merge_message(
    number: PullNumber,
    facts: &BlockFacts,
) -> Result<MergeMessage, Failure> {
    let block = verdict_block(facts)?;
    Ok(MergeMessage {
        body: Text::from_string(format!(
            "Evaluated head {}\n\n{}",
            facts.commits.head.short().as_str(),
            block.as_str()
        )),
        subject: Text::from_string(format!(
            "Squash merge of pull request #{}",
            number.count().get()
        )),
    })
}

/// Reads the block that ends `candidate` (a final newline or two is allowed).
#[must_use]
fn block_at(candidate: &Text) -> Option<ParsedBlock> {
    let mut lines = Lines(candidate.as_str().split('\n'));
    if lines.0.next()? != BLOCK_HEADER {
        return None;
    }
    let base = lines.sha(Label::Base)?;
    let head = lines.sha(Label::Head)?;
    let policy_commit = lines.sha(Label::PolicyCommit)?;
    let reasons = lines.field(Label::Reasons)?;
    let approver = lines.field(Label::Approver)?;
    let trailing: Vec<&str> = lines.0.collect();
    let clean_tail = matches!(trailing.as_slice(), [] | [""] | ["", ""]);
    let reasons_ok = reasons
        .as_str()
        .chars()
        .all(|found| found.is_ascii_uppercase() || found == '_' || found == ',');
    let approver_ok = !approver.as_str().is_empty()
        && approver
            .as_str()
            .chars()
            .all(|found| found.is_ascii_alphanumeric() || found == ',' || found == '-');
    (clean_tail && reasons_ok && approver_ok).then_some(ParsedBlock {
        approver,
        base,
        head,
        policy_commit,
        reasons,
    })
}

/// The verdict block at the end of `message`, if there is one.
#[must_use]
pub(crate) fn parse_block(message: &Text) -> Option<ParsedBlock> {
    message
        .as_str()
        .match_indices(BLOCK_HEADER)
        .find_map(|(start, _)| {
            message
                .as_str()
                .get(start..)
                .and_then(|tail| block_at(&Text::of(tail)))
        })
}
