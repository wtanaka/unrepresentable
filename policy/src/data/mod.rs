//! The verdict data stored in the App-authored check run (and handed to the
//! merge step): everything re-evaluation needs, in one JSON object.

#[cfg(test)]
mod tests;

use crate::failure::Failure;
use crate::json::{Flag, Json};
use crate::reason::{Blocker, BlockerCode, Code, Commits, Counts, Job, Reason};
use crate::report::Approval;
use crate::types::{Count, Message, Name, RelPath, Sha, Text};
use crate::verdict::{Approvable, PullNumber, Verdict};

/// The result each input job reported.
pub(crate) type InputResults = Vec<(Job, Text)>;

/// What re-evaluating approval recorded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Reviewed {
    /// The approval state.
    pub approval: Approval,
    /// The reviewers who approved.
    pub approvers: Vec<Name>,
}

/// The decision of one policy run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct VerdictData {
    /// Whether the change may be approved by a person.
    pub approvable: Approvable,
    /// The attempt number of the run.
    pub attempt: Count,
    /// The blockers that stop an approval.
    pub blockers: Vec<Blocker>,
    /// The base and head commits.
    pub commits: Commits,
    /// The counts of files by bucket.
    pub counts: Counts,
    /// The results of the input jobs.
    pub inputs: InputResults,
    /// The commit of the policy that produced the verdict.
    pub policy_commit: Sha,
    /// The pull request number.
    pub pull: PullNumber,
    /// The reasons the change needs a human.
    pub reasons: Vec<Reason>,
    /// The review state, when one was evaluated.
    pub reviewed: Option<Reviewed>,
    /// The workflow run identifier.
    pub run_id: Count,
    /// The verdict itself.
    pub verdict: Verdict,
}

/// The members of the stored object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Key {
    /// The `approvable` key.
    Approvable,
    /// The `approved` key.
    Approved,
    /// The `approvers` key.
    Approvers,
    /// The `attempt` key.
    Attempt,
    /// The `base` key.
    Base,
    /// The `blockers` key.
    Blockers,
    /// The `counts` key.
    Counts,
    /// The `head` key.
    Head,
    /// The `inputs` key.
    Inputs,
    /// The `policy_commit` key.
    PolicyCommit,
    /// The `pull` key.
    Pull,
    /// The `reasons` key.
    Reasons,
    /// The `run_id` key.
    RunId,
    /// The `verdict` key.
    Verdict,
    /// The `version` key.
    Version,
}

impl Key {
    /// The failure for a damaged value under this key.
    fn malformed(self) -> Failure {
        Failure::parse(&Message::from_string(format!(
            "verdict data: bad or missing {}",
            self.name().as_str()
        )))
    }

    /// The key as it is written in the data.
    fn name(self) -> Name {
        Name::of(match self {
            Self::Version => "v",
            Self::Pull => "pr",
            Self::Head => "head",
            Self::Base => "base",
            Self::PolicyCommit => "policy_commit",
            Self::RunId => "run_id",
            Self::Attempt => "attempt",
            Self::Verdict => "verdict",
            Self::Approvable => "approvable",
            Self::Inputs => "inputs",
            Self::Reasons => "reasons",
            Self::Blockers => "blockers",
            Self::Counts => "counts",
            Self::Approved => "approved",
            Self::Approvers => "approvers",
        })
    }
}

/// A member with its stored key, for building objects.
type Entry = (Key, Json);

impl VerdictData {
    /// The entries about the decision.
    fn decision_entries(&self) -> Vec<Entry> {
        let inputs = self
            .inputs
            .iter()
            .map(|(job, result)| (Name::of(job.name().as_str()), Json::text_of(result)))
            .collect();
        let counts = self
            .counts
            .entries()
            .into_iter()
            .map(|(key, value)| (key, Json::count_of(value)))
            .collect();
        vec![
            (Key::Verdict, Json::text_of(&self.verdict.name())),
            (
                Key::Approvable,
                Json::flag_of(match self.approvable {
                    Approvable::Yes => Flag::On,
                    Approvable::No => Flag::Off,
                }),
            ),
            (Key::Inputs, Json::object_of(inputs)),
            (
                Key::Reasons,
                Json::array_of(self.reasons.iter().map(reason_json).collect()),
            ),
            (
                Key::Blockers,
                Json::array_of(self.blockers.iter().map(blocker_json).collect()),
            ),
            (Key::Counts, Json::object_of(counts)),
        ]
    }

