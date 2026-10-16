//! Workspace members and every cargo target of each member.
use alloc::collections::BTreeMap;

use core::{iter, slice};

use crate::config::Config;

use crate::failure::Failure;

use crate::glob::{Brackets, Glob};

use crate::io::{EntryKind, Root};

use crate::toml::{self, Doc, Entry, Section, SectionKind, Value};

use crate::types::{Answer, Count, Holds as _, Line, Message, Name, RelPath, Text};

use crate::util::is_rust_file;

/// The kind of a cargo build target.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum TargetKind {
    /// A benchmark target.
    Bench,
    /// A binary target.
    Bin,
    /// The package build script.
    Build,
    /// An example target.
    Example,
    /// The library target.
    Lib,
    /// An integration test target.
    Test,
}

/// One cargo target of a workspace member.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Target {
    /// What kind of target this is.
    pub kind: TargetKind,
    /// The name of the package that owns the target.
    pub member: Name,
    /// The repo-relative path of the target's root source file.
    pub root: RelPath,
}

/// A workspace member package.
#[derive(Clone, Debug)]
pub(crate) struct Member {
    /// The member's directory, relative to the repo root.
    pub dir: RelPath,
    /// The parsed member `Cargo.toml`.
    pub manifest: Doc,
    /// The repo-relative path of the member's `Cargo.toml`.
    pub manifest_path: RelPath,
    /// The package name from the member manifest.
    pub name: Name,
}

impl Member {
    /// The name the package's library goes by in code: `[lib] name`, else the
    /// package name with `-` turned into `_`.
    pub(crate) fn lib_name(&self) -> Name {
        let lib = table_of(&self.manifest, &Name::of("lib"));
        match value_in(lib.as_deref(), &Name::of("name")).and_then(Value::as_text) {
            Some(text) => Name::of(text.as_str()),
            None => Name::from_string(self.name.as_str().replace('-', "_")),
        }
    }
}

/// The workspace as described by the root manifest and member manifests.
#[derive(Clone, Debug)]
pub(crate) struct Workspace {
    /// The directories selected by `default-members`.
    pub default_members: Vec<RelPath>,
    /// All members, sorted by directory.
    pub members: Vec<Member>,
    /// The parsed root `Cargo.toml`.
    pub root_manifest: Doc,
}

impl Workspace {
    /// Every member must have a role in the config.
    pub(crate) fn check_roles(&self, config: &Config) -> Result<(), Failure> {
        for member in &self.members {
            if !config.members.iter().any(|entry| entry.name == member.name) {
                return Err(Failure::usage(&Message::from_string(format!(
                    "workspace member `{}` has no role in .unrepresentable/config ([member.{}] role = \"pure\" or \"shell\")",
                    member.name.as_str(),
                    member.name.as_str()
                ))));
            }
        }
        Ok(())
    }

    /// Reads the root manifest and every member manifest under `root`.
    pub(crate) fn load(root: &Root) -> Result<Self, Failure> {
        let manifest_path = RelPath::of("Cargo.toml");
        let root_manifest = toml::parse(&manifest_path, &root.read(&manifest_path)?)?;
        let workspace = table_of(&root_manifest, &Name::of("workspace"));
        let patterns = string_list(workspace.as_deref(), &Name::of("members"));
        let excludes = string_list(workspace.as_deref(), &Name::of("exclude"));
        let default_patterns = string_list(workspace.as_deref(), &Name::of("default-members"));
        let mut dirs: Vec<RelPath> = Vec::new();
        if table_of(&root_manifest, &Name::of("package")).is_some() {
            dirs.push(RelPath::of(""));
        }
        let all_dirs = all_directories(root)?;
        for pattern in &patterns {
            for dir in expand(pattern, &all_dirs, root)? {
                if !dirs.contains(&dir) {
                    dirs.push(dir);
                }
            }
        }
        let mut removed: Vec<RelPath> = Vec::new();
        for pattern in &excludes {
            removed.extend(expand(pattern, &all_dirs, root)?);
        }
        dirs.retain(|dir| !removed.contains(dir));
        dirs.sort();
        let mut members = Vec::new();
        for dir in dirs {
            members.push(member_at(root, dir)?);
        }
        if table_of(&root_manifest, &Name::of("workspace")).is_some() {
            // Cargo makes every path dependency that lives inside the workspace
            // directory (and is not excluded) a member, transitively.
            let mut taken = Adoption {
                members: &mut members,
                removed: &removed,
                root,
            };
            taken.adopt(&RelPath::of(""), &root_manifest)?;
            let mut cursor = Count::new(0);
            while let Some(found) = taken.members.get(cursor.get()).cloned() {
                taken.adopt(&found.dir, &found.manifest)?;
                cursor = cursor.next()?;
            }
            members.sort_by(|left, right| left.dir.cmp(&right.dir));
        }
        let mut default_members = Vec::new();
        for pattern in &default_patterns {
            default_members.extend(expand(pattern, &all_dirs, root)?);
        }
        Ok(Self {
            default_members,
            members,
            root_manifest,
        })
    }
}

