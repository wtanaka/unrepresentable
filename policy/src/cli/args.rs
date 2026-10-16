//! Command-line arguments: `--flag value` (or `--flag=value`) pairs, which may
//! repeat, and positional words.

use crate::failure::Failure;
use crate::types::{Count, Message, Name, Sha, Text};
use alloc::collections::BTreeMap;
use std::path::PathBuf;

/// The parsed arguments of one command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Arguments {
    /// The values of each flag.
    flags: BTreeMap<Name, Vec<Text>>,
    /// The arguments that are not flags.
    positional: Vec<Text>,
}

impl Arguments {
    /// Every value given for `key`, in order.
    pub(crate) fn all(&self, key: &Name) -> Vec<Text> {
        match self.flags.get(key) {
            Some(values) => values.clone(),
            None => Vec::new(),
        }
    }

    /// The commit given for `key`.
    pub(crate) fn commit(&self, key: &Name) -> Result<Sha, Failure> {
        let text = self.required(key)?;
        Sha::parse(&text).ok_or_else(|| {
            usage(&Text::from_string(format!(
                "--{} is not a 40-digit commit id",
                key.as_str()
            )))
        })
    }

    /// No flags and no positional words yet.
    const fn empty() -> Self {
        Self {
            flags: BTreeMap::new(),
            positional: Vec::new(),
        }
    }

    /// The positional word at `index`, if there is one.
    #[must_use]
    pub(crate) fn maybe_word(&self, index: Count) -> Option<Text> {
        self.positional.get(index.get()).cloned()
    }

    /// The last value given for `key`.
    #[must_use]
    pub(crate) fn optional(&self, key: &Name) -> Option<Text> {
        self.flags.get(key)?.last().cloned()
    }

    /// Splits `args` into flags and positional words. A flag needs a value.
    pub(crate) fn parse(args: &[Text]) -> Result<Self, Failure> {
        let mut parsed = Self::empty();
        let mut words = args.iter();
        while let Some(word) = words.next() {
            let Some(flag) = word.as_str().strip_prefix("--") else {
                parsed.positional.push(word.clone());
                continue;
            };
            let (key, value) = match flag.split_once('=') {
                Some((before, after)) => (Name::of(before), Text::of(after)),
                None => match words.next() {
                    Some(next) => (Name::of(flag), next.clone()),
                    None => {
                        return Err(usage(&Text::from_string(format!("--{flag} needs a value"))))
                    }
                },
            };
            parsed.flags.entry(key).or_default().push(value);
        }
        Ok(parsed)
    }

    /// The path given for `key`.
    pub(crate) fn path(&self, key: &Name) -> Result<PathBuf, Failure> {
        Ok(PathBuf::from(self.required(key)?.as_str()))
    }

    /// The last value given for `key`, or a usage failure.
    pub(crate) fn required(&self, key: &Name) -> Result<Text, Failure> {
        self.optional(key)
            .ok_or_else(|| usage(&Text::from_string(format!("missing --{}", key.as_str()))))
    }

    /// The positional word at `index`.
    pub(crate) fn word(&self, index: Count) -> Result<Text, Failure> {
        self.positional
            .get(index.get())
            .cloned()
            .ok_or_else(|| usage(&Text::of("missing an argument")))
    }

    /// The positional word at `index` as a commit id.
    pub(crate) fn word_commit(&self, index: Count) -> Result<Sha, Failure> {
        let text = self.word(index)?;
        Sha::parse(&text).ok_or_else(|| usage(&Text::of("expected a 40-digit commit id")))
    }

    /// The positional word at `index` as a path.
    pub(crate) fn word_path(&self, index: Count) -> Result<PathBuf, Failure> {
        Ok(PathBuf::from(self.word(index)?.as_str()))
    }
}

/// The failure for a command line that does not parse.
fn usage(why: &Text) -> Failure {
    Failure::usage(&Message::of(why.as_str()))
}
