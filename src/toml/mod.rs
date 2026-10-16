//! TOML reading: the `toml` crate parses (TOML 1.1), this module turns its
//! result into the section-and-entry shape the rest of the tool reads.
//!
//! The crate returns a merged tree with byte spans and no record of how a
//! table was written. The tool needs the written form: which `[table]` or
//! `[[array]]` header an entry sits under, the line of each header and entry,
//! and dotted keys as written. This module recovers that from the spans: a
//! table whose span starts at `[` was written as a header, `{` as an inline
//! table, and anything else was made by a dotted key or an enclosing header.
//! Floats and dates are kept as their text; the tool never computes with them.
//! So is an integer outside the 64-bit range: it is not an error for the file
//! (a clippy knob may be a `u64`), only for a reader that needs an integer.
//! Any failure names file, line and column.

#[cfg(test)]
mod tests;

use core::iter;

use ::toml::de::{DeArray, DeTable, DeValue};

use crate::failure::Failure;

use crate::types::{
    Answer, Col, Count, Holds as _, Line, Message, Name, Offset, RelPath, Source, Text,
};

/// A parsed TOML value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Value {
    /// An array of values.
    Array(Vec<Self>),
    /// A boolean.
    Bool(bool),
    /// A signed 64-bit integer.
    Int(i64),
    /// A float, date or time, or an integer that does not fit in 64 bits, kept as
    /// its source text: the tool never computes with these, it only needs to read
    /// past them. A reader that needs an integer rejects it, naming its position.
    Raw(Text),
    /// A string, basic or literal.
    Str(Text),
    /// An inline table, as its list of entries.
    Table(Vec<Entry>),
}

/// A `key = value` pair, possibly with a dotted key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Entry {
    /// The key path; more than one segment for dotted keys.
    pub key: KeyPath,
    /// The one-based line on which the entry starts.
    pub line: Line,
    /// The value assigned to the key.
    pub value: Value,
}

/// The key of an entry: at least one segment, more for a dotted key. A TOML key
/// is never empty, so the first segment is a field and an empty path cannot be
/// built; [`KeyPath::from_segments`] refuses an empty list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct KeyPath {
    /// The first segment.
    pub first: Name,
    /// The segments after the first, in order; empty for a single key.
    pub rest: Vec<Name>,
}

/// The syntactic form of a section header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SectionKind {
    /// A `[[array]]` header that starts a new element of an array of tables.
    ArrayOfTables,
    /// The implicit section holding entries before any header.
    Root,
    /// A plain `[table]` header.
    Table,
}

/// A header and the entries that follow it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Section {
    /// The entries that appear under the header, in source order.
    pub entries: Vec<Entry>,
    /// The header key path; empty for the root section.
    pub header: Vec<Name>,
    /// Whether this is the root, a table or an array-of-tables section.
    pub kind: SectionKind,
    /// The one-based line of the header; 1 for the root section.
    pub line: Line,
}

/// A parsed TOML document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Doc {
    /// All sections in source order, starting with the root section.
    pub sections: Vec<Section>,
}

impl Value {
    /// The items when this value is an array, otherwise `None`.
    #[must_use]
    pub(crate) fn as_array(&self) -> Option<&[Self]> {
        match self {
            Self::Array(items) => Some(items),
            Self::Str(_) | Self::Int(_) | Self::Bool(_) | Self::Raw(_) | Self::Table(_) => None,
        }
    }

    /// The entries when this value is an inline table, otherwise `None`.
    #[must_use]
    pub(crate) fn as_table(&self) -> Option<&[Entry]> {
        match self {
            Self::Table(entries) => Some(entries),
            Self::Str(_) | Self::Int(_) | Self::Bool(_) | Self::Raw(_) | Self::Array(_) => None,
        }
    }

    /// The text when this value is a string, otherwise `None`.
    #[must_use]
    pub(crate) const fn as_text(&self) -> Option<&Text> {
        match self {
            Self::Str(text) => Some(text),
            Self::Int(_) | Self::Bool(_) | Self::Raw(_) | Self::Array(_) | Self::Table(_) => None,
        }
    }
}

