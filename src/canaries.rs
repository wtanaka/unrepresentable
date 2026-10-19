//! Line-anchored canaries: files of known-bad code that must fail for the
//! stated reason at the stated line.
use alloc::collections::{BTreeMap, BTreeSet};

use crate::analyze;

use crate::config::{ClippyCanaries, Config, MemberConfig, Role, CONFIG_FILE};

use crate::failure::Failure;

use crate::findings::Finding;

use crate::index::{self, Resolver, WorkspaceIndex};

use crate::io::{EntryKind, Root};

use crate::json::{self, Flag, Json};

use crate::ledger::{self, CANARY_EXEMPTIONS_FILE};

use crate::lintcov::Universe;

use crate::loader;

use crate::process::{self, Args, Bytes};

use crate::rules;

use crate::scan::{self, Env};

use crate::toml::{self, SectionKind};

use crate::types::{Answer, Col, Count, Holds as _, Line, Message, Name, RelPath, Source, Text};

use crate::util::{is_rust_file, or_empty};

use crate::workspace::{Target, TargetKind, Workspace};

/// One line a canary expects to fail, and with which id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Mark {
    /// The expected diagnostic id, possibly with an `(entry)` suffix.
    pub id: Name,
    /// The line carrying the `//~` marker.
    pub line: Line,
}

/// One diagnostic a canary produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Seen {
    /// The id of the diagnostic.
    pub id: Name,
    /// The line the diagnostic points at.
    pub line: Line,
}

/// The outcome of one rule canary.
pub(crate) struct CanaryResult {
    /// Mismatches between the marks and the diagnostics.
    pub findings: Vec<Finding>,
    /// The base ids that the canary's marks cover.
    pub ids: BTreeSet<Name>,
}

// ---- compiler-lint canaries ---------------------------------------------

/// One compiler diagnostic read from cargo's JSON output.
struct Diagnostic {
    /// The lint or error code, if the diagnostic has one.
    code: Option<Text>,
    /// The first line of the primary span.
    line: Line,
    /// The source text of the primary span, lines joined by newlines.
    span_text: Text,
}

/// One entry of a `disallowed-*` array in the root `clippy.toml`.
struct DisallowedEntry {
    /// The `disallowed-*` key the entry came from.
    key: Name,
    /// The path the entry disallows.
    path: Text,
}

/// The per-run context of the compiler-lint canaries.
struct ClippyRun<'run> {
    /// The canary package that clippy runs on.
    package: &'run Name,
    /// The workspace root where cargo runs.
    root: &'run Root,
}

/// The result of checking one clippy canary binary.
struct ClippyOutcome {
    /// Mismatches and breakage found in the canary.
    findings: Vec<Finding>,
    /// The ids that the canary marks.
    marked: BTreeSet<Name>,
}

/// The files of the clippy canary directory.
struct CanaryFiles<'files> {
    /// Every `.rs` file under the directory.
    all: &'files [RelPath],
    /// The files directly in the directory, which are the canaries.
    roots: &'files [RelPath],
}

/// The lints that must be covered and the ids the canaries mark.
struct LintSets<'sets> {
    /// Lints denied by the workspace that need a canary.
    denied: &'sets BTreeSet<Name>,
    /// Ids marked by the canaries.
    marked: &'sets BTreeSet<Name>,
}

/// Reads every `//~ <id>` marker in `source` as a mark on its line.
pub(crate) fn parse_marks(source: &Source) -> Result<Vec<Mark>, Failure> {
    let mut marks = Vec::new();
    for (index, text) in source.as_str().lines().enumerate() {
        let Some((_, tail)) = text.split_once("//~") else {
            continue;
        };
        for id in tail.split_whitespace() {
            marks.push(Mark {
                id: Name::of(id),
                line: Line::from_index(Count::new(index))?,
            });
        }
    }
    Ok(marks)
}

