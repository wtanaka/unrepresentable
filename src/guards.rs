//! Configuration the compiler reads: manifests, lint config files, cargo
//! config, and the dependency boundaries.
use alloc::collections::{BTreeMap, BTreeSet, VecDeque};
use core::iter;

use crate::config::{Config, ForbiddenDependency, CONFIG_FILE};

use crate::failure::Failure;

use crate::findings::Finding;

use crate::io::{self, EntryKind, Root};

use crate::rules::{self, RuleId};

use crate::toml::{self, Doc, Entry, Section, SectionKind, Value};

use crate::types::{Answer, Col, Holds as _, Line, Message, Name, RelPath, Source, Text};

use crate::workspace::Workspace;

/// A dependency declared by a manifest.
struct Dependency {
    /// Name of the dependency crate, after any `package` rename.
    name: Name,
}

/// Crate name to the names of the crates it depends on.
pub(crate) type Graph = BTreeMap<Name, BTreeSet<Name>>;

/// The crates reachable from a start crate, each with the crate it was reached through.
struct Reach {
    /// For each reached crate, the crate it was first reached through.
    parent: BTreeMap<Name, Name>,
    /// All crates reached, including the start crate.
    seen: BTreeSet<Name>,
}

/// What one `[[forbidden-dependency]]` rule found in a graph.
pub(crate) struct RuleReport {
    /// Crates that are reached.
    pub findings: Vec<Finding>,
    /// `must-never-reach` names that are not in the graph yet.
    pub notes: Vec<Finding>,
}

/// The repository's files, and the symbolic links to directories that were not
/// followed.
pub(crate) struct Tree {
    /// Every file, symbolic links to files included (they are read through).
    pub files: Vec<RelPath>,
    /// Symbolic links to directories. A walk that followed them could loop, and
    /// one that did not would never find a `clippy.toml` or `.cargo/config`
    /// behind one, so each is reported by `stray-lint-config`.
    pub linked_dirs: Vec<RelPath>,
}

/// A crate that must not reach another, and the crate it reaches.
struct Edge<'edge> {
    /// The crate the rule starts from.
    start: &'edge Name,
    /// The crate that is reached.
    target: &'edge Name,
}

/// One key path a cargo config sets, and the line it is written on.
struct Setting {
    /// The line of the entry that sets it.
    line: Line,
    /// The full key path, from the table header through any dotted or inline-table keys.
    path: Vec<Name>,
}

/// Builds a finding at column 1 with no enclosing item.
fn finding(path: &RelPath, line: Line, rule: RuleId, message: &Message) -> Finding {
    Finding {
        col: Col::new(1),
        item: Text::from_string(String::new()),
        line,
        message: message.clone(),
        path: path.clone(),
        rule: Name::of(rule.name()),
    }
}

/// Whether the section is the plain table named by the dotted header `dotted`.
fn header_is(section: &Section, dotted: &Text) -> Answer {
    let have: Vec<&str> = section.header.iter().map(Name::as_str).collect();
    (section.kind == SectionKind::Table && have.join(".") == dotted.as_str()).answer()
}

/// `manifest-lints` for every member manifest and the root.
pub(crate) fn manifest_lints(ws: &Workspace) -> Vec<Finding> {
    let mut out = Vec::new();
    let root_path = RelPath::of("Cargo.toml");
    let forbids_unsafe = ws
        .root_manifest
        .sections
        .iter()
        .filter(|section| header_is(section, &Text::of("workspace.lints.rust")) == Answer::Yes)
        .flat_map(|section| section.entries.iter())
        .any(|entry| unsafe_code_forbidden(entry) == Answer::Yes);
    if !forbids_unsafe {
        out.push(finding(
            &root_path,
            Line::new(1),
            rules::MANIFEST_LINTS,
            &Message::of(
                "root Cargo.toml needs unsafe_code = \"forbid\" in [workspace.lints.rust]",
            ),
        ));
    }
    for member in &ws.members {
        check_member_lints(&member.manifest_path, &member.manifest, &mut out);
    }
    out
}

/// Whether the entry sets `unsafe_code` to `forbid`.
fn unsafe_code_forbidden(entry: &Entry) -> Answer {
    if entry.single_key().map(Name::as_str) != Some("unsafe_code") {
        return Answer::No;
    }
    match &entry.value {
        Value::Str(text) => (text.as_str() == "forbid").answer(),
        Value::Table(inner) => inner
            .iter()
            .any(|item| {
                item.single_key().map(Name::as_str) == Some("level")
                    && item
                        .value
                        .as_text()
                        .is_some_and(|text| text.as_str() == "forbid")
            })
            .answer(),
        Value::Int(_) | Value::Bool(_) | Value::Raw(_) | Value::Array(_) => Answer::No,
    }
}

