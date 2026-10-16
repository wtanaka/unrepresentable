//! Reasons a pull request needs a human, the jobs that may report them, and
//! the blockers that cannot be approved away.

use alloc::collections::BTreeMap;

use crate::types::{Answer, Count, Holds as _, Name, RelPath, Sha, Text};

/// The base and head commits one policy run evaluates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Commits {
    /// The base commit.
    pub base: Sha,
    /// The head commit.
    pub head: Sha,
}

/// An input job of the policy run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Job {
    /// The job that classifies the changed paths.
    Classify,
    /// The job that compares the base and head binaries on inputs that are
    /// not secret.
    Differential,
    /// The job that compares the base and head binaries on the hidden corpus.
    DifferentialHidden,
    /// The job that runs the checks on the head.
    HeadChecks,
    /// The job that runs mutation tests.
    Mutants,
    /// The job that runs the tool on itself.
    SelfLint,
}

impl Job {
    /// Every job.
    pub(crate) const ALL: [Self; 6] = [
        Self::Classify,
        Self::SelfLint,
        Self::HeadChecks,
        Self::Differential,
        Self::DifferentialHidden,
        Self::Mutants,
    ];

    /// The job name as the workflow spells it.
    pub(crate) fn name(self) -> Text {
        Text::of(match self {
            Self::Classify => "classify",
            Self::SelfLint => "self-lint",
            Self::HeadChecks => "head-checks",
            Self::Differential => "differential",
            Self::DifferentialHidden => "differential-hidden",
            Self::Mutants => "mutants",
        })
    }

    /// The job named `text`, when there is one.
    #[must_use]
    pub(crate) fn parse(text: &Text) -> Option<Self> {
        Self::ALL.into_iter().find(|job| job.name() == *text)
    }

    /// The environment variable that carries the job result.
    pub(crate) fn result_variable(self) -> Name {
        Name::of(match self {
            Self::Classify => "RESULT_CLASSIFY",
            Self::SelfLint => "RESULT_SELF_LINT",
            Self::HeadChecks => "RESULT_HEAD_CHECKS",
            Self::Differential => "RESULT_DIFFERENTIAL",
            Self::DifferentialHidden => "RESULT_DIFFERENTIAL_HIDDEN",
            Self::Mutants => "RESULT_MUTANTS",
        })
    }
}

/// Why a change needs a human.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Code {
    /// The baseline grew.
    BaselineGrew,
    /// The CODEOWNERS file changed.
    CodeownersChanged,
    /// A configuration file changed.
    ConfigFileChanged,
    /// The configuration allows more than before.
    ConfigLoosened,
    /// A dependency changed.
    DependencyChanged,
    /// The diff could not be parsed.
    DiffUnparsable,
    /// A judge file changed.
    JudgeModified,
    /// A ledger file changed.
    LedgerFileChanged,
    /// A ledger row was added.
    LedgerRowAdded,
    /// A lint level was lowered.
    LintLowered,
    /// The head lost a finding the base reports.
    LostFinding,
    /// The head lost findings the base reports on the hidden corpus; only the
    /// number is known.
    LostFindingHidden,
    /// A manifest changed.
    ManifestChanged,
    /// A mutant survived.
    MutantSurvived,
    /// A path outside every known set changed.
    OtherPathChanged,
    /// The merge-policy tooling changed.
    PolicyChanged,
    /// A protected document changed.
    ProtectedDocChanged,
    /// The tool's own rule-enforcing code, under `src/`, changed.
    RulesCodeChanged,
    /// A symbolic link or submodule changed.
    SymlinkOrSubmodule,
    /// Test code changed.
    TestCodeChanged,
    /// The tool needs an approval for the change.
    ToolApproval,
    /// The toolchain changed.
    ToolchainChanged,
    /// A path has characters that are not safe to print.
    UnsafePathChars,
    /// A workflow changed.
    WorkflowChanged,
    /// A zizmor finding.
    ZizmorFinding,
}

impl Code {
    /// Every code.
    pub(crate) const ALL: [Self; 25] = [
        Self::JudgeModified,
        Self::LintLowered,
        Self::DependencyChanged,
        Self::WorkflowChanged,
        Self::UnsafePathChars,
        Self::ManifestChanged,
        Self::ToolchainChanged,
        Self::ConfigFileChanged,
        Self::LedgerFileChanged,
        Self::PolicyChanged,
        Self::CodeownersChanged,
        Self::ProtectedDocChanged,
        Self::TestCodeChanged,
        Self::RulesCodeChanged,
        Self::SymlinkOrSubmodule,
        Self::OtherPathChanged,
        Self::ZizmorFinding,
        Self::DiffUnparsable,
        Self::ConfigLoosened,
        Self::LedgerRowAdded,
        Self::BaselineGrew,
        Self::ToolApproval,
        Self::LostFinding,
        Self::LostFindingHidden,
        Self::MutantSurvived,
    ];