/// A canary-rule finding at `line` of `path`.
/// The message doubles as the item: canary findings are never baselined, and
/// several of them sit at one place (line 1 of a directory), so the message is
/// what keeps `normalise` from merging them.
pub(crate) fn canary_finding(path: &RelPath, line: Line, message: &Message) -> Finding {
    Finding {
        col: Col::new(1),
        item: Text::of(message.as_str()),
        line,
        message: message.clone(),
        path: path.clone(),
        rule: Name::of(rules::CANARY.name()),
    }
}

/// The id without a trailing `(entry)` suffix.
fn base_id(id: &Name) -> Name {
    match id.as_str().split_once('(') {
        Some((base, _)) => Name::of(base),
        None => id.clone(),
    }
}

/// Compares marks with diagnostics and names every mismatch.
pub(crate) fn verify(path: &RelPath, marks: &[Mark], diagnostics: &[Seen]) -> Vec<Finding> {
    let mut out = Vec::new();
    if marks.is_empty() {
        out.push(canary_finding(
            path,
            Line::new(1),
            &Message::of("canary has no `//~ <id>` marks"),
        ));
        return out;
    }
    for mark in marks {
        let wanted = base_id(&mark.id);
        let found = diagnostics
            .iter()
            .any(|seen| seen.line == mark.line && seen.id == wanted);
        if !found {
            out.push(canary_finding(
                path,
                mark.line,
                &Message::from_string(format!(
                    "expected `{}` on this line, but it did not fire",
                    wanted.as_str()
                )),
            ));
        }
    }
    for seen in diagnostics {
        let on_line = marks
            .iter()
            .any(|mark| mark.line == seen.line && base_id(&mark.id) == seen.id);
        if !on_line {
            out.push(canary_finding(
                path,
                seen.line,
                &Message::from_string(format!(
                    "unmarked diagnostic `{}`: mark this line with `//~ {}`",
                    seen.id.as_str(),
                    seen.id.as_str()
                )),
            ));
        }
    }
    out
}

/// Scans in-memory source as a one-file crate with the given role.
pub(crate) fn scan_source(
    config: &Config,
    role: Role,
    path: &RelPath,
    source: &Source,
) -> Result<Vec<Finding>, Failure> {
    let member = Name::of("canary");
    let krate = loader::in_memory(path, source)?;
    let member_config = MemberConfig {
        ambient: Vec::new(),
        name: member,
        newtypes: Vec::new(),
        role,
    };
    let own = index::build_crate(&krate);
    let workspace = WorkspaceIndex {
        aliases: BTreeMap::new(),
        externs: BTreeSet::new(),
        libs: BTreeMap::new(),
    };
    let mut newtypes = BTreeSet::new();
    for file in &krate.files {
        analyze::collect_newtypes(&file.ast.items, &mut newtypes);
    }
    let leaves = analyze::banned_leaves(config);
    let env = Env {
        banned_leaves: &leaves,
        config,
        member: &member_config,
        newtypes: &newtypes,
        resolver: Resolver {
            own: &own,
            workspace: &workspace,
        },
    };
    Ok(scan::scan_crate(&env, &krate).findings)
}

/// The role named by a canary's first line, `// canary role=pure` or `role=shell`.
#[must_use]
fn role_of_header(source: &Source) -> Option<Role> {
    let first = source.as_str().lines().next()?;
    let tail = first.trim().strip_prefix("// canary role=")?;
    match tail.trim() {
        "pure" => Some(Role::Pure),
        "shell" => Some(Role::Shell),
        _ => None,
    }
}

