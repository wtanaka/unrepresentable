//! `Cargo.toml` lint tables and `Cargo.lock` packages, read from the parsed
//! TOML tree. A document that does not parse is a failure, never an empty
//! result: a manifest the policy cannot read must not be waved through.

use crate::failure::Failure;
use crate::reason::{Code, Reason};
use crate::toml;
use crate::types::{Answer, Count, Holds as _, Name, RelPath, Text};
use alloc::collections::{BTreeMap, BTreeSet};
use core::iter;

/// The registry source of packages from crates.io.
const CRATES_IO: &str = "registry+https://github.com/rust-lang/crates.io-index";

/// A lint level; `strength` orders the levels. It does not derive `Ord`: a
/// derived order would follow the variant order, which is alphabetical, and
/// `LINT_LOWERED` compares strengths.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Level {
    /// The lint is allowed.
    Allow,
    /// The lint is denied.
    Deny,
    /// The lint is forbidden.
    Forbid,
    /// The lint warns.
    Warn,
}

impl Level {
    /// Every level, from strictest to loosest.
    const ALL: [Self; 4] = [Self::Forbid, Self::Deny, Self::Warn, Self::Allow];

    /// The level as written in a manifest.
    pub(crate) fn name(self) -> Text {
        Text::of(match self {
            Self::Allow => "allow",
            Self::Warn => "warn",
            Self::Deny => "deny",
            Self::Forbid => "forbid",
        })
    }

    /// The level written as `text`, if it is one of the four.
    #[must_use]
    fn of(text: &Text) -> Option<Self> {
        Self::ALL.into_iter().find(|level| level.name() == *text)
    }

    /// How strongly the level enforces a lint: allow, warn, deny, forbid.
    pub(crate) const fn strength(self) -> Count {
        match self {
            Self::Allow => Count::new(0),
            Self::Warn => Count::new(1),
            Self::Deny => Count::new(2),
            Self::Forbid => Count::new(3),
        }
    }
}

/// A lint's tool (`clippy`, `rust`, or empty for `[lints]`) and name.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct LintKey {
    /// The name of the lint.
    pub name: Name,
    /// The tool the lint belongs to, or empty for rustc lints.
    pub tool: Name,
}

impl LintKey {
    /// How the lint is written on the command line.
    pub(crate) fn label(&self) -> Text {
        match self.tool.as_str() {
            "clippy" => Text::from_string(format!("clippy::{}", self.name.as_str())),
            "" | "rust" => Text::of(self.name.as_str()),
            other => Text::from_string(format!("{other}::{}", self.name.as_str())),
        }
    }
}

/// A lint level and the line of the manifest that sets it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LintEntry {
    /// The level the lint is set to.
    pub level: Level,
    /// The line of the manifest that sets the level.
    pub line: Count,
}

/// Whether a manifest inherits the workspace lint table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Inherit {
    /// The manifest does not inherit the workspace lints.
    No,
    /// The manifest inherits the workspace lints.
    Yes,
}

/// The lint entries of one manifest, in document order.
#[derive(Clone, Debug)]
pub(crate) struct LintTable {
    /// The lint entries by key.
    entries: BTreeMap<LintKey, LintEntry>,
    /// Whether the workspace lint table is inherited.
    inherit: Inherit,
    /// The keys in the order the manifest writes them.
    order: Vec<LintKey>,
}

/// The base and head versions of one file.
#[derive(Clone, Debug)]
pub(crate) struct Revisions {
    /// The manifest at the base commit.
    pub base: Text,
    /// The manifest at the head commit.
    pub head: Text,
}

/// One `[[package]]` of a lock file.
#[derive(Clone, Debug)]
pub(crate) struct Package {
    /// The recorded checksum.
    checksum: Option<Text>,
    /// The package name.
    name: Option<Text>,
    /// The package source.
    source: Option<Text>,
    /// The package version.
    version: Option<Text>,
}

