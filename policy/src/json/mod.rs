//! JSON reading and writing. `serde_json` parses into its untyped value, which
//! is copied into [`Json`]; objects then hold their members sorted by key,
//! because the value type is backed by a sorted map. The writer in this module
//! is compact and sorts keys itself, so that the verdict text a check run
//! carries is stable and does not depend on the library's number formatting.
//!
//! Numbers are normalised by the library, not kept as written: an integer that
//! fits `i64` or `u64` keeps its digits, and any other number becomes an `f64`
//! and is written back in that form (`1.0e2` as `100.0`, `-0` as `-0.0`). The
//! `arbitrary_precision` feature does not give the written text back either
//! (`1.0e2` becomes `1.0e+2`, `-0` becomes `0`), so it is not enabled. Nothing
//! here depends on the bytes of an untrusted number that is parsed and written
//! again: such values are compared or shown after parsing, and what this crate
//! writes itself uses counts and strings.
#[cfg(test)]
mod tests;

use ::serde_json::Value;

use crate::types::{Count, Message, Name, Text};

/// A path of object keys: `keys!["a", "b"]`.
macro_rules! keys {
    ($($key:literal),* $(,)?) => {
        [$($crate::types::Name::of($key)),*]
    };
}

/// Code points below this are control characters.
const CONTROL_LIMIT: u32 = 0x20;

/// A JSON value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Json {
    /// An array.
    Array(Vec<Self>),
    /// A boolean.
    Bool(bool),
    /// The null value.
    Null,
    /// A number, kept as its text.
    Number(Text),
    /// An object, with its members sorted by key.
    Object(Vec<Member>),
    /// A string.
    Str(Text),
}

/// A boolean as a named state instead of a primitive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Flag {
    /// False.
    Off,
    /// True.
    On,
}

impl Json {
    /// An array of `items`.
    pub(crate) const fn array_of(items: Vec<Self>) -> Self {
        Self::Array(items)
    }

    /// The elements, when this is an array.
    #[must_use]
    pub(crate) fn as_array(&self) -> Option<&[Self]> {
        match self {
            Self::Array(items) => Some(items),
            Self::Null | Self::Bool(_) | Self::Number(_) | Self::Str(_) | Self::Object(_) => None,
        }
    }

    /// The number, when this is a non-negative integer.
    #[must_use]
    pub(crate) fn as_count(&self) -> Option<Count> {
        match self {
            Self::Number(text) => match text.as_str().parse::<usize>() {
                Ok(value) => Some(Count::new(value)),
                Err(_) => None,
            },
            Self::Null | Self::Bool(_) | Self::Str(_) | Self::Array(_) | Self::Object(_) => None,
        }
    }

    /// The boolean, when this is one.
    #[must_use]
    pub(crate) const fn as_flag(&self) -> Option<Flag> {
        match self {
            Self::Bool(true) => Some(Flag::On),
            Self::Bool(false) => Some(Flag::Off),
            Self::Null | Self::Number(_) | Self::Str(_) | Self::Array(_) | Self::Object(_) => None,
        }
    }

    /// The text, when this is a string.
    #[must_use]
    pub(crate) const fn as_text(&self) -> Option<&Text> {
        match self {
            Self::Str(text) => Some(text),
            Self::Null | Self::Bool(_) | Self::Number(_) | Self::Array(_) | Self::Object(_) => None,
        }
    }

    /// Compact text with object keys sorted.
    pub(crate) fn compact(&self) -> Text {
        match self {
            Self::Null => Text::of("null"),
            Self::Bool(true) => Text::of("true"),
            Self::Bool(false) => Text::of("false"),
            Self::Number(text) => text.clone(),
            Self::Str(text) => quoted(text),
            Self::Array(items) => {
                let parts: Vec<String> = items
                    .iter()
                    .map(|item| item.compact().as_str().to_owned())
                    .collect();
                Text::from_string(format!("[{}]", parts.join(",")))
            }
            Self::Object(entries) => {
                let mut sorted: Vec<&(Name, Self)> = entries.iter().collect();
                sorted.sort_by(|left, right| left.0.cmp(&right.0));
                let parts: Vec<String> = sorted
                    .into_iter()
                    .map(|(key, value)| {
                        format!(
                            "{}:{}",
                            quoted(&Text::of(key.as_str())).as_str(),
                            value.compact().as_str()
                        )
                    })
                    .collect();
                Text::from_string(format!("{{{}}}", parts.join(",")))
            }
        }
    }