/// Checks one rule-canary source and returns its findings and the ids it marks.
pub(crate) fn check_rule_canary(
    config: &Config,
    path: &RelPath,
    source: &Source,
) -> Result<CanaryResult, Failure> {
    let Some(role) = role_of_header(source) else {
        return Ok(CanaryResult {
            findings: vec![canary_finding(
                path,
                Line::new(1),
                &Message::of("first line must be `// canary role=pure` or `// canary role=shell`"),
            )],
            ids: BTreeSet::new(),
        });
    };
    let marks = parse_marks(source)?;
    let scanned = scan_source(config, role, path, source)?;
    let diagnostics: Vec<Seen> = scanned
        .iter()
        .map(|finding| Seen {
            id: finding.rule.clone(),
            line: finding.line,
        })
        .collect();
    let ids = marks.iter().map(|mark| base_id(&mark.id)).collect();
    Ok(CanaryResult {
        findings: verify(path, &marks, &diagnostics),
        ids,
    })
}

/// Rules that must be covered by marks somewhere.
pub(crate) fn required_rule_ids(config: &Config) -> Vec<Name> {
    let mut ids: Vec<Name> = rules::CANARY_RULES
        .iter()
        .map(|id| Name::of(id.name()))
        .collect();
    for ban in &config.bans {
        ids.push(Name::from_string(format!(
            "{}:{}",
            rules::BANNED.name(),
            ban.id.as_str()
        )));
    }
    ids
}

/// Findings for each required rule id that no canary marks.
pub(crate) fn coverage_findings(
    config: &Config,
    covered: &BTreeSet<Name>,
    anchor: &RelPath,
) -> Vec<Finding> {
    required_rule_ids(config)
        .into_iter()
        .filter(|id| !covered.contains(id))
        .map(|id| {
            canary_finding(
                anchor,
                Line::new(1),
                &Message::from_string(format!("no canary marks rule `{}`", id.as_str())),
            )
        })
        .collect()
}

/// Every `.rs` file under `dir`, recursively and sorted.
fn rust_files(root: &Root, dir: &RelPath) -> Result<Vec<RelPath>, Failure> {
    let mut out = Vec::new();
    let mut pending = vec![dir.clone()];
    while let Some(current) = pending.pop() {
        if root.is_dir(&current) == Answer::No {
            continue;
        }
        for entry in root.list(&current)? {
            let path = current.join(entry.name.as_str());
            match entry.kind {
                EntryKind::Dir => pending.push(path),
                EntryKind::File if is_rust_file(&entry.name) == Answer::Yes => out.push(path),
                EntryKind::File | EntryKind::Other => {}
            }
        }
    }
    out.sort();
    Ok(out)
}

/// Rule canaries from the config's canary directories.
pub(crate) fn run_rule_canaries(root: &Root, config: &Config) -> Result<Vec<Finding>, Failure> {
    let mut findings = Vec::new();
    let mut covered = BTreeSet::new();
    for dir in &config.canary_dirs {
        for path in rust_files(root, dir)? {
            let source = root.read(&path)?;
            let mut result = check_rule_canary(config, &path, &source)?;
            findings.append(&mut result.findings);
            covered.extend(result.ids);
        }
    }
    findings.extend(coverage_findings(
        config,
        &covered,
        &RelPath::of(CONFIG_FILE),
    ));
    Ok(findings)
}

/// The string value under `key` of a JSON object, if present.
#[must_use]
fn key_text(json: &Json, key: &Name) -> Option<Text> {
    json.get(key).and_then(Json::as_text).cloned()
}

/// Reads the compiler messages from cargo's JSON-lines output.
fn parse_diagnostics(stdout: &Bytes) -> Result<Vec<Diagnostic>, Failure> {
    let text = stdout.to_text();
    let mut out = Vec::new();
    for (index, line) in text.as_str().lines().enumerate() {
        if !line.starts_with('{') {
            continue;
        }
        let value = json::parse(&Source::from_string(line.to_owned())).map_err(|error| {
            Failure::parse_at(
                &RelPath::of("<cargo json>"),
                Line::new(index),
                &Message::from_string(format!("cannot read cargo JSON: {}", error.as_str())),
            )
        })?;
        if key_text(&value, &Name::of("reason"))
            .as_ref()
            .map(Text::as_str)
            != Some("compiler-message")
        {
            continue;
        }
        let Some(message) = value.get(&Name::of("message")) else {
            continue;
        };
        if let Some(found) = diagnostic_of(message) {
            out.push(found);
        }
    }
    Ok(out)
}

