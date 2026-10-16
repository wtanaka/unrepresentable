//! Raw bytes, and base64 over them (the `base64` crate, standard alphabet).

#[cfg(test)]
mod tests;

use ::base64::engine::general_purpose::STANDARD;
use ::base64::Engine as _;

use crate::failure::Failure;
use crate::types::{Answer, Count, Message, Text};

/// Raw bytes read from a file or a process.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Bytes(Vec<u8>);

impl Bytes {
    /// Borrows the bytes as a slice.
    pub(crate) fn as_slice(&self) -> &[u8] {
        &self.0
    }

    /// The sum of all byte values, for tests that build archives.
    #[cfg(test)]
    pub(crate) fn byte_sum(&self) -> Count {
        Count::new(self.0.iter().map(|byte| usize::from(*byte)).sum())
    }

    /// No bytes.
    #[cfg(test)]
    pub(crate) const fn empty() -> Self {
        Self(Vec::new())
    }

    /// Wraps an owned vector of bytes.
    pub(crate) const fn from_vec(items: Vec<u8>) -> Self {
        Self(items)
    }

    /// Whether there are no bytes.
    pub(crate) const fn is_empty(&self) -> Answer {
        if self.0.is_empty() {
            Answer::Yes
        } else {
            Answer::No
        }
    }

    /// The parts one after another, for tests that build archives.
    #[cfg(test)]
    pub(crate) fn join(parts: &[Self]) -> Self {
        Self(parts.iter().flat_map(|part| part.0.clone()).collect())
    }

    /// The number of bytes.
    pub(crate) const fn len(&self) -> Count {
        Count::new(self.0.len())
    }

    /// The bytes as text; invalid UTF-8 becomes replacement characters.
    pub(crate) fn lossy_text(&self) -> Text {
        Text::from_string(String::from_utf8_lossy(&self.0).into_owned())
    }

    /// Copies `items` into a byte string.
    pub(crate) fn of(items: &[u8]) -> Self {
        Self(items.to_vec())
    }

    /// The text of `text` as bytes.
    pub(crate) fn of_text(text: &Text) -> Self {
        Self(text.as_str().as_bytes().to_vec())
    }

    /// The bytes cut or zero-padded to `width`, for tests that build archives.
    #[cfg(test)]
    pub(crate) fn padded_to(&self, width: Count) -> Self {
        let mut items = self.0.clone();
        items.resize(width.get(), 0);
        Self(items)
    }

    /// The pieces between NUL bytes.
    pub(crate) fn split_nul(&self) -> Vec<Self> {
        self.0
            .split(|byte| *byte == 0)
            .map(|piece| Self(piece.to_vec()))
            .collect()
    }

    /// The bytes as text, or `None` when they are not valid UTF-8.
    #[must_use]
    pub(crate) fn strict_text(&self) -> Option<Text> {
        match String::from_utf8(self.0.clone()) {
            Ok(text) => Some(Text::from_string(text)),
            Err(_) => None,
        }
    }

    /// The bytes without one final NUL, or `None` when they do not end in one.
    #[must_use]
    pub(crate) fn strip_nul_suffix(&self) -> Option<Self> {
        self.0.strip_suffix(&[0]).map(|body| Self(body.to_vec()))
    }
}

/// Standard base64 with padding.
pub(crate) fn encode(data: &Bytes) -> Text {
    Text::from_string(STANDARD.encode(data.as_slice()))
}

/// Strict standard base64: only the alphabet, padding only at the end and
/// only as needed, length a multiple of four, no stray trailing bits.
pub(crate) fn decode(text: &Text) -> Result<Bytes, Failure> {
    STANDARD
        .decode(text.as_str())
        .map(Bytes)
        .map_err(|_failure| Failure::parse(&Message::of("bad base64")))
}