/// The targets found so far, without duplicates.
struct Found<'found> {
    /// The package that owns the targets.
    member: &'found Name,
    /// The distinct targets collected so far.
    targets: Vec<Target>,
}

impl Found<'_> {
    /// Records a target unless one with the same root path is already present.
    fn add(&mut self, kind: TargetKind, path: RelPath) {
        let target = Target {
            kind,
            member: self.member.clone(),
            root: path,
        };
        if !self
            .targets
            .iter()
            .any(|existing| existing.root == target.root)
        {
            self.targets.push(target);
        }
    }
}

/// The members found so far, and what may still join them.
struct Adoption<'adopt> {
    /// The members; path dependencies inside the workspace are appended.
    members: &'adopt mut Vec<Member>,
    /// The directories the `exclude` list removed.
    removed: &'adopt [RelPath],
    /// The workspace root.
    root: &'adopt Root,
}

impl Adoption<'_> {
    /// Adds the path dependencies of `manifest` (in `base`) that live inside the
    /// workspace directory, are not excluded and are packages of their own.
    fn adopt(&mut self, base: &RelPath, manifest: &Doc) -> Result<(), Failure> {
        for text in path_dependencies(manifest) {
            let Some(dir) = join_inside(base, &text) else {
                continue;
            };
            let known = self.members.iter().any(|member| member.dir == dir);
            let manifest_path = dir.join("Cargo.toml");
            if known
                || self.removed.contains(&dir)
                || self.root.is_file(&manifest_path) == Answer::No
            {
                continue;
            }
            let candidate = member_at(self.root, dir)?;
            if table_of(&candidate.manifest, &Name::of("workspace")).is_none() {
                self.members.push(candidate);
            }
        }
        Ok(())
    }
}

/// One dependency as written in a manifest: its key and its string fields.
struct DependencySpec {
    /// The string fields of the dependency (`path`, `package`, `version`, ..).
    fields: Vec<(Name, Text)>,
    /// The dependency key, which is the crate name unless `package` renames it.
    key: Name,
}

/// One field of one dependency: `dep` is the dependency key, `name` the field.
struct FieldRef<'name> {
    /// The dependency key.
    dep: &'name Name,
    /// The field name inside the dependency.
    name: &'name Name,
}

impl DependencySpec {
    /// The string field `name`, if the dependency has it.
    #[must_use]
    fn field(&self, name: &Name) -> Option<&Text> {
        self.fields
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
    }

    /// Adds `value` as the field `at` of its dependency, creating the spec on first use.
    fn record(specs: &mut Vec<Self>, at: &FieldRef<'_>, value: &Value) {
        let field: Vec<(Name, Text)> = value
            .as_text()
            .map(|text| (at.name.clone(), text.clone()))
            .into_iter()
            .collect();
        match specs.iter_mut().find(|spec| &spec.key == at.dep) {
            Some(spec) => spec.fields.extend(field),
            None => specs.push(Self {
                fields: field,
                key: at.dep.clone(),
            }),
        }
    }

    /// Adds the fields of an inline dependency table `value` as the dependency `dep`.
    fn record_table(specs: &mut Vec<Self>, dep: &Name, value: &Value) {
        let version = Name::of("version");
        match value.as_table() {
            Some(entries) => {
                for entry in entries {
                    if let Some(name) = entry.single_key() {
                        Self::record(specs, &FieldRef { dep, name }, &entry.value);
                    }
                }
            }
            None => Self::record(
                specs,
                &FieldRef {
                    dep,
                    name: &version,
                },
                value,
            ),
        }
    }
}