/// The diagnostic of a compiler message, from its code and primary span.
#[must_use]
fn diagnostic_of(message: &Json) -> Option<Diagnostic> {
    let code = message
        .get(&Name::of("code"))
        .and_then(|code| key_text(code, &Name::of("code")));
    let spans = message.get(&Name::of("spans")).and_then(Json::as_array);
    let primary = or_empty(spans)
        .iter()
        .find(|span| span.get(&Name::of("is_primary")).and_then(Json::as_flag) == Some(Flag::On))?;
    let line_start = primary
        .get(&Name::of("line_start"))
        .and_then(Json::as_count)?;
    let span_text: Vec<&str> = or_empty(primary.get(&Name::of("text")).and_then(Json::as_array))
        .iter()
        .filter_map(|entry| entry.get(&Name::of("text")).and_then(Json::as_text))
        .map(Text::as_str)
        .collect();
    Some(Diagnostic {
        code,
        line: Line::new(line_start.get()),
        span_text: Text::from_string(span_text.join("\n")),
    })
}

/// Is `code` a rustc error code such as `E0308`?
fn is_rustc_error_code(code: &Text) -> Answer {
    let text = code.as_str();
    (text.len() == 5 && text.starts_with('E') && text.chars().skip(1).all(|ch| ch.is_ascii_digit()))
        .answer()
}

/// The name with dashes replaced by underscores.
fn snake(name: &Name) -> Name {
    Name::from_string(name.as_str().replace('-', "_"))
}

/// The entries of every `disallowed-*` array in the root `clippy.toml`.
fn disallowed_entries(root: &Root) -> Result<Vec<DisallowedEntry>, Failure> {
    let path = RelPath::of("clippy.toml");
    if root.is_file(&path) == Answer::No {
        return Ok(Vec::new());
    }
    let doc = toml::parse(&path, &root.read(&path)?)?;
    let mut out = Vec::new();
    for section in doc
        .sections
        .iter()
        .filter(|section| section.kind == SectionKind::Root)
    {
        for entry in &section.entries {
            let Some(key) = entry.single_key() else {
                continue;
            };
            if !key.as_str().starts_with("disallowed-") {
                continue;
            }
            for item in or_empty(entry.value.as_array()) {
                let name = match item {
                    toml::Value::Str(text) => Some(text.clone()),
                    toml::Value::Table(fields) => fields
                        .iter()
                        .find(|field| field.single_key().map(Name::as_str) == Some("path"))
                        .and_then(|field| field.value.as_text())
                        .cloned(),
                    toml::Value::Int(_)
                    | toml::Value::Bool(_)
                    | toml::Value::Raw(_)
                    | toml::Value::Array(_) => None,
                };
                if let Some(found) = name {
                    out.push(DisallowedEntry {
                        key: key.clone(),
                        path: found,
                    });
                }
            }
        }
    }
    Ok(out)
}