/// Checks that a member manifest only inherits workspace lints.
fn check_member_lints(path: &RelPath, doc: &Doc, out: &mut Vec<Finding>) {
    let tables: Vec<&Section> = doc
        .sections
        .iter()
        .filter(|section| header_is(section, &Text::of("lints")) == Answer::Yes)
        .collect();
    let sub_tables = doc.sections.iter().any(|section| {
        section.header.first().map(Name::as_str) == Some("lints") && section.header.len() > 1
    });
    let exact = match tables.as_slice() {
        [only] => matches!(
            only.entries.as_slice(),
            [entry] if entry.single_key().map(Name::as_str) == Some("workspace")
                && entry.value == Value::Bool(true)
        ),
        _ => false,
    };
    if !exact || sub_tables {
        out.push(finding(
            path,
            Line::new(1),
            rules::MANIFEST_LINTS,
            &Message::from_string(format!(
                "{} must contain a [lints] table with exactly `workspace = true` and no [lints.*] tables",
                path.as_str()
            ))));
    }
}

/// Every file in the repository, skipping the root `target`, `.git` and `.jj`.
/// A `target` directory anywhere else is walked: it is not the build output of
/// this workspace, and cargo and clippy read their config from it like any other.
pub(crate) fn all_files(root: &Root) -> Result<Tree, Failure> {
    let mut files = Vec::new();
    let mut linked_dirs = Vec::new();
    let mut pending = vec![RelPath::of("")];
    while let Some(dir) = pending.pop() {
        for entry in root.list(&dir)? {
            let name = entry.name.as_str();
            let path = dir.join(name);
            match entry.kind {
                EntryKind::Dir => {
                    let skipped = (dir.as_str().is_empty() && name == "target")
                        || name == ".git"
                        || name == ".jj";
                    if !skipped {
                        pending.push(path);
                    }
                }
                EntryKind::File => files.push(path),
                EntryKind::Other => match (root.is_file(&path), root.is_dir(&path)) {
                    (Answer::Yes, Answer::Yes | Answer::No) => files.push(path),
                    (Answer::No, Answer::Yes) => linked_dirs.push(path),
                    (Answer::No, Answer::No) => {}
                },
            }
        }
    }
    files.sort();
    linked_dirs.sort();
    Ok(Tree { files, linked_dirs })
}

/// `stray-lint-config`: member clippy files and risky cargo config.
pub(crate) fn stray_lint_config(
    root: &Root,
    config: &Config,
    found: &Tree,
) -> Result<Vec<Finding>, Failure> {
    let mut out = Vec::new();
    let root_has_plain = root.is_file(&RelPath::of("clippy.toml"));
    for link in &found.linked_dirs {
        out.push(finding(
            link,
            Line::new(1),
            rules::STRAY_LINT_CONFIG,
            &Message::of("a symbolic link to a directory is not followed, so a clippy.toml or .cargo/config behind it would go unchecked; replace the link with the real directory"),
        ));
    }
    for path in &found.files {
        let name = path.file_name();
        let is_clippy = name.as_str() == "clippy.toml" || name.as_str() == ".clippy.toml";
        let at_root = path.parent().as_str().is_empty();
        if is_clippy
            && !(at_root && (name.as_str() == "clippy.toml" || root_has_plain == Answer::No))
        {
            out.push(finding(
                path,
                Line::new(1),
                rules::STRAY_LINT_CONFIG,
                &Message::of("clippy reads the nearest clippy.toml, so this file replaces the root one; only the root file is allowed")));
        }
        let in_cargo_dir = path.parent().file_name().as_str() == ".cargo";
        if in_cargo_dir && matches!(name.as_str(), "config" | "config.toml") {
            check_cargo_config(root, config, path, &mut out)?;
        }
    }
    for ancestor in root.ancestor_cargo_configs() {
        check_cargo_config(root, config, &ancestor, &mut out)?;
    }
    Ok(out)
}