impl Entry {
    /// The key when it is a single segment.
    #[must_use]
    pub(crate) const fn single_key(&self) -> Option<&Name> {
        self.key.single()
    }
}

impl KeyPath {
    /// The key written with `.` between its segments.
    #[must_use]
    pub(crate) fn dotted(&self) -> Text {
        Text::from_string(
            self.iter()
                .map(Name::as_str)
                .collect::<Vec<&str>>()
                .join("."),
        )
    }

    /// The path `prefix` followed by `last`; just `last` without a prefix.
    #[must_use]
    pub(crate) fn extending(prefix: Option<&Self>, last: &Name) -> Self {
        match prefix {
            Some(path) => {
                let mut rest = path.rest.clone();
                rest.push(last.clone());
                Self {
                    first: path.first.clone(),
                    rest,
                }
            }
            None => Self::of(last.clone()),
        }
    }

    /// The path of the given segments; `None` for an empty list.
    #[must_use]
    pub(crate) fn from_segments(segments: &[Name]) -> Option<Self> {
        segments.split_first().map(|(first, rest)| Self {
            first: first.clone(),
            rest: rest.to_vec(),
        })
    }

    /// The segments in order, the first one first.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &Name> {
        iter::once(&self.first).chain(self.rest.iter())
    }

    /// A one-segment path.
    #[must_use]
    pub(crate) const fn of(first: Name) -> Self {
        Self {
            first,
            rest: Vec::new(),
        }
    }

    /// The two segments of a two-segment path, otherwise `None`.
    #[must_use]
    pub(crate) const fn pair(&self) -> Option<(&Name, &Name)> {
        match self.rest.as_slice() {
            [second] => Some((&self.first, second)),
            _ => None,
        }
    }

    /// The only segment of a one-segment path, otherwise `None`.
    #[must_use]
    pub(crate) const fn single(&self) -> Option<&Name> {
        match self.rest.as_slice() {
            [] => Some(&self.first),
            _ => None,
        }
    }

    /// The path without its first segment; `None` for a one-segment path.
    #[must_use]
    pub(crate) fn without_first(&self) -> Option<Self> {
        Self::from_segments(&self.rest)
    }
}

impl Doc {
    /// Sections whose header equals `header` and whose kind is `kind`.
    pub(crate) fn sections_named<'ctx>(
        &'ctx self,
        header: &'ctx [Name],
        kind: SectionKind,
    ) -> impl Iterator<Item = &'ctx Section> + 'ctx {
        self.sections
            .iter()
            .filter(move |section| section.kind == kind && section.header == header)
    }
}

/// How a table was written in the source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Opener {
    /// As a `{ ... }` inline table.
    Brace,
    /// As a `[table]` or `[[array]]` header.
    Bracket,
    /// Implicitly: by a dotted key or as a parent of a deeper header.
    Key,
}

/// Where one level of the tree sits: its table, its full header path, and the
/// dotted-key prefix of the entries it yields.
struct Here<'tree, 'src> {
    /// The path of `[table]` keys from the root to this table.
    header: &'tree [Name],
    /// The dotted-key segments already consumed inside the current section.
    prefix: Option<&'tree KeyPath>,
    /// The table whose items are being read.
    table: &'tree DeTable<'src>,
}

/// One key and value of a table, with where they were written.
struct Item<'tree, 'src> {
    /// The key.
    key: Name,
    /// The offset of the key.
    key_at: Offset,
    /// The value.
    value: &'tree DeValue<'src>,
    /// The offset at which the value starts.
    value_at: Offset,
}

/// Entries paired with the offset of their key, to restore source order.
type Located = Vec<(Offset, Entry)>;

/// The offset at which each line of a source starts.
struct LineStarts(Vec<Offset>);

/// Reads one parsed document against its source text.
struct Reader<'src> {
    /// The file being read, used when reporting failures.
    file: &'src RelPath,
    /// The line starts of the source.
    starts: LineStarts,
    /// The source text the offsets point into.
    text: &'src Source,
}