/// Where a section sits among the dependency tables.
struct DependencyPlace<'section> {
    /// Is the section a `[dependencies.NAME]`-style table of one dependency?
    inside_one: Answer,
    /// Is the section a `[dependencies]`-style table of many dependencies?
    inside_table: Answer,
    /// The dependency name of a `[dependencies.NAME]` section.
    one: &'section Name,
    /// The section itself.
    section: &'section Section,
}

/// Reads the member package in `dir`.
fn member_at(root: &Root, dir: RelPath) -> Result<Member, Failure> {
    let member_manifest = dir.join("Cargo.toml");
    let manifest = toml::parse(&member_manifest, &root.read(&member_manifest)?)?;
    let name = package_name(&manifest).ok_or_else(|| {
        Failure::parse_at(
            &member_manifest,
            Line::new(1),
            &Message::of("member manifest has no [package] name"),
        )
    })?;
    Ok(Member {
        dir,
        manifest,
        manifest_path: member_manifest,
        name,
    })
}

/// `base` joined with the relative `tail`, with `.` and `..` resolved; `None`
/// when the result leaves the workspace directory.
#[must_use]
fn join_inside(base: &RelPath, tail: &Text) -> Option<RelPath> {
    let mut parts: Vec<&str> = base
        .as_str()
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .collect();
    for part in tail.as_str().split('/') {
        match part {
            "" | "." => {}
            ".." => {
                let keep = parts.len().checked_sub(1)?;
                parts.truncate(keep);
            }
            other => parts.push(other),
        }
    }
    Some(RelPath::from_string(parts.join("/")))
}

/// Reads one entry of a dependency table into `out`.
fn record_dependency_entry(
    place: &DependencyPlace<'_>,
    entry: &Entry,
    out: &mut Vec<DependencySpec>,
) {
    let kinds = ["dependencies", "dev-dependencies", "build-dependencies"];
    let keys: Vec<&Name> = entry.key.iter().collect();
    match place.section.kind {
        SectionKind::Table => match keys.as_slice() {
            [name] if place.inside_one == Answer::Yes => {
                DependencySpec::record(
                    out,
                    &FieldRef {
                        dep: place.one,
                        name,
                    },
                    &entry.value,
                );
            }
            [dep, name] if place.inside_table == Answer::Yes => {
                DependencySpec::record(out, &FieldRef { dep, name }, &entry.value);
            }
            [dep] if place.inside_table == Answer::Yes => {
                DependencySpec::record_table(out, dep, &entry.value);
            }
            _ => {}
        },
        SectionKind::Root => match keys.as_slice() {
            [kind, dep, name] if kinds.contains(&kind.as_str()) => {
                DependencySpec::record(out, &FieldRef { dep, name }, &entry.value);
            }
            [kind, dep] if kinds.contains(&kind.as_str()) => {
                DependencySpec::record_table(out, dep, &entry.value);
            }
            _ => {}
        },
        SectionKind::ArrayOfTables => {}
    }
}

/// Every dependency of `doc`, however it is written: `[dependencies]` entries,
/// dotted keys, `[dependencies.name]` tables, in dev, build and target tables.
fn dependency_specs(doc: &Doc) -> Vec<DependencySpec> {
    let kinds = ["dependencies", "dev-dependencies", "build-dependencies"];
    let mut out: Vec<DependencySpec> = Vec::new();
    let nobody = Name::of("");
    for section in &doc.sections {
        let header: Vec<&str> = section.header.iter().map(Name::as_str).collect();
        let place = DependencyPlace {
            inside_one: header
                .len()
                .checked_sub(2)
                .and_then(|index| header.get(index))
                .is_some_and(|parent| kinds.contains(parent))
                .answer(),
            inside_table: header
                .last()
                .is_some_and(|last| kinds.contains(last))
                .answer(),
            one: match section.header.last() {
                Some(last) => last,
                None => &nobody,
            },
            section,
        };
        for entry in &section.entries {
            record_dependency_entry(&place, entry, &mut out);
        }
    }
    out
}