/// Appends the path of `value` under `prefix`, then the paths of the entries of
/// an inline table (`build = { rustc-wrapper = ".." }` sets `build.rustc-wrapper`).
/// An `env` path is kept whole, as the exception for it names `env` or `env.X`.
fn settings_of(prefix: &[Name], line: Line, value: &Value, out: &mut Vec<Setting>) {
    out.push(Setting {
        line,
        path: prefix.to_vec(),
    });
    if prefix.first().map(Name::as_str) == Some("env") {
        return;
    }
    match value {
        Value::Table(inner) => {
            for item in inner {
                let mut next = prefix.to_vec();
                next.extend(item.key.iter().cloned());
                settings_of(&next, item.line, &item.value, out);
            }
        }
        Value::Str(_) | Value::Int(_) | Value::Bool(_) | Value::Raw(_) | Value::Array(_) => {}
    }
}

/// Whether a cargo config key path changes what the compiler does: any `env`
/// entry, `rustflags` and `rustdocflags` anywhere, and the `build` keys that
/// swap the compiler or documenter.
fn changes_compiler(path: &[Name]) -> Answer {
    let spelled: Vec<&str> = path.iter().map(Name::as_str).collect();
    match spelled.as_slice() {
        ["env", ..]
        | [.., "rustflags" | "rustdocflags"]
        | ["build", "rustc-wrapper" | "rustc-workspace-wrapper" | "rustc" | "rustdoc"] => {
            Answer::Yes
        }
        _ => Answer::No,
    }
}

/// Reports risky entries of one cargo config file that are not recorded exceptions.
fn check_cargo_config(
    root: &Root,
    config: &Config,
    path: &RelPath,
    out: &mut Vec<Finding>,
) -> Result<(), Failure> {
    let source = read_any(root, path)?;
    let doc = toml::parse(path, &source)?;
    for section in &doc.sections {
        let header: Vec<&str> = section.header.iter().map(Name::as_str).collect();
        let mut settings = Vec::new();
        for entry in &section.entries {
            let mut full = section.header.clone();
            full.extend(entry.key.iter().cloned());
            settings_of(&full, entry.line, &entry.value, &mut settings);
        }
        for setting in &settings {
            let spelled: Vec<&str> = setting.path.iter().map(Name::as_str).collect();
            if changes_compiler(&setting.path) == Answer::No {
                continue;
            }
            let exact = format!("{}:{}", path.as_str(), spelled.join("."));
            let allowed = config
                .cargo_exceptions
                .iter()
                .any(|exception| exception.as_str() == exact);
            if !allowed {
                out.push(finding(
                    path,
                    setting.line,
                    rules::STRAY_LINT_CONFIG,
                    &Message::from_string(format!("cargo config entry changes compiler behaviour; record the exact entry `{exact}` as a tracked exception"))));
            }
        }
        if header.first() == Some(&"env") && section.entries.is_empty() {
            let exact = format!("{}:{}", path.as_str(), header.join("."));
            if !config
                .cargo_exceptions
                .iter()
                .any(|entry| entry.as_str() == exact)
            {
                out.push(finding(
                    path,
                    section.line,
                    rules::STRAY_LINT_CONFIG,
                    &Message::from_string(format!(
                        "cargo [env] table; record the exact entry `{exact}` as a tracked exception"
                    )),
                ));
            }
        }
    }
    Ok(())
}

/// Reads a file in the repository, or one above it when the path starts with `../`.
fn read_any(root: &Root, path: &RelPath) -> Result<Source, Failure> {
    if path.as_str().starts_with("../") {
        let target = root.path().join(path.as_str());
        io::read_file(&target)
    } else {
        root.read(path)
    }
}

/// The items of an optional array.
const fn deps_list(items: Option<&[Value]>) -> &[Value] {
    match items {
        Some(found) => found,
        None => &[],
    }
}

/// Collects the dependencies declared in a manifest, honouring `package` renames.
fn dependencies_of(doc: &Doc) -> Vec<Dependency> {
    let kinds = ["dependencies", "dev-dependencies", "build-dependencies"];
    let mut out = Vec::new();
    for section in &doc.sections {
        if section.kind != SectionKind::Table {
            continue;
        }
        let header: Vec<&str> = section.header.iter().map(Name::as_str).collect();
        let (table_form, named_form) = match header.as_slice() {
            [kind] if kinds.contains(kind) => (true, None),
            ["target", _, kind] if kinds.contains(kind) => (true, None),
            [kind, name] if kinds.contains(kind) => (false, Some(*name)),
            ["target", _, kind, name] if kinds.contains(kind) => (false, Some(*name)),
            _ => continue,
        };
        if table_form {
            for entry in &section.entries {
                let first = &entry.key.first;
                let renamed = match &entry.value {
                    Value::Table(inner) => inner
                        .iter()
                        .find(|item| item.single_key().map(Name::as_str) == Some("package"))
                        .and_then(|item| item.value.as_text())
                        .map(|text| Name::of(text.as_str())),
                    Value::Str(_)
                    | Value::Int(_)
                    | Value::Bool(_)
                    | Value::Raw(_)
                    | Value::Array(_) => None,
                };
                out.push(Dependency {
                    name: match renamed {
                        Some(found) => found,
                        None => first.clone(),
                    },
                });
            }
        }
        if let Some(name) = named_form {
            let renamed = section
                .entries
                .iter()
                .find(|item| item.single_key().map(Name::as_str) == Some("package"))
                .and_then(|item| item.value.as_text())
                .map(|text| Name::of(text.as_str()));
            out.push(Dependency {
                name: match renamed {
                    Some(found) => found,
                    None => Name::of(name),
                },
            });
        }
    }
    out
}