impl LineStarts {
    /// The offset of the start of every line of `text`.
    fn of(text: &Source) -> Result<Self, Failure> {
        let mut starts = vec![Offset::new(0)];
        for (index, _) in text.as_str().match_indices('\n') {
            starts.push(Offset::new(Count::new(index).next()?.get()));
        }
        Ok(Self(starts))
    }
}

impl Reader<'_> {
    /// The header segments of `prefix` followed by `key`.
    fn appended(prefix: &[Name], key: &Name) -> Vec<Name> {
        let mut path = prefix.to_vec();
        path.push(key.clone());
        path
    }

    /// Reads the sections of an `[[array]]` into `sections`.
    fn array_sections(
        &self,
        items: &DeArray<'_>,
        path: &[Name],
        sections: &mut Vec<Section>,
    ) -> Result<(), Failure> {
        for item in items {
            if let DeValue::Table(child) = item.get_ref() {
                let body = self.body(child, path, sections)?;
                sections.push(Section {
                    entries: body,
                    header: path.to_vec(),
                    kind: SectionKind::ArrayOfTables,
                    line: self.line_at(Offset::new(item.span().start)),
                });
            }
        }
        Ok(())
    }

    /// The entries of a section body, in source order.
    fn body(
        &self,
        table: &DeTable<'_>,
        header: &[Name],
        sections: &mut Vec<Section>,
    ) -> Result<Vec<Entry>, Failure> {
        let mut found = Vec::new();
        self.collect(
            &Here {
                header,
                prefix: None,
                table,
            },
            &mut found,
            sections,
        )?;
        found.sort_by_key(|(at, _)| *at);
        Ok(found.into_iter().map(|(_, entry)| entry).collect())
    }

    /// The column at `offset` of `line`, counted in characters.
    fn col_at(&self, offset: Offset, line: Line) -> Result<Col, Failure> {
        let first = Count::new(line.get()).prev()?.get();
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
        Col::from_zero_based(Count::new(before.chars().count()))
    }

    /// Reads `table` into entries in `found`, and its header-defined
    /// descendants into `sections`.
    fn collect(
        &self,
        here: &Here<'_, '_>,
        found: &mut Located,
        sections: &mut Vec<Section>,
    ) -> Result<(), Failure> {
        for item in Self::items(here.table) {
            let path = Self::appended(here.header, &item.key);
            let dotted = KeyPath::extending(here.prefix, &item.key);
            match item.value {
                DeValue::Table(child) => match self.opener(item.value_at) {
                    Opener::Bracket => {
                        let body = self.body(child, &path, sections)?;
                        sections.push(Section {
                            entries: body,
                            header: path,
                            kind: SectionKind::Table,
                            line: self.line_at(item.value_at),
                        });
                    }
                    Opener::Brace => found.push((
                        item.key_at,
                        Entry {
                            key: dotted,
                            line: self.line_at(item.key_at),
                            value: Value::Table(self.inline(child)?),
                        },
                    )),
                    Opener::Key => self.collect(
                        &Here {
                            header: &path,
                            prefix: Some(&dotted),
                            table: child,
                        },
                        found,
                        sections,
                    )?,
                },
                DeValue::Array(items) if self.is_array_of_tables(items) == Answer::Yes => {
                    self.array_sections(items, &path, sections)?;
                }
                DeValue::Array(_)
                | DeValue::Boolean(_)
                | DeValue::Datetime(_)
                | DeValue::Float(_)
                | DeValue::Integer(_)
                | DeValue::String(_) => found.push((
                    item.key_at,
                    Entry {
                        key: dotted,
                        line: self.line_at(item.key_at),
                        value: self.value(item.value)?,
                    },
                )),
            }
        }
        Ok(())
    }

    /// A failure at `offset`, as `file:line:col: message`.
    fn fail_at(&self, offset: Offset, what: &Message) -> Failure {
        let line = self.line_at(offset);
        match self.col_at(offset, line) {
            Ok(col) => Failure::parse_at_col(self.file, line, col, what),
            Err(failure) => failure,
        }
    }

    /// The entries of an inline table, in source order.
    fn inline(&self, table: &DeTable<'_>) -> Result<Vec<Entry>, Failure> {
        let mut unused = Vec::new();
        self.body(table, &[], &mut unused)
    }

    /// Whether `items` is an array written with `[[header]]` lines: every item
    /// is a table whose text starts with `[[`. An array of arrays such as
    /// `[[["x"]]]` also has items that start with `[[` but they are not tables,
    /// and it is an ordinary value.
    fn is_array_of_tables(&self, items: &DeArray<'_>) -> Answer {
        let all_headers = items.iter().all(|item| {
            matches!(item.get_ref(), DeValue::Table(_))
                && self
                    .text
                    .as_str()
                    .get(item.span().start..)
                    .is_some_and(|rest| rest.starts_with("[["))
        });
        (!items.is_empty() && all_headers).answer()
    }

    /// The items of `table` in source order.
    fn items<'tree, 'src>(table: &'tree DeTable<'src>) -> Vec<Item<'tree, 'src>> {
        let mut items: Vec<Item<'tree, 'src>> = table
            .iter()
            .map(|(key, value)| Item {
                key: Name::of(key.get_ref().as_ref()),
                key_at: Offset::new(key.span().start),
                value: value.get_ref(),
                value_at: Offset::new(value.span().start),
            })
            .collect();
        items.sort_by_key(|item| item.key_at);
        items
    }

    /// The one-based line holding `offset`.
    fn line_at(&self, offset: Offset) -> Line {
        Line::new(self.starts.0.partition_point(|start| *start <= offset))
    }

    /// How the table whose value starts at `at` was written.
    fn opener(&self, at: Offset) -> Opener {
        match self
            .text
            .as_str()
            .get(at.get()..)
            .and_then(|rest| rest.chars().next())
        {
            Some('[') => Opener::Bracket,
            Some('{') => Opener::Brace,
            Some(_) | None => Opener::Key,
        }
    }

    /// Converts a scalar, array or inline table.
    fn value(&self, value: &DeValue<'_>) -> Result<Value, Failure> {
        match value {
            DeValue::Array(items) => items
                .iter()
                .map(|item| self.value(item.get_ref()))
                .collect::<Result<Vec<Value>, Failure>>()
                .map(Value::Array),
            DeValue::Boolean(flag) => Ok(Value::Bool(*flag)),
            DeValue::Datetime(moment) => Ok(Value::Raw(Text::from_string(moment.to_string()))),
            DeValue::Float(number) => Ok(Value::Raw(Text::of(number.as_str()))),
            DeValue::Integer(number) => {
                Ok(match i64::from_str_radix(number.as_str(), number.radix()) {
                    Ok(small) => Value::Int(small),
                    Err(_) => Value::Raw(Text::of(number.as_str())),
                })
            }
            DeValue::String(text) => Ok(Value::Str(Text::of(text.as_ref()))),
            DeValue::Table(table) => Ok(Value::Table(self.inline(table)?)),
        }
    }
}

/// Parses `source`, reporting failures against `file`.
pub(crate) fn parse(file: &RelPath, source: &Source) -> Result<Doc, Failure> {
    let reader = Reader {
        file,
        starts: LineStarts::of(source)?,
        text: source,
    };
    let parsed = DeTable::parse(source.as_str()).map_err(|error| {
        let what = Message::from_string(format!("malformed TOML: {}", error.message()));
        match error.span() {
            Some(span) => reader.fail_at(Offset::new(span.start), &what),
            None => Failure::parse_at_col(file, Line::new(1), Col::new(1), &what),
        }
    })?;
    let mut sections = Vec::new();
    let entries = reader.body(parsed.get_ref(), &[], &mut sections)?;
    sections.push(Section {
        entries,
        header: Vec::new(),
        kind: SectionKind::Root,
        line: Line::new(1),
    });
    sections.sort_by_key(|section| (section.kind != SectionKind::Root, section.line));
    Ok(Doc { sections })
}