    /// Reads data written by `to_json`, strictly: damaged data is an error, which
    /// callers read as "no verdict data for this head".
    pub(crate) fn from_json(value: &Json) -> Result<Self, Failure> {
        let mut reasons = Vec::new();
        for item in list_member(value, Key::Reasons)? {
            reasons.push(reason_from(item)?);
        }
        let mut blockers = Vec::new();
        for item in list_member(value, Key::Blockers)? {
            blockers.push(blocker_from(item)?);
        }
        Ok(Self {
            approvable: match flag_member(value, Key::Approvable)? {
                Flag::On => Approvable::Yes,
                Flag::Off => Approvable::No,
            },
            attempt: count_member(value, Key::Attempt)?,
            blockers,
            commits: Commits {
                base: sha_member(value, Key::Base)?,
                head: sha_member(value, Key::Head)?,
            },
            counts: counts_from(value)?,
            inputs: inputs_from(value)?,
            policy_commit: sha_member(value, Key::PolicyCommit)?,
            pull: PullNumber::parse(&count_member(value, Key::Pull)?.text())
                .ok_or_else(|| Key::Pull.malformed())?,
            reasons,
            reviewed: reviewed_from(value)?,
            run_id: count_member(value, Key::RunId)?,
            verdict: Verdict::parse(&text_member(value, Key::Verdict)?)
                .ok_or_else(|| Key::Verdict.malformed())?,
        })
    }

    /// The entries that identify the run.
    fn identity_entries(&self) -> Vec<Entry> {
        vec![
            (Key::Version, Json::count_of(Count::new(1))),
            (Key::Pull, Json::count_of(self.pull.count())),
            (
                Key::Head,
                Json::text_of(&Text::of(self.commits.head.as_str())),
            ),
            (
                Key::Base,
                Json::text_of(&Text::of(self.commits.base.as_str())),
            ),
            (
                Key::PolicyCommit,
                Json::text_of(&Text::of(self.policy_commit.as_str())),
            ),
            (Key::RunId, Json::count_of(self.run_id)),
            (Key::Attempt, Json::count_of(self.attempt)),
        ]
    }

    /// The entries about the review.
    fn review_entries(&self) -> Vec<Entry> {
        let Some(reviewed) = &self.reviewed else {
            return Vec::new();
        };
        vec![
            (
                Key::Approved,
                Json::flag_of(match reviewed.approval {
                    Approval::Approved => Flag::On,
                    Approval::NotApproved => Flag::Off,
                }),
            ),
            (
                Key::Approvers,
                Json::array_of(
                    reviewed
                        .approvers
                        .iter()
                        .map(|approver| Json::text_of(&Text::of(approver.as_str())))
                        .collect(),
                ),
            ),
        ]
    }

    /// The JSON object stored in the check run.
    pub(crate) fn to_json(&self) -> Json {
        let mut entries = self.identity_entries();
        entries.extend(self.decision_entries());
        entries.extend(self.review_entries());
        object(entries)
    }
}

/// Builds a JSON object from entries.
fn object(entries: Vec<Entry>) -> Json {
    Json::object_of(
        entries
            .into_iter()
            .map(|(key, item)| (key.name(), item))
            .collect(),
    )
}

/// The member `key` of an object.
fn member(value: &Json, key: Key) -> Result<&Json, Failure> {
    value.get(&key.name()).ok_or_else(|| key.malformed())
}

/// The text member `key` of an object.
fn text_member(value: &Json, key: Key) -> Result<Text, Failure> {
    member(value, key)?
        .as_text()
        .cloned()
        .ok_or_else(|| key.malformed())
}

/// The count member `key` of an object.
fn count_member(value: &Json, key: Key) -> Result<Count, Failure> {
    member(value, key)?
        .as_count()
        .ok_or_else(|| key.malformed())
}

