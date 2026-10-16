//! `lint-coverage`: proves the lint configuration is maximal.
use alloc::collections::{BTreeMap, BTreeSet};

use crate::config::LEDGER_FILE;

use crate::failure::Failure;

use crate::findings::Finding;

use crate::io::Root;

use crate::knobs;

use crate::ledger::{self, Ledger, Omission, OmissionRow, UNSTABLE_MARKER};

use crate::process::{self, Args, Success};

use crate::rules;

use crate::toml::{self, Doc, Section, SectionKind, Value};

use crate::types::{Answer, Col, Count, Holds as _, Line, Message, Name, RelPath, Source, Text};

use crate::workspace::{self, Member, TargetKind, Workspace};

/// Names of the clippy lint groups whose members must all be covered.
const CLIPPY_GROUPS: &[&str] = &[
    "correctness",
    "suspicious",
    "style",
    "complexity",
    "perf",
    "pedantic",
    "nursery",
    "cargo",
    "restriction",
];

/// A lint level as written in a lint table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Level {
    /// The lint is silenced.
    Allow,
    /// The lint is an error, which can still be overridden locally.
    Deny,
    /// The lint is an error that cannot be overridden locally.
    Forbid,
    /// The lint is reported as a warning.
    Warn,
}

impl Level {
    /// Whether the level is `deny` or `forbid`.
    const fn is_strong(self) -> Answer {
        match self {
            Self::Allow | Self::Warn => Answer::No,
            Self::Deny | Self::Forbid => Answer::Yes,
        }
    }