    /// The count in a number member.
    #[must_use]
    pub(crate) fn count_at(&self, key: &Name) -> Option<Count> {
        self.get(key)?.as_count()
    }

    /// A number from `value`.
    pub(crate) fn count_of(value: Count) -> Self {
        Self::Number(value.text())
    }

    /// The value reached by following `path` through nested objects.
    #[must_use]
    pub(crate) fn dig(&self, path: &[Name]) -> Option<&Self> {
        let mut here = self;
        for key in path {
            here = here.get(key)?;
        }
        Some(here)
    }

    /// The members of an object, sorted by key (the parser keeps no document order).
    #[must_use]
    pub(crate) fn entries(&self) -> Option<&[Member]> {
        match self {
            Self::Object(entries) => Some(entries),
            Self::Null | Self::Bool(_) | Self::Number(_) | Self::Str(_) | Self::Array(_) => None,
        }
    }

    /// A boolean from `value`.
    pub(crate) const fn flag_of(value: Flag) -> Self {
        match value {
            Flag::On => Self::Bool(true),
            Flag::Off => Self::Bool(false),
        }
    }

    /// Copies a `serde_json` value.
    fn from_value(value: Value) -> Self {
        match value {
            Value::Null => Self::Null,
            Value::Bool(flag) => Self::Bool(flag),
            Value::Number(number) => Self::Number(Text::from_string(number.to_string())),
            Value::String(text) => Self::Str(Text::from_string(text)),
            Value::Array(items) => Self::Array(items.into_iter().map(Self::from_value).collect()),
            Value::Object(members) => Self::Object(
                members
                    .into_iter()
                    .map(|(key, member)| (Name::from_string(key), Self::from_value(member)))
                    .collect(),
            ),
        }
    }

    /// The member `key`, when this is an object that has it.
    #[must_use]
    pub(crate) fn get(&self, key: &Name) -> Option<&Self> {
        match self {
            Self::Object(entries) => entries
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value),
            Self::Null | Self::Bool(_) | Self::Number(_) | Self::Str(_) | Self::Array(_) => None,
        }
    }

    /// An object from its entries.
    pub(crate) const fn object_of(entries: Vec<Member>) -> Self {
        Self::Object(entries)
    }

    /// The text of a string member.
    #[must_use]
    pub(crate) fn text_at(&self, key: &Name) -> Option<&Text> {
        self.get(key)?.as_text()
    }

    /// A string from `value`.
    pub(crate) fn text_of(value: &Text) -> Self {
        Self::Str(value.clone())
    }
}

/// One `"key": value` pair of an object.
pub(crate) type Member = (Name, Json);

/// Parses one JSON value; trailing content is an error.
pub(crate) fn parse(source: &Text) -> Result<Json, Message> {
    ::serde_json::from_str::<Value>(source.as_str())
        .map(Json::from_value)
        .map_err(|error| Message::from_string(error.to_string()))
}

/// The items of an array value, or none when absent or not an array.
pub(crate) fn items_of(value: Option<&Json>) -> &[Json] {
    match value.and_then(Json::as_array) {
        Some(found) => found,
        None => &[],
    }
}

/// The members of an object value, or none when absent or not an object.
pub(crate) fn members_of(value: Option<&Json>) -> &[Member] {
    match value.and_then(Json::entries) {
        Some(found) => found,
        None => &[],
    }
}

/// `text` as a quoted JSON string.
fn quoted(text: &Text) -> Text {
    let mut out = String::from("\"");
    for found in text.as_str().chars() {
        match found {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other if u32::from(other) < CONTROL_LIMIT => {
                let escaped = format!("\\u{:04x}", u32::from(other));
                out.push_str(&escaped);
            }
            other => out.push(other),
        }
    }
    out.push('"');
    Text::from_string(out)
}