/// The name crates.io compares by: `-` and `_` are the same character and case
/// does not matter.
pub(crate) fn canonical(name: &Name) -> Text {
    Text::from_string(name.as_str().replace('-', "_").to_ascii_lowercase())
}

/// Gives every crate one spelling, the first one seen, so that `rust-decimal`
/// and `rust_decimal` are one node of the graph.
fn intern(spellings: &mut BTreeMap<Text, Name>, name: &Name) -> Name {
    spellings
        .entry(canonical(name))
        .or_insert_with(|| name.clone())
        .clone()
}

/// The `[[package]]` tables of `Cargo.lock`, plus every member's own manifest.
fn dependency_graph(root: &Root, ws: &Workspace) -> Result<Graph, Failure> {
    let lock_path = RelPath::of("Cargo.lock");
    let lock = toml::parse(&lock_path, &root.read(&lock_path)?)?;
    let mut graph: Graph = BTreeMap::new();
    let mut spellings: BTreeMap<Text, Name> = BTreeMap::new();
    for section in &lock.sections {
        if section.kind != SectionKind::ArrayOfTables
            || section.header.len() != 1
            || section.header.first().map(Name::as_str) != Some("package")
        {
            continue;
        }
        let package = section
            .entries
            .iter()
            .find(|entry| entry.single_key().map(Name::as_str) == Some("name"))
            .and_then(|entry| entry.value.as_text())
            .map(|text| Name::of(text.as_str()));
        let Some(name) = package else {
            continue;
        };
        let edges = graph.entry(intern(&mut spellings, &name)).or_default();
        let deps = section
            .entries
            .iter()
            .find(|entry| entry.single_key().map(Name::as_str) == Some("dependencies"))
            .and_then(|entry| entry.value.as_array());
        for dep in deps_list(deps) {
            if let Some(text) = dep.as_text() {
                if let Some(first) = text.as_str().split_whitespace().next() {
                    edges.extend([intern(&mut spellings, &Name::of(first))]);
                }
            }
        }
    }
    for member in &ws.members {
        let edges = graph
            .entry(intern(&mut spellings, &member.name))
            .or_default();
        for dep in dependencies_of(&member.manifest) {
            edges.extend([intern(&mut spellings, &dep.name)]);
        }
    }
    Ok(graph)
}

/// Breadth-first search of the crates reachable from `start`.
fn reachable(graph: &Graph, start: &Name) -> Reach {
    let mut parent: BTreeMap<Name, Name> = BTreeMap::new();
    let mut queue: VecDeque<Name> = VecDeque::new();
    queue.push_back(start.clone());
    let mut seen: BTreeSet<Name> = BTreeSet::new();
    seen.extend([start.clone()]);
    while let Some(current) = queue.pop_front() {
        for next in graph.get(&current).into_iter().flatten() {
            if seen.insert(next.clone()) {
                parent.extend([(next.clone(), current.clone())]);
                queue.push_back(next.clone());
            }
        }
    }
    Reach { parent, seen }
}

/// The error for a `[[forbidden-dependency]]` that could never fire.
fn dead_rule(role: &Text, name: &Name) -> Failure {
    Failure::parse_at(
        &RelPath::of(CONFIG_FILE),
        Line::new(1),
        &Message::from_string(format!(
            "[[forbidden-dependency]] {} `{}` is not a crate in the workspace or Cargo.lock, so the rule could never fire; fix the name",
            role.as_str(),
            name.as_str()
        )),
    )
}

/// The crates of `graph` (nodes and dependencies) by their canonical name.
fn crate_index(graph: &Graph) -> BTreeMap<Text, Name> {
    graph
        .iter()
        .flat_map(|(name, edges)| iter::once(name).chain(edges))
        .map(|name| (canonical(name), name.clone()))
        .collect()
}