/// The `path = ".."` values of the dependency tables of `doc`.
fn path_dependencies(doc: &Doc) -> Vec<Text> {
    dependency_specs(doc)
        .iter()
        .filter_map(|spec| spec.field(&Name::of("path")).cloned())
        .collect()
}

/// Dependency names that rename a workspace library, mapped to its library name:
/// `foo = { package = "core-api", .. }` makes `foo::X` mean `core_api::X`.
pub(crate) fn dependency_aliases(ws: &Workspace) -> BTreeMap<Name, Name> {
    let mut aliases = BTreeMap::new();
    let docs = ws
        .members
        .iter()
        .map(|member| &member.manifest)
        .chain(iter::once(&ws.root_manifest));
    for doc in docs {
        for spec in dependency_specs(doc) {
            let Some(package) = spec.field(&Name::of("package")) else {
                continue;
            };
            let Some(member) = ws
                .members
                .iter()
                .find(|member| member.name.as_str() == package.as_str())
            else {
                continue;
            };
            let alias = Name::from_string(spec.key.as_str().replace('-', "_"));
            aliases.extend([(alias, member.lib_name())]);
        }
    }
    aliases
}

/// The entries that a root entry contributes to the top-level table `name`:
/// `None` when it belongs to another key, the entry without its first key
/// segment for a dotted key (`name.key = ..`), and the inner entries of an
/// inline table (`name = { .. }`).
#[must_use]
fn inside_root_entry(entry: &Entry, name: &Name) -> Option<Vec<Entry>> {
    if entry.key.first != *name {
        return None;
    }
    match entry.key.without_first() {
        None => Some(match &entry.value {
            Value::Table(inner) => inner.clone(),
            Value::Str(_) | Value::Int(_) | Value::Bool(_) | Value::Raw(_) | Value::Array(_) => {
                Vec::new()
            }
        }),
        Some(key) => Some(vec![Entry {
            key,
            line: entry.line,
            value: entry.value.clone(),
        }]),
    }
}

/// The entries of the top-level table `name` of `doc`, however it is written:
/// as a `[name]` header, as dotted keys (`name.key = ..`) or as an inline table
/// (`name = { .. }`). `None` when the document has no such table. A table that
/// was read only from its `[name]` header would leave a root `package` or
/// `workspace` written in the other forms unseen, and its members unlinted.
#[must_use]
fn table_of(doc: &Doc, name: &Name) -> Option<Vec<Entry>> {
    let mut entries: Vec<Entry> = Vec::new();
    let mut found = false;
    for section in &doc.sections {
        match section.kind {
            SectionKind::Table if section.header.as_slice() == slice::from_ref(name) => {
                found = true;
                entries.extend(section.entries.iter().cloned());
            }
            SectionKind::Root => {
                for entry in &section.entries {
                    if let Some(inside) = inside_root_entry(entry, name) {
                        found = true;
                        entries.extend(inside);
                    }
                }
            }
            SectionKind::Table | SectionKind::ArrayOfTables => {}
        }
    }
    found.then_some(entries)
}

/// The value for the single-segment `key` in `entries`, if both exist.
#[must_use]
fn value_in<'ctx>(entries: Option<&'ctx [Entry]>, key: &Name) -> Option<&'ctx Value> {
    entries?
        .iter()
        .find(|entry| entry.single_key() == Some(key))
        .map(|entry| &entry.value)
}

/// The string items of the array at `key` in `entries`, or empty if absent.
fn string_list(entries: Option<&[Entry]>, key: &Name) -> Vec<Text> {
    match value_in(entries, key).and_then(Value::as_array) {
        Some(items) => items
            .iter()
            .filter_map(|item| item.as_text().cloned())
            .collect(),
        None => Vec::new(),
    }
}

/// The package name from the `package` table of `manifest`, if present.
#[must_use]
fn package_name(manifest: &Doc) -> Option<Name> {
    let package = table_of(manifest, &Name::of("package"));
    value_in(package.as_deref(), &Name::of("name"))
        .and_then(Value::as_text)
        .map(|text| Name::of(text.as_str()))
}

