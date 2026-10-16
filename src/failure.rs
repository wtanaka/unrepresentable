//! Errors that end a run with exit code 2.

use crate::types::{Col, Line, Message, RelPath};

/// The reason a run ended with exit code 2.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Failure {
    /// A git command failed or produced unusable output.
    Git(Message),
    /// An internal error: the tool itself is wrong.
    Internal(Message),
    /// A file system or process input/output error.
    Io(Message),
    /// A file could not be parsed.
    Parse(Message),
    /// The command line was used incorrectly.
    Usage(Message),
}

impl Failure {
    /// Builds a git failure from `message`.
    pub(crate) fn git(message: &Message) -> Self {
        Self::Git(message.clone())
    }

    /// Builds an internal failure from `message`.
    pub(crate) fn internal(message: &Message) -> Self {
        Self::Internal(message.clone())
    }

    /// Builds an input/output failure from `message`.
    pub(crate) fn io(message: &Message) -> Self {
        Self::Io(message.clone())
    }

    /// A parse failure that names file and line.
    pub(crate) fn parse_at(file: &RelPath, line: Line, message: &Message) -> Self {
        Self::Parse(Message::from_string(format!(
            "{}:{}: {}",
            file.as_str(),
            line.get(),
            message.as_str()
        )))
    }

    /// A parse failure that names file, line and column.
    pub(crate) fn parse_at_col(file: &RelPath, line: Line, col: Col, message: &Message) -> Self {
        Self::Parse(Message::from_string(format!(
            "{}:{}:{}: {}",
            file.as_str(),
            line.get(),
            col.get(),
            message.as_str()
        )))
    }

    /// The message carried by the failure.
    pub(crate) fn text(&self) -> Message {
        match self {
            Self::Usage(message)
            | Self::Io(message)
            | Self::Parse(message)
            | Self::Git(message)
            | Self::Internal(message) => message.clone(),
        }
    }

    /// Builds a usage failure from `message`.
    pub(crate) fn usage(message: &Message) -> Self {
        Self::Usage(message.clone())
    }
}
