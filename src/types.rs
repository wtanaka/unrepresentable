//! Domain newtypes. Primitives live only inside these tuple structs.
use crate::failure::Failure;

/// A human-readable message, such as a failure description.
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

/// A repo-relative path with `/` separators.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RelPath(String);

impl RelPath {
    /// Borrows the path as a string slice.
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    /// The last segment.
    pub(crate) fn file_name(&self) -> Name {
        match self.0.rsplit_once('/') {
            Some((_, name)) => Name::of(name),
            None => Name::of(&self.0),
        }
    }

    /// Wraps an owned string as a path.
    pub(crate) const fn from_string(text: String) -> Self {
        Self(text)
    }

    /// Appends `tail` as a child path.
    pub(crate) fn join(&self, tail: &str) -> Self {
        if self.0.is_empty() || self.0 == "." {
            Self(tail.to_owned())
        } else {
            Self(format!("{}/{tail}", self.0.trim_end_matches('/')))
        }
    }

    /// Copies `text` into a path.
    pub(crate) fn of(text: &str) -> Self {
        Self(text.to_owned())
    }

    /// The directory part, or the empty path.
    pub(crate) fn parent(&self) -> Self {
        match self.0.rsplit_once('/') {
            Some((dir, _)) => Self(dir.to_owned()),
            None => Self(String::new()),
        }
    }
}

/// Contents of a file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Source(String);

impl Source {
    /// Borrows the contents as a string slice.
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    /// Wraps an owned string as file contents.
    pub(crate) const fn from_string(text: String) -> Self {
        Self(text)
    }
}

/// A string value read from a configuration file.
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

    /// Copies `text` into a text value.
    pub(crate) fn of(text: &str) -> Self {
        Self(text.to_owned())
    }
}

/// One identifier or path segment.
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

/// A one-based line number in a source file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Line(usize);

impl Line {
    /// Converts a zero-based line index to a one-based line.
    pub(crate) fn from_index(index: Count) -> Result<Self, Failure> {
        Ok(Self(index.next()?.get()))
    }

    /// The line number as a plain integer.
    pub(crate) const fn get(self) -> usize {
        self.0
    }

    /// Wraps a one-based line number.
    pub(crate) const fn new(value: usize) -> Self {
        Self(value)
    }
}

/// A byte offset into a source text.
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

/// A one-based column number in a source line.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Col(usize);

impl Col {
    /// Converts a zero-based column to a one-based column.
    pub(crate) fn from_zero_based(column: Count) -> Result<Self, Failure> {
        Ok(Self(column.next()?.get()))
    }

    /// The column number as a plain integer.
    pub(crate) const fn get(self) -> usize {
        self.0
    }

    /// Wraps a one-based column number.
    pub(crate) const fn new(value: usize) -> Self {
        Self(value)
    }
}

/// A count or an index. All arithmetic is checked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Count(usize);

impl Count {
    /// The count as a plain integer.
    pub(crate) const fn get(self) -> usize {
        self.0
    }

    /// Subtracts `other`, failing if the result would be negative.
    pub(crate) fn minus(self, other: Self) -> Result<Self, Failure> {
        match self.0.checked_sub(other.0) {
            Some(value) => Ok(Self(value)),
            None => Err(overflow()),
        }
    }

    /// Wraps a count or index.
    pub(crate) const fn new(value: usize) -> Self {
        Self(value)
    }

    /// Adds one, failing on overflow.
    pub(crate) fn next(self) -> Result<Self, Failure> {
        match self.0.checked_add(1) {
            Some(value) => Ok(Self(value)),
            None => Err(overflow()),
        }
    }

    /// Subtracts one, failing on underflow.
    pub(crate) fn prev(self) -> Result<Self, Failure> {
        match self.0.checked_sub(1) {
            Some(value) => Ok(Self(value)),
            None => Err(overflow()),
        }
    }
}

/// The answer to a yes/no question about a value, named at the call site. A
/// bool-returning predicate returns this, made with [`Holds::answer`] on a
/// condition, and callers compare `== Answer::Yes`. A single `char` in a
/// signature is a [`Glyph`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Answer {
    /// The condition does not hold.
    No,
    /// The condition holds.
    Yes,
}

/// Turns a condition into an [`Answer`] at the boundary of a predicate.
pub(crate) trait Holds {
    /// Converts the condition into an [`Answer`].
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
    /// The character as a plain `char`.
    pub(crate) const fn get(self) -> char {
        self.0
    }

    /// Wraps a character.
    pub(crate) const fn new(ch: char) -> Self {
        Self(ch)
    }
}

/// Builds the internal failure used when checked arithmetic overflows.
fn overflow() -> Failure {
    Failure::internal(&Message::of("arithmetic overflow"))
}