/// Every directory under the root, skipping `target` and hidden directories.
fn all_directories(root: &Root) -> Result<Vec<RelPath>, Failure> {
    let mut found = Vec::new();
    let mut pending = vec![RelPath::of("")];
    while let Some(dir) = pending.pop() {
        for entry in root.list(&dir)? {
            let name = entry.name.as_str();
            if entry.kind == EntryKind::Dir && name != "target" && !name.starts_with('.') {
                let child = dir.join(name);
                pending.push(child.clone());
                found.push(child);
            }
        }
    }
    found.sort();
    Ok(found)
}

/// Expands a member pattern, with `*`, `?` and `[..]` wildcards as cargo reads
/// them, into matching package directories. A pattern the `glob` crate rejects
/// (an unclosed `[`) or one starting with `/` is exit 2, never a silent miss.
fn expand(pattern: &Text, all_dirs: &[RelPath], root: &Root) -> Result<Vec<RelPath>, Failure> {
    let trimmed = Text::of(pattern.as_str().trim_end_matches('/'));
    let glob = Glob::parse(&trimmed, Brackets::Class)
        .map_err(|message| Failure::parse_at(&RelPath::of("Cargo.toml"), Line::new(1), &message))?;
    if !trimmed.as_str().contains(['*', '?', '[']) {
        let dir = RelPath::of(trimmed.as_str());
        return Ok(if root.is_file(&dir.join("Cargo.toml")) == Answer::Yes {
            vec![dir]
        } else {
            Vec::new()
        });
    }
    Ok(all_dirs
        .iter()
        .filter(|dir| {
            glob.matches(dir) == Answer::Yes && root.is_file(&dir.join("Cargo.toml")) == Answer::Yes
        })
        .cloned()
        .collect())
}

/// The library target and the build script.
fn lib_and_build(root: &Root, member: &Member, found: &mut Found<'_>) {
    let manifest = &member.manifest;
    let package = table_of(manifest, &Name::of("package"));
    let lib_section = table_of(manifest, &Name::of("lib"));
    if let Some(path) = value_in(lib_section.as_deref(), &Name::of("path")).and_then(Value::as_text)
    {
        found.add(TargetKind::Lib, member.dir.join(path.as_str()));
    } else {
        let default = member.dir.join("src/lib.rs");
        if root.is_file(&default) == Answer::Yes {
            found.add(TargetKind::Lib, default);
        }
    }
    let build_value = value_in(package.as_deref(), &Name::of("build"));
    let custom_build = match build_value {
        Some(Value::Str(path)) => Some(member.dir.join(path.as_str())),
        Some(
            Value::Bool(_) | Value::Int(_) | Value::Raw(_) | Value::Array(_) | Value::Table(_),
        )
        | None => None,
    };
    match custom_build {
        Some(path) => found.add(TargetKind::Build, path),
        None => match build_value {
            Some(Value::Bool(false)) => {}
            Some(
                Value::Bool(true)
                | Value::Str(_)
                | Value::Int(_)
                | Value::Raw(_)
                | Value::Array(_)
                | Value::Table(_),
            )
            | None => {
                let default = member.dir.join("build.rs");
                if root.is_file(&default) == Answer::Yes {
                    found.add(TargetKind::Build, default);
                }
            }
        },
    }
}