/// Runs clippy on one canary binary and checks its marks.
fn check_clippy_canary(run: &ClippyRun<'_>, path: &RelPath) -> Result<ClippyOutcome, Failure> {
    let name = path.file_name();
    let bin = name.as_str().trim_end_matches(".rs").to_owned();
    let source = run.root.read(path)?;
    let marks = parse_marks(&source)?;
    let args = Args::of(&["clippy", "-p", run.package.as_str(), "--bin", &bin])
        .with("--message-format=json")
        .with("--")
        .with("-D")
        .with("warnings");
    let captured = process::cargo(run.root.path(), &args)?;
    let diagnostics = parse_diagnostics(&captured.stdout)?;
    let mut findings = Vec::new();
    let mut broken = false;
    for diagnostic in &diagnostics {
        match &diagnostic.code {
            Some(code) if is_rustc_error_code(code) == Answer::Yes => {
                findings.push(canary_finding(
                    path,
                    diagnostic.line,
                    &Message::from_string(format!(
                        "canary is broken: it fails with rustc error {}",
                        code.as_str()
                    )),
                ));
                broken = true;
            }
            Some(_) => {}
            None => {
                findings.push(canary_finding(
                    path,
                    diagnostic.line,
                    &Message::of("canary is broken: a diagnostic without a lint code"),
                ));
                broken = true;
            }
        }
    }
    if broken {
        return Ok(ClippyOutcome {
            findings,
            marked: BTreeSet::new(),
        });
    }
    let seen: Vec<Seen> = diagnostics
        .iter()
        .filter_map(|diagnostic| {
            diagnostic.code.as_ref().map(|code| Seen {
                id: Name::of(code.as_str()),
                line: diagnostic.line,
            })
        })
        .collect();
    findings.extend(verify(path, &marks, &seen));
    findings.extend(check_entry_marks(path, &marks, &diagnostics));
    Ok(ClippyOutcome {
        findings,
        marked: marks.into_iter().map(|mark| mark.id).collect(),
    })
}

/// Marks of the form `code(entry)` also need the entry's leaf on the marked line.
fn check_entry_marks(path: &RelPath, marks: &[Mark], diagnostics: &[Diagnostic]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for mark in marks {
        let Some((id, raw_entry)) = mark.id.as_str().split_once('(') else {
            continue;
        };
        let entry = raw_entry.trim_end_matches(')');
        let leaf = match entry.rsplit("::").next() {
            Some(last) => last,
            None => entry,
        };
        let text_ok = diagnostics.iter().any(|diagnostic| {
            diagnostic.line == mark.line
                && diagnostic.code.as_ref().map(Text::as_str) == Some(id)
                && diagnostic.span_text.as_str().contains(leaf)
        });
        if !text_ok {
            findings.push(canary_finding(
                path,
                mark.line,
                &Message::from_string(format!(
                    "the marked line does not mention `{leaf}` for disallowed entry `{entry}`"
                )),
            ));
        }
    }
    findings
}

/// The lints with a row in the canary exemptions table, in snake case.
fn exempt_lints(root: &Root) -> Result<BTreeSet<Name>, Failure> {
    let path = RelPath::of(CANARY_EXEMPTIONS_FILE);
    if root.is_file(&path) == Answer::No {
        return Ok(BTreeSet::new());
    }
    let rows = ledger::table_rows(&root.read(&path)?, &Text::of("## Canary exemptions"))?;
    Ok(rows
        .iter()
        .filter_map(|row| row.cells.first())
        .map(|cell| snake(&Name::of(cell.as_str())))
        .collect())
}

/// Findings about the canary package itself: it must be a workspace member and
/// must not be in `default-members`.
fn canary_package_findings(ws: &Workspace, settings: &ClippyCanaries) -> Vec<Finding> {
    let mut findings = Vec::new();
    let member = ws
        .members
        .iter()
        .find(|member| member.name == settings.package);
    match member {
        None => findings.push(canary_finding(
            &RelPath::of(CONFIG_FILE),
            Line::new(1),
            &Message::from_string(format!(
                "canary package `{}` is not a workspace member",
                settings.package.as_str()
            )),
        )),
        Some(found) => {
            if ws.default_members.contains(&found.dir) {
                findings.push(canary_finding(
                    &found.manifest_path,
                    Line::new(1),
                    &Message::of("the canary package must not be in default-members"),
                ));
            }
        }
    }
    findings
}

