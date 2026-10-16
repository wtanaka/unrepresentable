//! Errors that end a run with exit code 2.

use crate::types::Message;

/// Why a command could not decide.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Failure {
    /// The GitHub API refused a call or answered badly.
    Api(Message),
    /// The program has a bug.
    Internal(Message),
    /// A file or stream could not be read or written.
    Io(Message),
    /// Input could not be parsed.
    Parse(Message),
    /// A child process failed.
    Process(Message),
    /// The command refused to act.
    Refused(Message),
    /// The command line is wrong.
    Usage(Message),
}

impl Failure {
    /// A GitHub API failure.
    pub(crate) fn api(message: &Message) -> Self {
        Self::Api(message.clone())
    }

    /// An internal failure.
    pub(crate) fn internal(message: &Message) -> Self {
        Self::Internal(message.clone())
    }

    /// An input or output failure.
    pub(crate) fn io(message: &Message) -> Self {
        Self::Io(message.clone())
    }

    /// A parse failure.
    pub(crate) fn parse(message: &Message) -> Self {
        Self::Parse(message.clone())
    }

    /// A child process failure.
    pub(crate) fn process(message: &Message) -> Self {
        Self::Process(message.clone())
    }

    /// A refusal.
    pub(crate) fn refused(message: &Message) -> Self {
        Self::Refused(message.clone())
    }

    /// The message, with the kind of failure in front.
    pub(crate) fn text(&self) -> Message {
        match self {
            Self::Usage(message)
            | Self::Io(message)
            | Self::Parse(message)
            | Self::Process(message)
            | Self::Api(message)
            | Self::Refused(message)
            | Self::Internal(message) => message.clone(),
        }
    }

    /// A usage failure.
    pub(crate) fn usage(message: &Message) -> Self {
        Self::Usage(message.clone())
    }
}