/// The commit member `key` of an object.
fn sha_member(value: &Json, key: Key) -> Result<Sha, Failure> {
    Sha::parse(&text_member(value, key)?).ok_or_else(|| key.malformed())
}

/// The list member `key` of an object.
fn list_member(value: &Json, key: Key) -> Result<&[Json], Failure> {
    member(value, key)?
        .as_array()
        .ok_or_else(|| key.malformed())
}

/// The flag member `key` of an object.
fn flag_member(value: &Json, key: Key) -> Result<Flag, Failure> {
    member(value, key)?.as_flag().ok_or_else(|| key.malformed())
}

/// The JSON form of a reason.
fn reason_json(reason: &Reason) -> Json {
    Json::array_of(vec![
        Json::text_of(&reason.code.name()),
        Json::text_of(&reason.evidence),
        match &reason.path {
            Some(path) => Json::text_of(&path.to_text()),
            None => Json::Null,
        },
        Json::count_of(reason.line),
    ])
}

/// Reads a reason from JSON.
fn reason_from(item: &Json) -> Result<Reason, Failure> {
    let bad = || Key::Reasons.malformed();
    let [code, evidence, path, line] = item.as_array().ok_or_else(bad)? else {
        return Err(bad());
    };
    Ok(Reason {
        code: Code::parse(code.as_text().ok_or_else(bad)?).ok_or_else(bad)?,
        evidence: evidence.as_text().cloned().ok_or_else(bad)?,
        line: line.as_count().ok_or_else(bad)?,
        path: path.as_text().map(|found| RelPath::of(found.as_str())),
    })
}

/// The JSON form of a blocker.
fn blocker_json(blocker: &Blocker) -> Json {
    Json::array_of(vec![
        Json::text_of(&blocker.code.name()),
        Json::text_of(&blocker.detail),
    ])
}

/// Reads a blocker from JSON.
fn blocker_from(item: &Json) -> Result<Blocker, Failure> {
    let bad = || Key::Blockers.malformed();
    let [code, detail] = item.as_array().ok_or_else(bad)? else {
        return Err(bad());
    };
    Ok(Blocker {
        code: BlockerCode::parse(code.as_text().ok_or_else(bad)?).ok_or_else(bad)?,
        detail: detail.as_text().cloned().ok_or_else(bad)?,
    })
}

/// Reads the counts from JSON.
fn counts_from(value: &Json) -> Result<Counts, Failure> {
    let entries = member(value, Key::Counts)?
        .entries()
        .ok_or_else(|| Key::Counts.malformed())?;
    let mut counts = Counts::empty();
    for (key, item) in entries {
        counts = counts.with(key, item.as_count().ok_or_else(|| Key::Counts.malformed())?);
    }
    Ok(counts)
}

/// Reads the input job results from JSON.
fn inputs_from(value: &Json) -> Result<InputResults, Failure> {
    let entries = member(value, Key::Inputs)?
        .entries()
        .ok_or_else(|| Key::Inputs.malformed())?;
    let mut inputs = Vec::new();
    for (key, item) in entries {
        if let Some(job) = Job::parse(&Text::of(key.as_str())) {
            let result = item.as_text().ok_or_else(|| Key::Inputs.malformed())?;
            inputs.push((job, result.clone()));
        }
    }
    Ok(inputs)
}

/// Reads the review state from JSON.
fn reviewed_from(value: &Json) -> Result<Option<Reviewed>, Failure> {
    if value.get(&Key::Approved.name()).is_none() {
        return Ok(None);
    }
    let approval = match flag_member(value, Key::Approved)? {
        Flag::On => Approval::Approved,
        Flag::Off => Approval::NotApproved,
    };
    let mut approvers = Vec::new();
    for item in list_member(value, Key::Approvers)? {
        let login = item.as_text().ok_or_else(|| Key::Approvers.malformed())?;
        approvers.push(Name::of(login.as_str()));
    }
    Ok(Some(Reviewed {
        approval,
        approvers,
    }))
}