/// Enumerates every cargo target of `member`: lib, bin, example, test, bench and
/// build script, by `autobins`-style discovery and by explicit `[[bin]]` and
/// `[[test]]` tables. Every member must have a `[member.<name>]` role in the
/// config, or the run exits 2.
pub(crate) fn targets(root: &Root, member: &Member) -> Result<Vec<Target>, Failure> {
    let manifest = &member.manifest;
    let package = table_of(manifest, &Name::of("package"));
    let auto =
        |key: &str| value_in(package.as_deref(), &Name::of(key)) != Some(&Value::Bool(false));
    let mut found = Found {
        member: &member.name,
        targets: Vec::new(),
    };
    lib_and_build(root, member, &mut found);

    // bins
    if auto("autobins") {
        let main = member.dir.join("src/main.rs");
        if root.is_file(&main) == Answer::Yes {
            found.add(TargetKind::Bin, main);
        }
        for path in auto_targets(root, &member.dir.join("src/bin"))? {
            found.add(TargetKind::Bin, path);
        }
    }
    for (kind, table, dir, auto_key) in [
        (TargetKind::Example, "example", "examples", "autoexamples"),
        (TargetKind::Test, "test", "tests", "autotests"),
        (TargetKind::Bench, "bench", "benches", "autobenches"),
    ] {
        if auto(auto_key) {
            for path in auto_targets(root, &member.dir.join(dir))? {
                found.add(kind, path);
            }
        }
        for section in manifest.sections_named(&[Name::of(table)], SectionKind::ArrayOfTables) {
            explicit(root, member, section, &RelPath::of(dir), |path| {
                found.add(kind, path);
            })?;
        }
    }
    for section in manifest.sections_named(&[Name::of("bin")], SectionKind::ArrayOfTables) {
        explicit(root, member, section, &RelPath::of("src/bin"), |path| {
            found.add(TargetKind::Bin, path);
        })?;
    }
    let mut all = found.targets;
    all.sort();
    Ok(all)
}

/// Resolves the source file of an explicit target table and passes it to `add`.
fn explicit(
    root: &Root,
    member: &Member,
    section: &Section,
    default_dir: &RelPath,
    mut add: impl FnMut(RelPath),
) -> Result<(), Failure> {
    let entries = Some(section.entries.as_slice());
    if let Some(path) = value_in(entries, &Name::of("path")).and_then(Value::as_text) {
        add(member.dir.join(path.as_str()));
        return Ok(());
    }
    let Some(name) = value_in(entries, &Name::of("name")).and_then(Value::as_text) else {
        return Err(Failure::parse_at(
            &member.manifest_path,
            section.line,
            &Message::of("target table has neither `path` nor `name`"),
        ));
    };
    let flat = member
        .dir
        .join(default_dir.as_str())
        .join(&format!("{}.rs", name.as_str()));
    let nested = member
        .dir
        .join(default_dir.as_str())
        .join(&format!("{}/main.rs", name.as_str()));
    if root.is_file(&flat) == Answer::Yes {
        add(flat);
    } else if root.is_file(&nested) == Answer::Yes {
        add(nested);
    } else if default_dir.as_str() == "src/bin"
        && root.is_file(&member.dir.join("src/main.rs")) == Answer::Yes
    {
        add(member.dir.join("src/main.rs"));
    } else {
        return Err(Failure::parse_at(
            &member.manifest_path,
            section.line,
            &Message::from_string(format!(
                "cannot find the source of target `{}`",
                name.as_str()
            )),
        ));
    }
    Ok(())
}

/// `dir/*.rs` and `dir/*/main.rs`.
fn auto_targets(root: &Root, dir: &RelPath) -> Result<Vec<RelPath>, Failure> {
    let mut found = Vec::new();
    if root.is_dir(dir) == Answer::No {
        return Ok(found);
    }
    for entry in root.list(dir)? {
        let name = entry.name.as_str();
        match entry.kind {
            EntryKind::File if is_rust_file(&entry.name) == Answer::Yes => {
                found.push(dir.join(name));
            }
            EntryKind::Dir => {
                let main = dir.join(name).join("main.rs");
                if root.is_file(&main) == Answer::Yes {
                    found.push(main);
                }
            }
            EntryKind::Other => {
                // A symbolic link: cargo follows it, so a link to a file or to a
                // directory with a `main.rs` is a target like any other.
                // A link out of the workspace is reported by `unreached-file`, not loaded.
                let linked = dir.join(name);
                if root.escapes(&linked) == Answer::Yes {
                    continue;
                }
                let main = linked.join("main.rs");
                let is_source = root.is_file(&linked) == Answer::Yes
                    && is_rust_file(&entry.name) == Answer::Yes;
                let is_bin_dir =
                    root.is_dir(&linked) == Answer::Yes && root.is_file(&main) == Answer::Yes;
                let target = if is_source {
                    Some(linked)
                } else if is_bin_dir {
                    Some(main)
                } else {
                    None
                };
                found.extend(target);
            }
            EntryKind::File => {}
        }
    }
    Ok(found)
}
