use ::serde_json::Value;

use crate::types::{Count, Message, Name, Source, Text};

/// A parsed JSON value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Json {
    /// An array of values.
    Array(Vec<Self>),
    /// A boolean.
    Bool(bool),
    /// The `null` literal.
    Null,
    /// A number, kept as text.
    Number(Text),
    /// An object, as key and value pairs sorted by key.
    Object(Vec<(Name, Self)>),
    /// A string, with escapes decoded.
    Str(Text),
}

/// A two-state switch read from a JSON boolean.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Flag {
    /// The boolean was `false`.
    Off,
    /// The boolean was `true`.
    On,
}

impl Json {
    /// The elements, if this is an array.
    #[must_use]
    pub(crate) fn as_array(&self) -> Option<&[Self]> {
        match self {
            Self::Array(items) => Some(items),
            Self::Null | Self::Bool(_) | Self::Number(_) | Self::Str(_) | Self::Object(_) => None,
        }
    }

    /// The value as a non-negative integer, if it is a number that parses as one.
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

    /// The value as a flag, if it is a boolean.
    #[must_use]
    pub(crate) const fn as_flag(&self) -> Option<Flag> {
        match self {
            Self::Bool(true) => Some(Flag::On),
            Self::Bool(false) => Some(Flag::Off),
            Self::Null | Self::Number(_) | Self::Str(_) | Self::Array(_) | Self::Object(_) => None,
        }
    }

    /// The text, if this is a string.
    #[must_use]
    pub(crate) const fn as_text(&self) -> Option<&Text> {
        match self {
            Self::Str(text) => Some(text),
            Self::Null | Self::Bool(_) | Self::Number(_) | Self::Array(_) | Self::Object(_) => None,
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

    /// The value stored under `key`, if this is an object that has it.
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
}

/// Parses one JSON value; trailing content is an error.
pub(crate) fn parse(source: &Source) -> Result<Json, Message> {
    ::serde_json::from_str::<Value>(source.as_str())
        .map(Json::from_value)
        .map_err(|error| Message::from_string(error.to_string()))
}