/// The note for a `must-never-reach` name that is not in the graph.
fn absent_note(edge: &Edge<'_>) -> Finding {
    finding(
        &RelPath::of(CONFIG_FILE),
        Line::new(1),
        rules::FORBIDDEN_DEPENDENCY,
        &Message::from_string(format!(
            "`{}` is not in the dependency graph; the rule that keeps it out of `{}` fires if it appears",
            edge.target.as_str(),
            edge.start.as_str()
        )),
    )
}

/// Runs one rule over `graph`, whose crates are listed in `index`.
///
/// The start crate and every `must-not-reach` name must be in the graph, else
/// the rule could never fire and the result is an error. A `must-never-reach`
/// name may be absent: it is reported as a note and the rule fires once it
/// appears. Names match with `-` and `_` taken as the same, ignoring case, and a
/// name listed twice counts once.
fn check_rule(
    graph: &Graph,
    index: &BTreeMap<Text, Name>,
    rule: &ForbiddenDependency,
) -> Result<RuleReport, Failure> {
    let Some(start) = index.get(&canonical(&rule.crate_name)) else {
        return Err(dead_rule(&Text::of("crate"), &rule.crate_name));
    };
    for target in &rule.must_not_reach {
        if !index.contains_key(&canonical(target)) {
            return Err(dead_rule(&Text::of("must-not-reach"), target));
        }
    }
    let reach = reachable(graph, start);
    let mut report = RuleReport {
        findings: Vec::new(),
        notes: Vec::new(),
    };
    let mut done: BTreeSet<Text> = BTreeSet::new();
    for named in rule.must_not_reach.iter().chain(&rule.must_never_reach) {
        if !done.insert(canonical(named)) {
            continue;
        }
        match index.get(&canonical(named)) {
            Some(target) if target != start && reach.seen.contains(target) => {
                report
                    .findings
                    .push(reach_finding(&reach, &Edge { start, target }));
            }
            Some(_) => {}
            None => report.notes.push(absent_note(&Edge {
                start,
                target: named,
            })),
        }
    }
    Ok(report)
}

/// Runs every rule over `graph`; see `check_rule`.
pub(crate) fn check_rules(
    graph: &Graph,
    rules: &[ForbiddenDependency],
) -> Result<RuleReport, Failure> {
    let index = crate_index(graph);
    let mut out = RuleReport {
        findings: Vec::new(),
        notes: Vec::new(),
    };
    for rule in rules {
        let mut report = check_rule(graph, &index, rule)?;
        out.findings.append(&mut report.findings);
        out.notes.append(&mut report.notes);
    }
    Ok(out)
}

/// The finding for `start` reaching `target`, with the chain that gets there.
///
/// The item tells two edges of the same rule apart: every one of these findings
/// sits at Cargo.lock:1:1, and a baseline entry for one of them must not cover
/// another.
fn reach_finding(reach: &Reach, edge: &Edge<'_>) -> Finding {
    let (start, target) = (edge.start, edge.target);
    let mut chain = vec![target.as_str().to_owned()];
    let mut cursor = target.clone();
    while let Some(up) = reach.parent.get(&cursor) {
        chain.push(up.as_str().to_owned());
        cursor = up.clone();
    }
    chain.reverse();
    Finding {
        item: Text::from_string(format!("{} -> {}", start.as_str(), target.as_str())),
        ..finding(
            &RelPath::of("Cargo.lock"),
            Line::new(1),
            rules::FORBIDDEN_DEPENDENCY,
            &Message::from_string(format!(
                "`{}` must not reach `{}`: {}",
                start.as_str(),
                target.as_str(),
                chain.join(" -> ")
            )),
        )
    }
}

/// `forbidden-dependency` over manifests and `Cargo.lock`.
///
/// A rule that names a start crate or a `must-not-reach` crate the dependency
/// graph does not contain can never fire, so it is exit 2 and not a pass: a typo
/// in the name would otherwise switch the boundary off. `must-never-reach` is
/// for names that may be absent; each absent one is a note on every run. A rule
/// that lists no crate to keep out is exit 2 too, but already when the config is
/// read (see `config::parse`).
pub(crate) fn forbidden_dependencies(
    root: &Root,
    config: &Config,
    ws: &Workspace,
) -> Result<RuleReport, Failure> {
    if config.forbidden_dependencies.is_empty() {
        return Ok(RuleReport {
            findings: Vec::new(),
            notes: Vec::new(),
        });
    }
    check_rules(&dependency_graph(root, ws)?, &config.forbidden_dependencies)
}