/// Compiler-lint canaries: runs clippy on each canary binary.
pub(crate) fn run_clippy_canaries(
    root: &Root,
    config: &Config,
    ws: &Workspace,
    universe: &Universe,
    denied: &BTreeSet<Name>,
) -> Result<Vec<Finding>, Failure> {
    let Some(settings) = &config.clippy_canaries else {
        return Ok(Vec::new());
    };
    let mut findings = Vec::new();
    let member = ws
        .members
        .iter()
        .find(|member| member.name == settings.package);
    findings.extend(canary_package_findings(ws, settings));
    let run = ClippyRun {
        package: &settings.package,
        root,
    };
    let mut marked: BTreeSet<Name> = BTreeSet::new();
    let all_files = rust_files(root, &settings.dir)?;
    let mut roots: Vec<RelPath> = Vec::new();
    for path in all_files
        .iter()
        .filter(|path| path.parent() == settings.dir)
    {
        let mut outcome = check_clippy_canary(&run, path)?;
        findings.append(&mut outcome.findings);
        marked.extend(outcome.marked);
        roots.push(path.clone());
    }
    if let Some(found) = member {
        findings.extend(unrun_canary_files(
            root,
            &found.dir,
            &CanaryFiles {
                all: &all_files,
                roots: &roots,
            },
        ));
    }
    findings.extend(coverage_of_lints(
        root,
        config,
        universe,
        &LintSets {
            denied,
            marked: &marked,
        },
    )?);
    Ok(findings)
}

/// Findings for files in the clippy canary directory that no canary run reaches:
/// only the files directly in the directory are canaries, and a file deeper down
/// counts only when some canary loads it with `mod`. Production code parked there
/// would otherwise be scanned by nothing: the normal scan skips canary directories.
fn unrun_canary_files(root: &Root, member_dir: &RelPath, files: &CanaryFiles<'_>) -> Vec<Finding> {
    let mut reached: BTreeSet<RelPath> = BTreeSet::new();
    for canary in files.roots {
        let target = Target {
            kind: TargetKind::Bin,
            member: Name::of("canary"),
            root: canary.clone(),
        };
        match loader::load_crate(root, member_dir, &target) {
            Ok(krate) => reached.extend(krate.files.into_iter().map(|file| file.path)),
            // A canary that this tool cannot parse (it may be written in syntax
            // the lint under test needs) is still run by the clippy canary run;
            // only its own file counts as reached, since its `mod` lines cannot be read.
            Err(_unparsed) => reached.extend([canary.clone()]),
        }
    }
    files
        .all
        .iter()
        .filter(|path| !reached.contains(*path))
        .map(|path| {
            canary_finding(
                path,
                Line::new(1),
                &Message::of(
                    "this file is not a canary run and no canary loads it with `mod`, and canary directories are not scanned as code; move it out or load it from a canary",
                ),
            )
        })
        .collect()
}

/// Findings for denied lints and disallowed entries that no clippy canary marks.
fn coverage_of_lints(
    root: &Root,
    config: &Config,
    universe: &Universe,
    sets: &LintSets<'_>,
) -> Result<Vec<Finding>, Failure> {
    let (denied, marked) = (sets.denied, sets.marked);
    let Some(settings) = &config.clippy_canaries else {
        return Ok(Vec::new());
    };
    let mut findings = Vec::new();
    let exempt = exempt_lints(root)?;
    let marked_bases: BTreeSet<Name> = marked.iter().map(base_id).collect();
    for lint in denied {
        if universe.groups.contains_key(lint) {
            continue;
        }
        let code = snake(lint);
        if !marked_bases.contains(&code) && !exempt.contains(&code) {
            findings.push(canary_finding(
                &settings.dir,
                Line::new(1),
                &Message::from_string(format!(
                    "no clippy canary marks `{}`; add one or an exemption row in {CANARY_EXEMPTIONS_FILE}",
                    lint.as_str()
                )),
            ));
        }
    }
    for entry in disallowed_entries(root)? {
        let code = format!("clippy::{}", snake(&entry.key).as_str());
        let wanted = Name::from_string(format!("{code}({})", entry.path.as_str()));
        if !marked.contains(&wanted) {
            findings.push(canary_finding(
                &settings.dir,
                Line::new(1),
                &Message::from_string(format!("no clippy canary marks `{}`", wanted.as_str())),
            ));
        }
    }
    Ok(findings)
}