/// Reads the lint tables of `text`: `[lints]`, `[lints.<tool>]`,
/// `[workspace.lints]` and `[workspace.lints.<tool>]`, however they are
/// written. Entries are visited in document order; a lint written twice keeps
/// its first position and its last level.
pub(crate) fn parse_lints(text: &Text) -> Result<LintTable, Failure> {
    let root = toml::parse(text)?;
    let mut table = LintTable {
        entries: BTreeMap::new(),
        inherit: Inherit::No,
        order: Vec::new(),
    };
    let mut found: Vec<(Count, LintKey, Level)> = Vec::new();
    if let Some(lints) = root.table_at(&Name::of("lints")) {
        if lints
            .get(&Name::of("workspace"))
            .is_some_and(|item| item.value.is_true() == Answer::Yes)
        {
            table.inherit = Inherit::Yes;
        }
        collect_lints(lints, &mut found);
    }
    if let Some(lints) = root
        .table_at(&Name::of("workspace"))
        .and_then(|workspace| workspace.table_at(&Name::of("lints")))
    {
        collect_lints(lints, &mut found);
    }
    found.sort_by_key(|(line, _, _)| *line);
    for (line, key, level) in found {
        if !table.entries.contains_key(&key) {
            table.order.push(key.clone());
        }
        match table.entries.insert(key, LintEntry { level, line }) {
            Some(_) | None => {}
        }
    }
    Ok(table)
}

/// Adds the lints of one `lints` table to `out`: direct entries have no tool,
/// and each table of lints is named for its tool.
fn collect_lints(lints: &toml::Table, out: &mut Vec<(Count, LintKey, Level)>) {
    for item in lints.items() {
        if item.key.as_str() == "workspace" {
            continue;
        }
        if let Some(level) = level_of(&item.value) {
            out.push((
                item.line,
                LintKey {
                    name: item.key.clone(),
                    tool: Name::of(""),
                },
                level,
            ));
            continue;
        }
        match &item.value {
            toml::Value::Table(tool_lints) => {
                for lint in tool_lints.items() {
                    if let Some(level) = level_of(&lint.value) {
                        out.push((
                            lint.line,
                            LintKey {
                                name: lint.key.clone(),
                                tool: item.key.clone(),
                            },
                            level,
                        ));
                    }
                }
            }
            toml::Value::Array(_)
            | toml::Value::Bool(_)
            | toml::Value::Other
            | toml::Value::Str(_) => {}
        }
    }
}

/// The level a lint value sets: a bare string, or the `level` of a table.
#[must_use]
fn level_of(value: &toml::Value) -> Option<Level> {
    match value {
        toml::Value::Str(text) => Level::of(text),
        toml::Value::Table(inner) => inner.text_at(&Name::of("level")).and_then(Level::of),
        toml::Value::Array(_) | toml::Value::Bool(_) | toml::Value::Other => None,
    }
}

/// Reasons for every lint the head weakens or drops relative to the base.
pub(crate) fn lint_reasons(path: &RelPath, texts: &Revisions) -> Result<Vec<Reason>, Failure> {
    let base = parse_lints(&texts.base)?;
    let head = parse_lints(&texts.head)?;
    let mut out = Vec::new();
    if base.inherit == Inherit::Yes && head.inherit == Inherit::No {
        out.push(Reason::at(
            Code::LintLowered,
            &Text::from_string(format!(
                "workspace lint inheritance removed ({})",
                path.as_str()
            )),
            path,
            Count::new(1),
        ));
    }
    for key in &base.order {
        let Some(old) = base.entries.get(key) else {
            continue;
        };
        let label = key.label();
        match head.entries.get(key) {
            Some(new) if new.level.strength() >= old.level.strength() => {}
            Some(new) => out.push(Reason::at(
                Code::LintLowered,
                &Text::from_string(format!(
                    "{} {}\u{2192}{} ({}:{})",
                    label.as_str(),
                    old.level.name().as_str(),
                    new.level.name().as_str(),
                    path.as_str(),
                    new.line.get()
                )),
                path,
                new.line,
            )),
            None => out.push(Reason::at(
                Code::LintLowered,
                &Text::from_string(format!(
                    "{} {}\u{2192}(removed) ({}, was line {})",
                    label.as_str(),
                    old.level.name().as_str(),
                    path.as_str(),
                    old.line.get()
                )),
                path,
                Count::new(1),
            )),
        }
    }
    Ok(out)
}