    /// Parses `allow`, `warn`, `deny` or `forbid`; `None` for anything else.
    #[must_use]
    pub(crate) fn parse(text: &Text) -> Option<Self> {
        match text.as_str() {
            "allow" => Some(Self::Allow),
            "warn" => Some(Self::Warn),
            "deny" => Some(Self::Deny),
            "forbid" => Some(Self::Forbid),
            _ => None,
        }
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

/// A lint priority from a root lint table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Priority(i64);

impl Priority {
    /// The priority as a plain integer.
    pub(crate) const fn get(self) -> i64 {
        self.0
    }

    /// Wraps an integer as a priority.
    pub(crate) const fn new(value: i64) -> Self {
        Self(value)
    }
}

/// Every lint and lint group known to the pinned toolchain.
#[derive(Clone, Debug)]
pub(crate) struct Universe {
    /// Names of all clippy lints, `clippy::` prefixed.
    pub clippy_lints: BTreeSet<Name>,
    /// Lint groups by name, each mapped to its member lint names.
    pub groups: BTreeMap<Name, Vec<Name>>,
    /// Names of all rustc lints.
    pub rust_lints: BTreeSet<Name>,
}

impl Universe {
    /// A universe with no lints, to be filled from the help output.
    const fn empty() -> Self {
        Self {
            clippy_lints: BTreeSet::new(),
            groups: BTreeMap::new(),
            rust_lints: BTreeSet::new(),
        }
    }
}

/// One entry of a root lint table.
#[derive(Clone, Debug)]
pub(crate) struct LintSetting {
    /// The level the lint is set to.
    pub level: Level,
    /// One-based line of the entry in the manifest.
    pub line: Line,
    /// The lint name as written, without the tool prefix.
    pub name: Name,
    /// The priority of the entry, `0` when none is given.
    pub priority: Priority,
    /// The lint table the entry sits in, such as `rust` or `clippy`.
    pub tool: Name,
}

impl LintSetting {
    /// The lint as `-W help` prints it: kebab-case, `clippy::` prefixed for clippy.
    /// Rustc lints (the `rust` table) have no prefix; every other tool keeps its
    /// own, so `[workspace.lints.rustdoc] unused_must_use` is `rustdoc::unused-must-use`
    /// and never covers the rustc lint of the same name.
    pub(crate) fn full_name(&self) -> Name {
        let kebab = kebab(&self.name);
        if self.tool.as_str() == "rust" {
            kebab
        } else {
            Name::from_string(format!("{}::{}", self.tool.as_str(), kebab.as_str()))
        }
    }

    /// Whether the setting belongs to the `clippy` lint table.
    fn is_clippy(&self) -> Answer {
        (self.tool.as_str() == "clippy").answer()
    }
}

/// Outcome of checking lint coverage.
pub(crate) struct Coverage {
    /// Every lint set to deny or forbid, groups expanded; `clippy::` prefixed for clippy.
    pub denied: BTreeSet<Name>,
    /// Violations found, to be reported as failures.
    pub findings: Vec<Finding>,
    /// Informational findings that do not fail the run.
    pub notes: Vec<Finding>,
}

/// The part of `-W help` output that the parser is currently inside.
#[derive(Clone, Copy, PartialEq, Eq)]
enum HelpSection {
    /// The table of clippy lint checks.
    ClippyChecks,
    /// The table of clippy lint groups.
    ClippyGroups,
    /// Text outside any lint table.
    Outside,
    /// The table of rustc lint checks.
    RustChecks,
    /// The table of rustc lint groups.
    RustGroups,
}

/// What one omission row yields.
struct RowReport {
    /// Violations found for the row.
    findings: Vec<Finding>,
    /// Informational findings for the row.
    notes: Vec<Finding>,
}

/// How specifically a lint table entry names a lint; a lint named by itself
/// outranks a group entry of the same priority.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Specificity {
    /// The entry names a lint group.
    Group,
    /// The entry names the lint itself.
    Lint,
}

/// The entry that currently decides a lint's level.
#[derive(Clone, Copy, Debug)]
struct Winner {
    /// The level the winning entry sets.
    level: Level,
    /// The priority of the winning entry.
    priority: Priority,
    /// Whether the winning entry is a group or the lint itself.
    specificity: Specificity,
}

/// Converts a lint name from underscore-separated to hyphen-separated form.
pub(crate) fn kebab(name: &Name) -> Name {
    Name::from_string(name.as_str().replace('_', "-"))
}

/// Builds a parse failure for the lint help output at `line`.
fn help_failure(line: Count, text: &Message) -> Failure {
    Failure::parse_at(
        &RelPath::of("<lint help output>"),
        Line::new(line.get()),
        &Message::from_string(format!("cannot parse the lint tables: {}", text.as_str())),
    )
}

/// Runs cargo in the root and turns a failing exit status into an error.
fn run_cargo(root: &Root, args: &Args) -> Result<process::Captured, Failure> {
    let captured = process::cargo(root.path(), args)?;
    match captured.success {
        Success::No => Err(Failure::io(&Message::from_string(format!(
            "cargo failed: {}",
            captured.stderr.as_str().trim()
        )))),
        Success::Yes => Ok(captured),
    }
}

/// The table that a heading line of the help output opens, if it is one.
#[must_use]
fn section_of(trimmed: &Text) -> Option<HelpSection> {
    match trimmed.as_str() {
        "Lint checks provided by rustc:" => Some(HelpSection::RustChecks),
        "Lint groups provided by rustc:" => Some(HelpSection::RustGroups),
        "Lint checks loaded by this crate:" => Some(HelpSection::ClippyChecks),
        "Lint groups loaded by this crate:" => Some(HelpSection::ClippyGroups),
        _ => None,
    }
}

/// Parses `cargo clippy -- -W help` output into the lint universe.
pub(crate) fn parse_help(text: &Source) -> Result<Universe, Failure> {
    let mut universe = Universe::empty();
    let mut section = HelpSection::Outside;
    let mut seen_rows = false;
    for (index, line) in text.as_str().lines().enumerate() {
        let number = Count::new(index).next()?;
        let trimmed = line.trim();
        if let Some(next) = section_of(&Text::of(trimmed)) {
            if section == HelpSection::ClippyGroups {
                // clippy prints the tables once per target; the first copy is enough.
                break;
            }
            section = next;
            seen_rows = false;
            continue;
        }
        let ignorable = trimmed.is_empty() || trimmed.starts_with("name ");
        if section == HelpSection::Outside || ignorable || trimmed.starts_with("----") {
            seen_rows = seen_rows || trimmed.starts_with("----");
            continue;
        }
        if !seen_rows {
            continue;
        }
        add_row(&mut universe, section, &Text::of(trimmed), number)?;
    }
    if universe.rust_lints.is_empty() || universe.clippy_lints.is_empty() {
        return Err(help_failure(
            Count::new(0),
            &Message::of("no lint tables found"),
        ));
    }
    Ok(universe)
}

/// Parses one table row of the help output and records it in `universe`.
fn add_row(
    universe: &mut Universe,
    section: HelpSection,
    row: &Text,
    number: Count,
) -> Result<(), Failure> {
    let trimmed = row.as_str();
    let mut parts = trimmed.split_whitespace();
    let Some(name) = parts.next() else {
        return Err(help_failure(number, &Message::of("empty row")));
    };
    match section {
        HelpSection::RustChecks | HelpSection::ClippyChecks => {
            let Some(default) = parts.next() else {
                return Err(help_failure(
                    number,
                    &Message::of("row without a default level"),
                ));
            };
            if Level::parse(&Text::of(default)).is_none() {
                return Err(help_failure(
                    number,
                    &Message::of("default level is not allow/warn/deny/forbid"),
                ));
            }
            let target = match section {
                HelpSection::RustChecks => &mut universe.rust_lints,
                HelpSection::ClippyChecks => &mut universe.clippy_lints,
                HelpSection::Outside | HelpSection::RustGroups | HelpSection::ClippyGroups => {
                    return Err(help_failure(
                        number,
                        &Message::of("row outside a lint table"),
                    ));
                }
            };
            target.extend([Name::of(name)]);
        }
        HelpSection::RustGroups | HelpSection::ClippyGroups => {
            if name == "warnings" {
                return Ok(());
            }
            let rest = match trimmed.strip_prefix(name) {
                Some(found) => found.trim(),
                None => "",
            };
            let members: Vec<Name> = rest
                .split(',')
                .map(str::trim)
                .filter(|member| !member.is_empty())
                .map(Name::of)
                .collect();
            if members.is_empty() {
                return Err(help_failure(
                    number,
                    &Message::of("group row without sub-lints"),
                ));
            }
            universe.groups.extend([(Name::of(name), members)]);
        }
        HelpSection::Outside => {}
    }
    Ok(())
}

/// `cargo clippy -p <package> --lib` or `--bin <name>`: one target, so the
/// tables print once and build scripts are not compiled.
fn target_selector(root: &Root, member: &Member) -> Result<Args, Failure> {
    let targets = workspace::targets(root, member)?;
    let base = Args::of(&["clippy", "-p", member.name.as_str()]);
    if targets.iter().any(|target| target.kind == TargetKind::Lib) {
        return Ok(base.with("--lib"));
    }
    for target in &targets {
        if target.kind == TargetKind::Bin {
            let file = target.root.file_name();
            let name = if file.as_str() == "main.rs" {
                let parent = target.root.parent();
                if parent.file_name().as_str() == "src" {
                    member.name.clone()
                } else {
                    parent.file_name()
                }
            } else {
                Name::of(file.as_str().trim_end_matches(".rs"))
            };
            return Ok(base.with("--bin").with(name.as_str()));
        }
    }
    Err(Failure::usage(&Message::of(
        "the first workspace member has no library or binary target to ask clippy about",
    )))
}

/// Asks the pinned clippy for its lint tables and parses them.
pub(crate) fn load_universe(root: &Root, ws: &Workspace) -> Result<Universe, Failure> {
    let version = run_cargo(root, &Args::of(&["clippy", "--version"]))?;
    let version_text = version.stdout.to_text();
    let found = version_text.as_str().split_whitespace().nth(1);
    if found != Some(knobs::CLIPPY_VERSION) {
        return Err(Failure::usage(&Message::from_string(format!(
            "unknown clippy version `{}`; this tool knows {}",
            found.into_iter().collect::<String>(),
            knobs::CLIPPY_VERSION
        ))));
    }
    // `cargo clippy -- -W help` runs on the first workspace member (sorted by
    // directory; the root package if any).
    let Some(first) = ws.members.first() else {
        return Err(Failure::usage(&Message::of("workspace has no members")));
    };
    let selector = target_selector(root, first)?;
    let help = run_cargo(root, &selector.with("--").with("-W").with("help"))?;
    let stdout = help.stdout.to_text();
    let text = if stdout.as_str().contains("Lint checks provided by rustc") {
        Source::from_string(stdout.as_str().to_owned())
    } else {
        help.stderr
    };
    parse_help(&text)
}

/// Reads the level and priority from a lint table value, or `None` if malformed.
#[must_use]
fn level_and_priority(value: &Value) -> Option<(Level, Priority)> {
    match value {
        Value::Str(text) => Level::parse(text).map(|level| (level, Priority::new(0))),
        Value::Table(entries) => {
            let level = entries
                .iter()
                .find(|entry| entry.single_key().map(Name::as_str) == Some("level"))
                .and_then(|entry| entry.value.as_text())
                .and_then(Level::parse)?;
            let priority = entries
                .iter()
                .find(|entry| entry.single_key().map(Name::as_str) == Some("priority"))
                .and_then(|entry| match &entry.value {
                    Value::Int(number) => Some(Priority::new(*number)),
                    Value::Str(_)
                    | Value::Bool(_)
                    | Value::Raw(_)
                    | Value::Array(_)
                    | Value::Table(_) => None,
                });
            Some((
                level,
                match priority {
                    Some(found) => found,
                    None => Priority::new(0),
                },
            ))
        }
        Value::Int(_) | Value::Bool(_) | Value::Raw(_) | Value::Array(_) => None,
    }
}

/// Reads `[workspace.lints.rust]` and `[workspace.lints.clippy]`.
pub(crate) fn root_lint_settings(doc: &Doc) -> Vec<LintSetting> {
    let mut out = Vec::new();
    for section in &doc.sections {
        let header: Vec<&str> = section.header.iter().map(Name::as_str).collect();
        match (section.kind, header.as_slice()) {
            (SectionKind::Table, ["workspace", "lints", tool]) => {
                push_settings(section, &Name::of(tool), &mut out);
            }
            (SectionKind::Table, ["workspace", "lints"]) => push_dotted(section, &mut out),
            (SectionKind::Root | SectionKind::Table | SectionKind::ArrayOfTables, _) => {}
        }
    }
    out
}

/// Settings written as `rust.name = ".."` under a bare `[workspace.lints]`.
fn push_dotted(section: &Section, out: &mut Vec<LintSetting>) {
    for entry in &section.entries {
        let Some((tool, name)) = entry.key.pair() else {
            continue;
        };
        if let Some((level, priority)) = level_and_priority(&entry.value) {
            out.push(LintSetting {
                level,
                line: entry.line,
                name: name.clone(),
                priority,
                tool: tool.clone(),
            });
        }
    }
}

/// Appends one setting per entry of the `[workspace.lints.<tool>]` section.
fn push_settings(section: &Section, tool: &Name, out: &mut Vec<LintSetting>) {
    for entry in &section.entries {
        let Some(name) = entry.single_key() else {
            continue;
        };
        if let Some((level, priority)) = level_and_priority(&entry.value) {
            out.push(LintSetting {
                level,
                line: entry.line,
                name: name.clone(),
                priority,
                tool: tool.clone(),
            });
        }
    }
}

/// Adds the leaf lints of a group to `out`, following nested groups to a fixed depth.
fn expand_group(universe: &Universe, group: &Name, out: &mut BTreeSet<Name>, depth: Count) {
    if depth.get() > 8 {
        return;
    }
    let Some(members) = universe.groups.get(group) else {
        return;
    };
    let Ok(next_depth) = depth.next() else {
        return;
    };
    for member in members {
        if universe.groups.contains_key(member) {
            expand_group(universe, member, out, next_depth);
        } else {
            out.extend([member.clone()]);
        }
    }
}

/// Builds a `lint-coverage` finding at the given line of `path`.
fn finding_at(path: &RelPath, line: Line, message: &Message) -> Finding {
    Finding {
        col: Col::new(1),
        item: Text::from_string(String::new()),
        line,
        message: message.clone(),
        path: path.clone(),
        rule: Name::of(rules::LINT_COVERAGE.name()),
    }
}

/// Lets `entry` decide `lint` when it outranks the current choice. Cargo orders
/// entries by priority (lower first, a later entry replaces an earlier one); at
/// equal priority a lint named by itself outranks a group. Two entries of equal rank that disagree leave the
/// weaker level, the stricter reading of an order cargo leaves to the lint name.
fn offer(winners: &mut BTreeMap<Name, Winner>, lint: Name, entry: Winner) {
    let rank = (entry.priority, entry.specificity);
    let replace = match winners.get(&lint) {
        None => true,
        Some(current) => {
            let held = (current.priority, current.specificity);
            rank > held || (rank == held && entry.level.strength() < current.level.strength())
        }
    };
    if replace {
        winners.extend([(lint, entry)]);
    }
}

/// Is `lint` a group of `universe`, or one of the clippy group names?
fn is_group(universe: &Universe, setting: &LintSetting) -> Answer {
    (universe.groups.contains_key(&setting.full_name())
        || (setting.is_clippy() == Answer::Yes && CLIPPY_GROUPS.contains(&setting.name.as_str())))
    .answer()
}

/// Offers one lint table entry to the winners, naming its problems in `findings`.
fn apply_setting(
    universe: &Universe,
    setting: &LintSetting,
    winners: &mut BTreeMap<Name, Winner>,
    findings: &mut Vec<Finding>,
) {
    let full = setting.full_name();
    let group = is_group(universe, setting);
    if setting.is_clippy() == Answer::Yes && setting.name.as_str() == "restriction" {
        findings.push(finding_at(
            &RelPath::of("Cargo.toml"),
            setting.line,
            &Message::of("never set the clippy `restriction` group wholesale; name each lint"),
        ));
        return;
    }
    let specificity = match group {
        Answer::Yes => Specificity::Group,
        Answer::No => Specificity::Lint,
    };
    let entry = Winner {
        level: setting.level,
        priority: setting.priority,
        specificity,
    };
    let strong = setting.level.is_strong() == Answer::Yes;
    if group == Answer::Yes {
        if strong && setting.priority.get() >= 0 {
            findings.push(finding_at(
                &RelPath::of("Cargo.toml"),
                setting.line,
                &Message::from_string(format!(
                    "lint group `{}` needs a negative priority",
                    full.as_str()
                )),
            ));
        }
        let mut members = BTreeSet::new();
        expand_group(universe, &full, &mut members, Count::new(0));
        for member in members {
            offer(winners, member, entry);
        }
    }
    let known = match setting.tool.as_str() {
        "rust" => universe.rust_lints.contains(&full),
        "clippy" => universe.clippy_lints.contains(&full),
        _ => true,
    };
    if group == Answer::No && strong && !known {
        findings.push(finding_at(
            &RelPath::of("Cargo.toml"),
            setting.line,
            &Message::from_string(format!(
                "unknown lint `{}` in the root lint table",
                full.as_str()
            )),
        ));
    }
    offer(winners, full, entry);
}

/// Lints the root tables end up denying or forbidding, groups expanded and
/// overrides applied by cargo's precedence (priority, then a lint beating a group).
fn denied_set(universe: &Universe, settings: &[LintSetting]) -> (BTreeSet<Name>, Vec<Finding>) {
    let mut findings = Vec::new();
    let mut winners: BTreeMap<Name, Winner> = BTreeMap::new();
    for setting in settings {
        apply_setting(universe, setting, &mut winners, &mut findings);
    }
    let denied = winners
        .into_iter()
        .filter(|(_, winner)| winner.level.is_strong() == Answer::Yes)
        .map(|(lint, _)| lint)
        .collect();
    (denied, findings)
}

/// Every lint that must be set: all rustc lints and every clippy group member.
fn required_set(universe: &Universe) -> (BTreeSet<Name>, BTreeSet<Name>) {
    let mut required: BTreeSet<Name> = universe.rust_lints.clone();
    required.extend(universe.clippy_lints.iter().cloned());
    let mut restriction: BTreeSet<Name> = BTreeSet::new();
    for group in CLIPPY_GROUPS {
        let mut members = BTreeSet::new();
        let key = Name::from_string(format!("clippy::{group}"));
        expand_group(universe, &key, &mut members, Count::new(0));
        if *group == "restriction" {
            restriction.extend(members.iter().cloned());
        }
        required.extend(members);
    }
    (required, restriction)
}

/// Builds a finding located at the ledger row.
fn ledger_finding(row: &OmissionRow, message: &Message) -> Finding {
    finding_at(&RelPath::of(LEDGER_FILE), row.line, message)
}

/// Findings and notes for one omission row whose lint really is uncovered.
fn check_omission_row(row: &OmissionRow, denied: &BTreeSet<Name>) -> RowReport {
    let mut out = RowReport {
        findings: Vec::new(),
        notes: Vec::new(),
    };
    match row.kind() {
        Omission::Preference => out.notes.push(Finding {
            col: Col::new(1),
            item: Text::from_string(String::new()),
            line: row.line,
            message: Message::from_string(format!(
                "`{}` is omitted as a preference, not a contradiction",
                row.lint.as_str()
            )),
            path: RelPath::of(LEDGER_FILE),
            rule: Name::of(rules::PREFERENCE_OMISSION.name()),
        }),
        Omission::Unstable => {}
        Omission::Contradicts(other) => {
            if other.is_empty() || !denied.contains(&kebab(&Name::of(other))) {
                out.findings.push(ledger_finding(
                    row,
                    &Message::from_string(format!(
                        "omission of `{}` must contradict a lint set to deny, not `{other}`",
                        row.lint.as_str()
                    )),
                ));
            }
        }
    }
    if ledger::is_blank(&row.reason) == Answer::Yes {
        out.findings.push(ledger_finding(
            row,
            &Message::from_string(format!(
                "omission of `{}` has an empty Reason",
                row.lint.as_str()
            )),
        ));
    }
    out
}

/// Asks the pinned toolchain about every lint the ledger marks `unstable`:
/// it must still say that the lint is unstable. The question is put to
/// `rustc` on an empty crate, so it does not depend on the project compiling.
/// The question goes to `rustc` and not to cargo: after any successful build
/// `cargo clippy -p x -- -W lint` finds the crate fresh and prints nothing, and a
/// crate that does not compile hides the diagnostics. A lint that stabilises
/// turns its row into a `lint-coverage` finding.
fn unstable_findings(root: &Root, rows: &[&OmissionRow]) -> Result<Vec<Finding>, Failure> {
    if rows.is_empty() {
        return Ok(Vec::new());
    }
    let mut args = Args::of(&["--crate-type=lib", "--emit=llvm-ir=-", "-"]);
    for row in rows {
        args = args.with("-W").with(row.lint.as_str());
    }
    // The exit status says nothing here; the report on stderr is what matters.
    let captured = process::rustc(root.path(), &args)?;
    let report = captured.stderr.as_str().to_owned();
    Ok(rows
        .iter()
        .filter(|row| {
            let wanted = format!("the `{}` lint is unstable", row.lint.as_str());
            !report.contains(&wanted)
        })
        .map(|row| {
            ledger_finding(
                row,
                &Message::from_string(format!(
                    "`{}` is marked `{UNSTABLE_MARKER}`, but the pinned toolchain does not report it as unstable",
                    row.lint.as_str()
                )),
            )
        })
        .collect())
}

/// Checks the root lint tables, the omission ledger and `clippy.toml`.
pub(crate) fn coverage(
    root: &Root,
    ws: &Workspace,
    universe: &Universe,
    ledger: &Ledger,
) -> Result<Coverage, Failure> {
    let settings = root_lint_settings(&ws.root_manifest);
    let (mut denied, mut findings) = denied_set(universe, &settings);
    let (required, restriction) = required_set(universe);
    // A restriction lint counts only when named; a group entry never covers it.
    let named: BTreeSet<Name> = settings
        .iter()
        .filter(|setting| setting.level.is_strong() == Answer::Yes)
        .map(LintSetting::full_name)
        .collect();
    denied.retain(|lint| !restriction.contains(lint) || named.contains(lint));
    let uncovered: BTreeSet<Name> = required.difference(&denied).cloned().collect();
    let mut notes = Vec::new();
    let mut covered_rows: BTreeSet<Name> = BTreeSet::new();
    let mut unstable_rows: Vec<&OmissionRow> = Vec::new();
    for row in &ledger.omissions {
        let lint = kebab(&Name::of(row.lint.as_str()));
        covered_rows.extend([lint.clone()]);
        if !uncovered.contains(&lint) {
            findings.push(ledger_finding(
                row,
                &Message::from_string(format!(
                    "omission row for `{}`, which is set or unknown",
                    row.lint.as_str()
                )),
            ));
            continue;
        }
        match row.kind() {
            Omission::Unstable => unstable_rows.push(row),
            Omission::Preference | Omission::Contradicts(_) => {}
        }
        let report = check_omission_row(row, &denied);
        findings.extend(report.findings);
        notes.extend(report.notes);
    }
    for lint in &uncovered {
        if !covered_rows.contains(lint) {
            findings.push(Finding {
                item: Text::of(lint.as_str()),
                ..finding_at(
                    &RelPath::of("Cargo.toml"),
                    Line::new(1),
                    &Message::from_string(format!(
                        "lint `{}` is not set to deny or forbid and has no omission-ledger row",
                        lint.as_str()
                    )),
                )
            });
        }
    }
    findings.extend(unstable_findings(root, &unstable_rows)?);
    findings.extend(check_clippy_toml(root)?);
    Ok(Coverage {
        denied,
        findings,
        notes,
    })
}

/// Checks that every clippy knob in `clippy.toml` is at its strictest setting.
fn check_clippy_toml(root: &Root) -> Result<Vec<Finding>, Failure> {
    let path = RelPath::of("clippy.toml");
    let doc = if root.is_file(&path) == Answer::Yes {
        Some(toml::parse(&path, &root.read(&path)?)?)
    } else {
        None
    };
    let mut out = Vec::new();
    for knob in knobs::KNOBS {
        let value = doc.as_ref().and_then(|found| {
            found
                .sections
                .iter()
                .filter(|section| section.kind == SectionKind::Root)
                .flat_map(|section| section.entries.iter())
                .find(|entry| entry.single_key().map(Name::as_str) == Some(knob.name.text()))
        });
        let ok = match value {
            Some(entry) => knobs::is_strictest(knob, &entry.value) == Answer::Yes,
            None => knob.default == knob.strictest,
        };
        if !ok {
            let line = match value {
                Some(entry) => entry.line,
                None => Line::new(1),
            };
            out.push(Finding {
                item: Text::of(knob.name.text()),
                ..finding_at(
                    &RelPath::of("clippy.toml"),
                    line,
                    &Message::from_string(format!(
                        "clippy.toml knob `{}` must be at its strictest setting",
                        knob.name.text()
                    )),
                )
            });
        }
    }
    Ok(out)
}

/// The channel a toolchain file names. `rust-toolchain.toml` is TOML. The legacy
/// `rust-toolchain` file is TOML only when it has a table or several lines;
/// otherwise its whole content is the bare channel (`1.98.1`), as rustup reads it.
fn toolchain_channel(path: &RelPath, source: &Source) -> Result<Option<Text>, Failure> {
    let bare = path.as_str() == "rust-toolchain" && source.as_str().trim().lines().count() <= 1;
    if bare {
        let channel = source.as_str().trim();
        return Ok(if channel.is_empty() || channel.contains('=') {
            None
        } else {
            Some(Text::of(channel))
        });
    }
    let doc = toml::parse(path, source)?;
    Ok(doc
        .sections
        .iter()
        .flat_map(|section| section.entries.iter())
        .find(|entry| entry.single_key().map(Name::as_str) == Some("channel"))
        .and_then(|entry| entry.value.as_text())
        .cloned())
}

/// Is the toolchain pinned to an exact version?
pub(crate) fn toolchain_findings(root: &Root) -> Result<Vec<Finding>, Failure> {
    for name in ["rust-toolchain.toml", "rust-toolchain"] {
        let path = RelPath::of(name);
        if root.is_file(&path) == Answer::No {
            continue;
        }
        let channel = toolchain_channel(&path, &root.read(&path)?)?;
        let pinned = channel.is_some_and(|text| {
            let parts: Vec<&str> = text.as_str().split('.').collect();
            parts.len() == 3
                && parts
                    .iter()
                    .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit()))
        });
        return Ok(if pinned {
            Vec::new()
        } else {
            vec![finding_at(
                &RelPath::of(name),
                Line::new(1),
                &Message::of(
                    "rust-toolchain must pin an exact toolchain such as channel = \"1.98.1\"",
                ),
            )]
        });
    }
    Ok(vec![finding_at(
        &RelPath::of("rust-toolchain.toml"),
        Line::new(1),
        &Message::of("missing rust-toolchain.toml; lint coverage needs a pinned toolchain"),
    )])
}
