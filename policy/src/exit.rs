//! How a command ends: a status and the lines it prints.

use crate::types::Text;

/// Exit status of a command that ran to a decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExitStatus {
    /// The command found nothing to stop on.
    Clean,
    /// The command found something that stops the merge.
    Failed,
}

/// A command's status and the lines it prints (on standard output).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Report {
    /// The lines to print.
    pub lines: Vec<Text>,
    /// How the command ended.
    pub status: ExitStatus,
}

impl Report {
    /// A report with no lines and a clean status.
    pub(crate) const fn clean() -> Self {
        Self {
            lines: Vec::new(),
            status: ExitStatus::Clean,
        }
    }

    /// A failed report with `lines`.
    pub(crate) const fn failed(lines: Vec<Text>) -> Self {
        Self {
            lines,
            status: ExitStatus::Failed,
        }
    }

    /// The same report with a failed status.
    #[must_use]
    pub(crate) const fn into_failed(mut self) -> Self {
        self.status = ExitStatus::Failed;
        self
    }

    /// One printed line, with a clean status.
    pub(crate) fn saying(line: &Text) -> Self {
        Self {
            lines: vec![line.clone()],
            status: ExitStatus::Clean,
        }
    }
}