/// Reads the packages of a lock file.
pub(crate) fn parse_lock(text: &Text) -> Result<Vec<Package>, Failure> {
    let root = toml::parse(text)?;
    let entries: &[toml::Value] = match root.get(&Name::of("package")).map(|item| &item.value) {
        Some(toml::Value::Array(items)) => items,
        Some(
            toml::Value::Bool(_) | toml::Value::Other | toml::Value::Str(_) | toml::Value::Table(_),
        )
        | None => &[],
    };
    Ok(entries
        .iter()
        .filter_map(|entry| match entry {
            toml::Value::Table(fields) => Some(Package {
                checksum: fields.text_at(&Name::of("checksum")).cloned(),
                name: fields.text_at(&Name::of("name")).cloned(),
                source: fields.text_at(&Name::of("source")).cloned(),
                version: fields.text_at(&Name::of("version")).cloned(),
            }),
            toml::Value::Array(_)
            | toml::Value::Bool(_)
            | toml::Value::Other
            | toml::Value::Str(_) => None,
        })
        .collect())
}

/// Groups the versions of each package name in a lock file.
fn versions_by_name(text: &Text) -> Result<BTreeMap<Text, BTreeSet<Text>>, Failure> {
    let mut found: BTreeMap<Text, BTreeSet<Text>> = BTreeMap::new();
    for package in parse_lock(text)? {
        let Some(name) = package.name else {
            continue;
        };
        let version = match package.version {
            Some(known) => known,
            None => Text::of("?"),
        };
        found.entry(name).or_default().extend(iter::once(version));
    }
    Ok(found)
}

/// Reasons for every package whose versions differ between base and head.
pub(crate) fn lock_reasons(path: &RelPath, texts: &Revisions) -> Result<Vec<Reason>, Failure> {
    let base = versions_by_name(&texts.base)?;
    let head = versions_by_name(&texts.head)?;
    let names: BTreeSet<&Text> = base.keys().chain(head.keys()).collect();
    let empty = BTreeSet::new();
    let mut out = Vec::new();
    for name in names {
        let before = match base.get(name) {
            Some(found) => found,
            None => &empty,
        };
        let after = match head.get(name) {
            Some(found) => found,
            None => &empty,
        };
        let gone: Vec<&Text> = before.difference(after).collect();
        let added: Vec<&Text> = after.difference(before).collect();
        let evidence = match (gone.as_slice(), added.as_slice()) {
            ([], []) => continue,
            ([old], [new]) => format!(
                "{} {} {}\u{2192}{}",
                path.as_str(),
                name.as_str(),
                old.as_str(),
                new.as_str()
            ),
            _ => {
                let removed = gone
                    .iter()
                    .map(|version| format!(" (removed {})", version.as_str()))
                    .collect::<Vec<String>>()
                    .concat();
                let appended = added
                    .iter()
                    .map(|version| format!(" (added {})", version.as_str()))
                    .collect::<Vec<String>>()
                    .concat();
                format!("{} {}{removed}{appended}", path.as_str(), name.as_str())
            }
        };
        out.push(Reason::at(
            Code::DependencyChanged,
            &Text::from_string(evidence),
            path,
            Count::new(1),
        ));
    }
    Ok(out)
}

/// True when every external package in a lock file comes from crates.io with
/// a checksum, so fetching it cannot reach arbitrary hosts.
pub(crate) fn lock_is_registry_only(text: &Text) -> Result<Answer, Failure> {
    Ok(parse_lock(text)?
        .iter()
        .all(|package| match &package.source {
            Some(source) => source.as_str() == CRATES_IO && package.checksum.is_some(),
            None => true,
        })
        .answer())
}
