//! Domain newtypes. Primitives live only inside these tuple structs.

use crate::failure::Failure;

/// The number of digits in a commit id.
const SHA_DIGITS: usize = 40;
/// The number of digits in a shortened commit id.
const SHORT_DIGITS: usize = 12;

/// Human-readable diagnostic text.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Message(String);

impl Message {
    /// Borrows the message as a string slice.
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    /// Wraps an owned string as a message.
    pub(crate) const fn from_string(text: String) -> Self {
        Self(text)
    }

    /// Copies `text` into a message.
    pub(crate) fn of(text: &str) -> Self {
        Self(text.to_owned())
    }
}

/// Free-form text: evidence, details, file contents.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Text(String);

impl Text {
    /// Borrows the text as a string slice.
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    /// Wraps an owned string as text.
    pub(crate) const fn from_string(text: String) -> Self {
        Self(text)
    }

    /// Copies `text` into text.
    pub(crate) fn of(text: &str) -> Self {
        Self(text.to_owned())
    }
}

/// An API credential. It has no `Debug` or `Display`, so it cannot be logged
/// by accident.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Token(String);

impl Token {
    /// Borrows the token as a string slice.
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    /// Makes a token from `text`.
    pub(crate) fn from_text(text: &Text) -> Self {
        Self(text.as_str().to_owned())
    }
}

/// One identifier: a login, a job name, a key.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Name(String);

impl Name {
    /// Borrows the name as a string slice.
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    /// Wraps an owned string as a name.
    pub(crate) const fn from_string(text: String) -> Self {
        Self(text)
    }

    /// Copies `text` into a name.
    pub(crate) fn of(text: &str) -> Self {
        Self(text.to_owned())
    }
}

/// A repository-relative path with `/` separators.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RelPath(String);

impl RelPath {
    /// Borrows the path as a string slice.
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    /// Wraps an owned string as a path.
    pub(crate) const fn from_string(text: String) -> Self {
        Self(text)
    }

    /// Copies `text` into a path.
    pub(crate) fn of(text: &str) -> Self {
        Self(text.to_owned())
    }

    /// The path as text.
    pub(crate) fn to_text(&self) -> Text {
        Text::of(&self.0)
    }
}

/// A lowercase 40-digit hexadecimal commit id.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Sha(String);

impl Sha {
    /// Borrows the commit id as a string slice.
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    /// A commit id made of one repeated digit, for tests.
    #[cfg(test)]
    pub(crate) fn filled(digit: Glyph) -> Self {
        Self(digit.get().to_string().repeat(SHA_DIGITS))
    }

    /// The commit id in `text`, when it is one.
    #[must_use]
    pub(crate) fn parse(text: &Text) -> Option<Self> {
        let found = text.as_str();
        let all_hex = found
            .chars()
            .all(|digit| digit.is_ascii_digit() || matches!(digit, 'a'..='f'));
        (found.len() == SHA_DIGITS && all_hex).then(|| Self(found.to_owned()))
    }

    /// The first twelve digits.
    pub(crate) fn short(&self) -> Text {
        Text::from_string(self.0.chars().take(SHORT_DIGITS).collect())
    }
}

/// A byte offset into a text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Offset(usize);

impl Offset {
    /// The offset as a plain integer.
    pub(crate) const fn get(self) -> usize {
        self.0
    }

    /// Wraps a byte offset.
    pub(crate) const fn new(value: usize) -> Self {
        Self(value)
    }
}

/// A count, an index, a line number or a pull request number. All arithmetic
/// is checked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Count(usize);

impl Count {
    /// The quotient, or a failure when `other` is zero.
    pub(crate) fn div(self, other: Self) -> Result<Self, Failure> {
        match self.0.checked_div(other.0) {
            Some(value) => Ok(Self(value)),
            None => Err(overflow()),
        }
    }

    /// The value.
    pub(crate) const fn get(self) -> usize {
        self.0
    }

    /// The difference, or a failure on underflow.
    pub(crate) fn minus(self, other: Self) -> Result<Self, Failure> {
        match self.0.checked_sub(other.0) {
            Some(value) => Ok(Self(value)),
            None => Err(overflow()),
        }
    }

    /// A count of `value`.
    pub(crate) const fn new(value: usize) -> Self {
        Self(value)
    }

    /// This count plus one, or a failure on overflow.
    pub(crate) fn next(self) -> Result<Self, Failure> {
        self.plus(Self(1))
    }

    /// Parses decimal digits only: no sign, no space.
    #[must_use]
    pub(crate) fn parse(text: &Text) -> Option<Self> {
        let found = text.as_str();
        if found.is_empty() || !found.chars().all(|digit| digit.is_ascii_digit()) {
            return None;
        }
        match found.parse::<usize>() {
            Ok(value) => Some(Self(value)),
            Err(_) => None,
        }
    }

    /// The sum, or a failure on overflow.
    pub(crate) fn plus(self, other: Self) -> Result<Self, Failure> {
        match self.0.checked_add(other.0) {
            Some(value) => Ok(Self(value)),
            None => Err(overflow()),
        }
    }

    /// The count as decimal text.
    pub(crate) fn text(self) -> Text {
        Text::from_string(self.0.to_string())
    }

    /// The product, or a failure on overflow.
    pub(crate) fn times(self, other: Self) -> Result<Self, Failure> {
        match self.0.checked_mul(other.0) {
            Some(value) => Ok(Self(value)),
            None => Err(overflow()),
        }
    }
}

/// The answer to a yes/no question about a value, named at the call site.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Answer {
    /// The condition does not hold.
    No,
    /// The condition holds.
    Yes,
}

/// Turns a condition into an [`Answer`] at the boundary of a predicate.
pub(crate) trait Holds {
    /// The condition as an answer.
    fn answer(self) -> Answer;
}

impl Holds for bool {
    fn answer(self) -> Answer {
        if self {
            Answer::Yes
        } else {
            Answer::No
        }
    }
}

/// One Unicode scalar read from text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Glyph(char);

impl Glyph {
    /// The character.
    pub(crate) const fn get(self) -> char {
        self.0
    }

    /// A glyph of `found`.
    pub(crate) const fn new(found: char) -> Self {
        Self(found)
    }
}

/// The failure for an arithmetic overflow.
fn overflow() -> Failure {
    Failure::internal(&Message::of("arithmetic overflow"))
}
