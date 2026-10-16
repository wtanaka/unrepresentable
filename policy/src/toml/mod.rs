//! TOML reading for manifests and lock files. The `toml` crate parses (TOML
//! 1.1); this module copies the result into a tree whose tables list their
//! entries in source order, each with the line of its key. How a table was
//! written (header, inline or dotted keys) does not matter to the policy, so
//! it is not kept. Floats, dates and integers are not needed and read as
//! [`Value::Other`].

#[cfg(test)]
mod tests;

use ::toml::de::{DeTable, DeValue};

use crate::failure::Failure;
use crate::types::{Answer, Count, Message, Name, Offset, Text};

/// A key, its value, and the line of the key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Item {
    /// The key.
    pub key: Name,
    /// The one-based line on which the key is written.
    pub line: Count,
    /// The value.
    pub value: Value,
}

/// A table: its entries in source order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Table(Vec<Item>);

/// A TOML value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Value {
    /// An array of values.
    Array(Vec<Self>),
    /// A boolean.
    Bool(bool),
    /// An integer, float, date or time.
    Other,
    /// A string.
    Str(Text),
    /// A table, from a header, an inline table or dotted keys.
    Table(Table),
}

/// The offset at which each line of a text starts.
struct LineStarts(Vec<Offset>);

/// Reads one document against its text.
struct Reader<'src> {
    /// The line starts of the text.
    starts: LineStarts,
    /// The text the offsets point into.
    text: &'src Text,
}

impl Table {
    /// A table with no entries.
    #[cfg(test)]
    pub(crate) const fn empty() -> Self {
        Self(Vec::new())
    }

    /// The entry named `key`.
    #[must_use]
    pub(crate) fn get(&self, key: &Name) -> Option<&Item> {
        self.0.iter().find(|item| item.key == *key)
    }

    /// The entries, in source order.
    pub(crate) fn items(&self) -> &[Item] {
        &self.0
    }

    /// The table stored under `key`.
    #[must_use]
    pub(crate) fn table_at(&self, key: &Name) -> Option<&Self> {
        match &self.get(key)?.value {
            Value::Table(found) => Some(found),
            Value::Array(_) | Value::Bool(_) | Value::Other | Value::Str(_) => None,
        }
    }

    /// The string stored under `key`.
    #[must_use]
    pub(crate) fn text_at(&self, key: &Name) -> Option<&Text> {
        match &self.get(key)?.value {
            Value::Str(found) => Some(found),
            Value::Array(_) | Value::Bool(_) | Value::Other | Value::Table(_) => None,
        }
    }
}

impl Value {
    /// Whether this is the boolean `true`.
    pub(crate) const fn is_true(&self) -> Answer {
        match self {
            Self::Bool(true) => Answer::Yes,
            Self::Bool(false) | Self::Array(_) | Self::Other | Self::Str(_) | Self::Table(_) => {
                Answer::No
            }
        }
    }
}

impl Reader<'_> {
    /// The one-based column at `offset`, counted in characters.
    fn col_at(&self, offset: Offset) -> Result<Count, Failure> {
        let first = self.line_at(offset).minus(Count::new(1))?.get();
        let start = self
            .starts
            .0
            .get(first)
            .ok_or_else(|| Failure::internal(&Message::of("line start out of range")))?;
        let before = self
            .text
            .as_str()
            .get(start.get()..offset.get())
            .ok_or_else(|| {
                Failure::internal(&Message::of("TOML span is not a character boundary"))
            })?;
        Count::new(before.chars().count()).next()
    }

    /// The one-based line holding `offset`.
    fn line_at(&self, offset: Offset) -> Count {
        Count::new(self.starts.0.partition_point(|start| *start <= offset))
    }

    /// Copies a table, in source order.
    fn table(&self, table: &DeTable<'_>) -> Result<Table, Failure> {
        let mut keyed: Vec<(Offset, Name, &DeValue<'_>)> = table
            .iter()
            .map(|(key, value)| {
                (
                    Offset::new(key.span().start),
                    Name::of(key.get_ref().as_ref()),
                    value.get_ref(),
                )
            })
            .collect();
        keyed.sort_by_key(|(at, _, _)| *at);
        let mut items = Vec::new();
        for (at, key, value) in keyed {
            items.push(Item {
                key,
                line: self.line_at(at),
                value: self.value(value)?,
            });
        }
        Ok(Table(items))
    }

    /// Copies a value.
    fn value(&self, value: &DeValue<'_>) -> Result<Value, Failure> {
        match value {
            DeValue::Array(items) => items
                .iter()
                .map(|item| self.value(item.get_ref()))
                .collect::<Result<Vec<Value>, Failure>>()
                .map(Value::Array),
            DeValue::Boolean(flag) => Ok(Value::Bool(*flag)),
            DeValue::Datetime(_) | DeValue::Float(_) | DeValue::Integer(_) => Ok(Value::Other),
            DeValue::String(text) => Ok(Value::Str(Text::of(text.as_ref()))),
            DeValue::Table(inner) => Ok(Value::Table(self.table(inner)?)),
        }
    }
}

/// Parses a document; a failure names the line and column.
pub(crate) fn parse(text: &Text) -> Result<Table, Failure> {
    let mut starts = vec![Offset::new(0)];
    for (index, _) in text.as_str().match_indices('\n') {
        starts.push(Offset::new(Count::new(index).next()?.get()));
    }
    let reader = Reader {
        starts: LineStarts(starts),
        text,
    };
    let parsed = DeTable::parse(text.as_str()).map_err(|error| {
        let what = match error.span() {
            Some(span) => {
                let at = Offset::new(span.start);
                match reader.col_at(at) {
                    Ok(col) => format!(
                        "malformed TOML at line {}, column {}: {}",
                        reader.line_at(at).get(),
                        col.get(),
                        error.message()
                    ),
                    Err(failure) => return failure,
                }
            }
            None => format!("malformed TOML: {}", error.message()),
        };
        Failure::parse(&Message::from_string(what))
    })?;
    reader.table(parsed.get_ref())
}