    /// The only job that may report this code.
    pub(crate) const fn emitter(self) -> Job {
        match self {
            Self::JudgeModified
            | Self::LintLowered
            | Self::DependencyChanged
            | Self::WorkflowChanged
            | Self::UnsafePathChars
            | Self::ManifestChanged
            | Self::ToolchainChanged
            | Self::ConfigFileChanged
            | Self::LedgerFileChanged
            | Self::PolicyChanged
            | Self::CodeownersChanged
            | Self::ProtectedDocChanged
            | Self::TestCodeChanged
            | Self::RulesCodeChanged
            | Self::SymlinkOrSubmodule
            | Self::OtherPathChanged
            | Self::ZizmorFinding
            | Self::DiffUnparsable => Job::Classify,
            Self::ConfigLoosened
            | Self::LedgerRowAdded
            | Self::BaselineGrew
            | Self::ToolApproval => Job::SelfLint,
            Self::LostFinding => Job::Differential,
            Self::LostFindingHidden => Job::DifferentialHidden,
            Self::MutantSurvived => Job::Mutants,
        }
    }

    /// Whether the reason names a real repository path, so that an owner of
    /// that path must approve. Every other reason needs an owner of
    /// `policy/**`.
    pub(crate) const fn is_path_based(self) -> Answer {
        match self {
            Self::JudgeModified
            | Self::LintLowered
            | Self::DependencyChanged
            | Self::WorkflowChanged
            | Self::ManifestChanged
            | Self::ToolchainChanged
            | Self::ConfigFileChanged
            | Self::LedgerFileChanged
            | Self::PolicyChanged
            | Self::CodeownersChanged
            | Self::ProtectedDocChanged
            | Self::TestCodeChanged
            | Self::RulesCodeChanged
            | Self::SymlinkOrSubmodule
            | Self::OtherPathChanged
            | Self::ZizmorFinding => Answer::Yes,
            Self::UnsafePathChars
            | Self::DiffUnparsable
            | Self::ConfigLoosened
            | Self::LedgerRowAdded
            | Self::BaselineGrew
            | Self::ToolApproval
            | Self::LostFinding
            | Self::LostFindingHidden
            | Self::MutantSurvived => Answer::No,
        }
    }

    /// Codes shown with a warning sign instead of a cross.
    pub(crate) const fn is_warning(self) -> Answer {
        match self {
            Self::LostFinding | Self::LostFindingHidden | Self::MutantSurvived => Answer::Yes,
            Self::JudgeModified
            | Self::LintLowered
            | Self::DependencyChanged
            | Self::WorkflowChanged
            | Self::UnsafePathChars
            | Self::ManifestChanged
            | Self::ToolchainChanged
            | Self::ConfigFileChanged
            | Self::LedgerFileChanged
            | Self::PolicyChanged
            | Self::CodeownersChanged
            | Self::ProtectedDocChanged
            | Self::TestCodeChanged
            | Self::RulesCodeChanged
            | Self::SymlinkOrSubmodule
            | Self::OtherPathChanged
            | Self::ZizmorFinding
            | Self::DiffUnparsable
            | Self::ConfigLoosened
            | Self::LedgerRowAdded
            | Self::BaselineGrew
            | Self::ToolApproval => Answer::No,
        }
    }

    /// The code as it is written in a result.
    pub(crate) fn name(self) -> Text {
        Text::of(match self {
            Self::JudgeModified => "JUDGE_MODIFIED",
            Self::LintLowered => "LINT_LOWERED",
            Self::DependencyChanged => "DEPENDENCY_CHANGED",
            Self::WorkflowChanged => "WORKFLOW_CHANGED",
            Self::UnsafePathChars => "UNSAFE_PATH_CHARS",
            Self::ManifestChanged => "MANIFEST_CHANGED",
            Self::ToolchainChanged => "TOOLCHAIN_CHANGED",
            Self::ConfigFileChanged => "CONFIG_FILE_CHANGED",
            Self::LedgerFileChanged => "LEDGER_FILE_CHANGED",
            Self::PolicyChanged => "POLICY_CHANGED",
            Self::CodeownersChanged => "CODEOWNERS_CHANGED",
            Self::ProtectedDocChanged => "PROTECTED_DOC_CHANGED",
            Self::TestCodeChanged => "TEST_CODE_CHANGED",
            Self::RulesCodeChanged => "RULES_CODE_CHANGED",
            Self::SymlinkOrSubmodule => "SYMLINK_OR_SUBMODULE",
            Self::OtherPathChanged => "OTHER_PATH_CHANGED",
            Self::ZizmorFinding => "ZIZMOR_FINDING",
            Self::DiffUnparsable => "DIFF_UNPARSABLE",
            Self::ConfigLoosened => "CONFIG_LOOSENED",
            Self::LedgerRowAdded => "LEDGER_ROW_ADDED",
            Self::BaselineGrew => "BASELINE_GREW",
            Self::ToolApproval => "TOOL_APPROVAL",
            Self::LostFinding => "LOST_FINDING",
            Self::LostFindingHidden => "LOST_FINDING_HIDDEN",
            Self::MutantSurvived => "MUTANT_SURVIVED",
        })
    }

    /// The code named `text`, when there is one.
    #[must_use]
    pub(crate) fn parse(text: &Text) -> Option<Self> {
        Self::ALL.into_iter().find(|code| code.name() == *text)
    }
}

/// One reason a change needs a human: a code, evidence text, and where.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Reason {
    /// The code of the reason.
    pub code: Code,
    /// The text that shows why.
    pub evidence: Text,
    /// The line the reason points at, or zero.
    pub line: Count,
    /// The path the reason points at, when there is one.
    pub path: Option<RelPath>,
}

impl Reason {
    /// A reason located at `path`, line `line`.
    pub(crate) fn at(code: Code, evidence: &Text, path: &RelPath, line: Count) -> Self {
        Self {
            code,
            evidence: evidence.clone(),
            line,
            path: Some(path.clone()),
        }
    }

    /// A reason that names no file.
    pub(crate) fn general(code: Code, evidence: &Text) -> Self {
        Self {
            code,
            evidence: evidence.clone(),
            line: Count::new(0),
            path: None,
        }
    }
}

/// Why a decision could not be made at all; never approvable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum BlockerCode {
    /// A required input job failed.
    InputJobFailed,
    /// A job result could not be parsed.
    ResultUnparsable,
    /// The settings audit found drift.
    SettingsAudit,
}

impl BlockerCode {
    /// The code as it is written in a result.
    pub(crate) fn name(self) -> Text {
        Text::of(match self {
            Self::InputJobFailed => "INPUT_JOB_FAILED",
            Self::ResultUnparsable => "RESULT_UNPARSABLE",
            Self::SettingsAudit => "SETTINGS_AUDIT",
        })
    }

    /// The code named `text`, when there is one.
    #[must_use]
    pub(crate) fn parse(text: &Text) -> Option<Self> {
        [
            Self::InputJobFailed,
            Self::ResultUnparsable,
            Self::SettingsAudit,
        ]
        .into_iter()
        .find(|code| code.name() == *text)
    }
}

/// A reason a decision could not be made, with detail.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Blocker {
    /// The kind of blocker.
    pub code: BlockerCode,
    /// The detail that explains it.
    pub detail: Text,
}

/// Named non-negative numbers reported next to the reasons.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Counts(BTreeMap<Name, Count>);

impl Counts {
    /// Whether a count named `key` is present.
    pub(crate) fn contains(&self, key: &Name) -> Answer {
        self.0.contains_key(key).answer()
    }

    /// No counts.
    pub(crate) const fn empty() -> Self {
        Self(BTreeMap::new())
    }

    /// Every name with its count, in name order.
    pub(crate) fn entries(&self) -> Vec<(Name, Count)> {
        self.0
            .iter()
            .map(|(key, value)| (key.clone(), *value))
            .collect()
    }

    /// The named count, or zero when it is absent.
    pub(crate) fn get(&self, key: &Name) -> Count {
        match self.0.get(key) {
            Some(found) => *found,
            None => Count::new(0),
        }
    }

    /// The counts with `key` set to `value`.
    #[must_use]
    pub(crate) fn with(self, key: &Name, value: Count) -> Self {
        let mut map = self.0;
        match map.insert(key.clone(), value) {
            Some(_) | None => {}
        }
        Self(map)
    }
}
