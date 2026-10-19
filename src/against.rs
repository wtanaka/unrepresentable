//! `check --against`: classifies every changed path (protected, judge, code) and
//! every config, ledger and lint-table change as tightening or loosening.
//!
//! Judge paths:
//! - `.unrepresentable/**` is one built-in judge glob. The detailed comparisons
//!   dispatch on the three exact paths (config, ledger, canary exemptions); any
//!   other modified file in the directory is `JUDGE_MODIFIED`, and an added or
//!   deleted one is `JUDGE_CHANGED`.
//! - `**/Cargo.toml` is a judge glob, so member manifests are judge paths: a
//!   modified one is `MEMBER_MANIFEST_CHANGED`, an added or deleted one is
//!   `JUDGE_CHANGED`. The root manifest keeps its finer lint-table comparison.
//! - `deny.toml` and `.deny.toml` (cargo-deny's licence, ban, source and
//!   advisory policy, at any depth) are judge globs: a modified one is
//!   `DENY_CONFIG_CHANGED`, an added or deleted one is `JUDGE_CHANGED`. Any
//!   change to them needs a human, because weakening the policy is not
//!   distinguishable from tightening it without a TOML-level comparison.
//!
//! Which paths count: a path is a judge or code path if the config at the base
//! of the range or the config of the change says so (the union), because a
//! change that edits the globs must not escape the check it edits. A path is
//! free as an append-only addition only if the config at the base says so; the
//! change's own `append-only` globs never exempt the files added beside them.
//!
//! Ledger prose: editing a `Reason` is an approval (`LEDGER_REASON_CHANGED`,
//! `OMISSION_REASON_CHANGED`, `CANARY_EXEMPTION_REASON_CHANGED`). Rows are
//! matched as a multiset, by key with identical prose first, then by key alone.
//! Removing a ledger row or a baseline entry is never an approval reason, since it
//! tightens.
use alloc::collections::{BTreeMap, BTreeSet};

use crate::baseline::{crate_of, Group, MemberDir};

use crate::config::{self, BaselineMode, Config, Role, CONFIG_FILE, LEDGER_FILE};

use crate::failure::Failure;

use crate::findings::Finding;

use crate::git::{self, Base, Change, Status};

use crate::guards;

use crate::glob::{Brackets, Glob};

use crate::io::Root;

use crate::knobs::{self, Transition};

use crate::ledger::{self, Ledger, CANARY_EXEMPTIONS_FILE};

use crate::lintcov::{self, LintSetting};

use crate::rules;

use crate::toml::{self, Doc, SectionKind, Value};

use crate::types::{Answer, Col, Count, Holds as _, Line, Message, Name, RelPath, Source, Text};

use crate::util::remove_index;

/// Globs of paths whose any change always needs human approval: the CI script,
/// the workflows that run the tool and the review checks, and CODEOWNERS files in
/// every place GitHub reads them (the root, `.github/`, `docs/`; any directory
/// is covered). The two workflow paths are defaults for a consumer repository
/// and for this one; a project that names its CI workflow differently lists it
/// under `judge`, where it needs approval as a judge path.
const PROTECTED: &[&str] = &[
    "**/CODEOWNERS",
    ".github/workflows/policy.yml",
    ".github/workflows/unrepresentable.yml",
    "ci/unrepresentable.sh",
];

/// Built-in globs of judge paths: files that define or weaken the checks themselves.
const ALWAYS_JUDGE: &[&str] = &[
    ".unrepresentable/**",
    "**/clippy.toml",
    "**/.clippy.toml",
    "clippy.toml",
    ".clippy.toml",
    "**/deny.toml",
    "**/.deny.toml",
    "deny.toml",
    ".deny.toml",
    "rust-toolchain",
    "rust-toolchain.toml",
    ".cargo/**",
    "**/.cargo/**",
    "Cargo.toml",
    "**/Cargo.toml",
    "Cargo.lock",
    ".gitignore",
    "**/.gitignore",
    ".github/**",
    "ci/unrepresentable.sh",
    "**/build.rs",
    "**/rustfmt.toml",
    "**/.rustfmt.toml",
    "rustfmt.toml",
    ".rustfmt.toml",
];

/// The result of `check --against`: approval reasons and pairing findings.
pub(crate) struct Outcome {
    /// Approval reasons a human must sign off on, sorted and deduplicated.
    pub approvals: Vec<Text>,
    /// Findings raised by the change itself, such as mixing judge and code paths.
    pub findings: Vec<Finding>,
}

/// Everything `classify_judge` needs besides the change itself.
struct Context<'ctx> {
    /// The resolved base of the change range that old file versions are read from.
    base: &'ctx Base,
    /// The config as it was at the base; empty when the base has none.
    before: &'ctx Config,
    /// The config of the change under review.
    config: &'ctx Config,
    /// Workspace member directories, used to attribute baseline entries to crates.
    members: &'ctx [MemberDir],
    /// The workspace root where the new file versions are read.
    root: &'ctx Root,
}

/// Which built-in list of patterns to compile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Builtin {
    /// [`ALWAYS_JUDGE`].
    Judge,
    /// [`PROTECTED`].
    Protected,
}

/// The globs one side of the change declares.
struct Declared {
    /// The `append-only` globs.
    append_only: Vec<Glob>,
    /// Directory globs of the canary directories.
    canary: Vec<Glob>,
    /// The `code` globs.
    code: Vec<Glob>,
    /// The `judge` globs.
    judge: Vec<Glob>,
}

impl Declared {
    /// What `config` declares.
    fn of(config: &Config) -> Result<Self, Failure> {
        Ok(Self {
            append_only: config.append_only.clone(),
            canary: canary_dir_globs(config)?,
            code: config.code.clone(),
            judge: config.judge.clone(),
        })
    }
}

/// How paths are classified for one run: the built-in lists and what the config
/// at the base and the config of the change declare.
struct Scope {
    /// The built-in judge globs.
    always_judge: Vec<Glob>,
    /// What the config of the change declares.
    now: Declared,
    /// The built-in protected globs.
    protected: Vec<Glob>,
    /// What the config at the base declares.
    then: Declared,
}

impl Scope {
    /// Does an added `path` need no approval because the config at the base makes
    /// it append-only?
    fn is_append_only(&self, path: &RelPath) -> Answer {
        (matches_any(&self.then.append_only, path) == Answer::Yes
            || matches_any(&self.then.canary, path) == Answer::Yes)
            .answer()
    }

    /// Is `path` a code path for either config?
    fn is_code(&self, path: &RelPath) -> Answer {
        (matches_any(&self.now.code, path) == Answer::Yes
            || matches_any(&self.then.code, path) == Answer::Yes)
            .answer()
    }

    /// Is `path` a judge path: built in, or declared by either config?
    fn is_judge(&self, path: &RelPath) -> Answer {
        [
            &self.always_judge,
            &self.now.judge,
            &self.then.judge,
            &self.now.canary,
            &self.then.canary,
        ]
        .iter()
        .any(|globs| matches_any(globs, path) == Answer::Yes)
        .answer()
    }

    /// Is `path` protected, so that any change needs approval?
    fn is_protected(&self, path: &RelPath) -> Answer {
        matches_any(&self.protected, path)
    }

    /// The classification of the change described by `context`.
    fn of(context: &Context<'_>) -> Result<Self, Failure> {
        Ok(Self {
            always_judge: builtin(Builtin::Judge)?,
            now: Declared::of(context.config)?,
            protected: builtin(Builtin::Protected)?,
            then: Declared::of(context.before)?,
        })
    }
}

// ---- config ---------------------------------------------------------------

/// The comparable facts of one config: what restricts, what excepts and what is counted.
struct Facts {
    /// Number of baseline entries per (crate, rule) group.
    baseline: BTreeMap<Group, Count>,
    /// Facts that loosen checking when added, such as an exception or an allowance.
    exceptions: BTreeSet<Text>,
    /// Facts that tighten checking, so removing one loosens the config.
    restrictions: BTreeSet<Text>,
    /// Single-valued settings, where any change needs approval.
    scalars: BTreeMap<Text, Text>,
}

impl Facts {
    /// Records `text` as an exception fact.
    fn except(&mut self, text: Text) {
        self.exceptions.extend([text]);
    }

    /// Records `text` as a restriction fact.
    fn restrict(&mut self, text: Text) {
        self.restrictions.extend([text]);
    }
}

/// A ledger row reduced to what `check --against` compares: its identity and its prose.
struct KeyedRow {
    /// The row identity used to match it between old and new versions.
    key: Text,
    /// The row's prose justification.
    reason: Text,
}

/// The same ledger table before and after the change.
struct Tables<'rows> {
    /// Rows of the table after the change.
    new: &'rows [KeyedRow],
    /// Rows of the table before the change.
    old: &'rows [KeyedRow],
}

/// Which ledger table is being compared.
#[derive(Clone, Copy)]
enum Table {
    /// The canary exemptions table.
    Exemptions,
    /// The expect ledger table.
    Expects,
    /// The lint omissions table.
    Omissions,
}

impl Table {
    /// The approval tag for a new row in this table.
    fn added(self) -> Text {
        Text::of(match self {
            Self::Expects => "LEDGER_ROW_ADDED",
            Self::Omissions => "OMISSION_ADDED",
            Self::Exemptions => "CANARY_EXEMPTION_ADDED",
        })
    }

    /// The approval tag for an edited reason in this table.
    fn reason_changed(self) -> Text {
        Text::of(match self {
            Self::Expects => "LEDGER_REASON_CHANGED",
            Self::Omissions => "OMISSION_REASON_CHANGED",
            Self::Exemptions => "CANARY_EXEMPTION_REASON_CHANGED",
        })
    }
}

/// The same document before and after the change.
struct DocPair<'doc> {
    /// The parsed document after the change.
    new: &'doc Doc,
    /// The parsed document before the change.
    old: &'doc Doc,
}

/// Does any glob in `globs` match `path`?
fn matches_any(globs: &[Glob], path: &RelPath) -> Answer {
    globs
        .iter()
        .any(|glob| glob.matches(path) == Answer::Yes)
        .answer()
}

/// Compiles built-in or derived pattern texts as globs.
fn compiled(texts: &[Text]) -> Result<Vec<Glob>, Failure> {
    texts
        .iter()
        .map(|text| {
            Glob::parse(text, Brackets::Literal).map_err(|message| Failure::usage(&message))
        })
        .collect()
}

/// Compiles a built-in list of pattern strings.
fn builtin(which: Builtin) -> Result<Vec<Glob>, Failure> {
    let patterns = match which {
        Builtin::Judge => ALWAYS_JUDGE,
        Builtin::Protected => PROTECTED,
    };
    compiled(
        &patterns
            .iter()
            .map(|pattern| Text::of(pattern))
            .collect::<Vec<Text>>(),
    )
}

/// Directory globs for the configured rule and clippy canary directories.
fn canary_dir_globs(config: &Config) -> Result<Vec<Glob>, Failure> {
    compiled(
        &config
            .canary_dirs
            .iter()
            .map(RelPath::as_str)
            .chain(
                config
                    .clippy_canaries
                    .iter()
                    .map(|clippy| clippy.dir.as_str()),
            )
            .map(|dir| Text::from_string(format!("{}/", dir.trim_end_matches('/'))))
            .collect::<Vec<Text>>(),
    )
}

/// The config as it was at the base of the range: empty when the base has none,
/// and read with `code` optional, since a base from before `code` was required
/// lacks it.
pub(crate) fn base_config(root: &Root, base: &Base) -> Result<Config, Failure> {
    let path = RelPath::of(CONFIG_FILE);
    match git::exists_at(root, base, &path) {
        Answer::Yes => config::parse_base(&path, &git::show(root, base, &path)?),
        Answer::No => Ok(Config::empty()),
    }
}

/// Classifies each change as protected, judge or code and gathers approvals.
///
/// A path is judge or code when the base config or the change's config says so,
/// and an added judge path is free only when the base config makes it
/// append-only (see the module comment).
pub(crate) fn evaluate(
    root: &Root,
    config: &Config,
    base: &Base,
    changes: &[Change],
    members: &[MemberDir],
) -> Result<Outcome, Failure> {
    let before = base_config(root, base)?;
    let context = Context {
        base,
        before: &before,
        config,
        members,
        root,
    };
    let scope = Scope::of(&context)?;
    let mut approvals: Vec<Text> = Vec::new();
    let mut judge_changed = false;
    let mut code_changed = false;
    for change in changes {
        let path = &change.path;
        if scope.is_protected(path) == Answer::Yes {
            approvals.push(Text::from_string(format!(
                "PROTECTED_CHANGED {} ({})",
                path.as_str(),
                status_word(change.status).as_str()
            )));
            judge_changed = true;
            continue;
        }
        if scope.is_code(path) == Answer::Yes {
            code_changed = true;
        }
        if scope.is_judge(path) == Answer::No {
            continue;
        }
        let added = match change.status {
            Status::Added => true,
            Status::Modified | Status::Deleted => false,
        };
        if added && scope.is_append_only(path) == Answer::Yes {
            continue;
        }
        judge_changed = true;
        classify_judge(&context, change, &mut approvals)?;
    }
    approvals.sort();
    approvals.dedup();
    let mut findings = Vec::new();
    if judge_changed && code_changed {
        findings.push(pairing_finding());
    }
    Ok(Outcome {
        approvals,
        findings,
    })
}

/// The finding for a change that mixes judge paths with code paths.
fn pairing_finding() -> Finding {
    Finding {
        col: Col::new(1),
        item: Text::from_string(String::new()),
        line: Line::new(1),
        message: Message::of(
            "one change touches judge paths and code paths; split it so a human can approve the judge change alone",
        ),
        path: RelPath::of(CONFIG_FILE),
        rule: Name::of(rules::PAIRING.name()),
    }
}

/// The lowercase word for a change status.
fn status_word(status: Status) -> Text {
    match status {
        Status::Added => Text::of("added"),
        Status::Modified => Text::of("modified"),
        Status::Deleted => Text::of("deleted"),
    }
}

/// Adds approvals for one changed judge file, comparing contents when modified.
fn classify_judge(
    context: &Context<'_>,
    change: &Change,
    approvals: &mut Vec<Text>,
) -> Result<(), Failure> {
    let name = change.path.as_str();
    match change.status {
        Status::Added | Status::Deleted => {
            approvals.push(Text::from_string(format!(
                "JUDGE_CHANGED {name} ({})",
                status_word(change.status).as_str()
            )));
            return Ok(());
        }
        Status::Modified => {}
    }
    match name {
        CONFIG_FILE => compare_config(context, approvals)?,
        LEDGER_FILE => compare_ledger(context, approvals)?,
        CANARY_EXEMPTIONS_FILE => compare_canary_exemptions(context, approvals)?,
        "Cargo.toml" => compare_root_manifest(context, approvals)?,
        "Cargo.lock" => approvals.push(Text::of("DEPENDENCY_CHANGED Cargo.lock")),
        other if other.ends_with("/Cargo.toml") => {
            approvals.push(Text::from_string(format!(
                "MEMBER_MANIFEST_CHANGED {other}"
            )));
        }
        "clippy.toml" | ".clippy.toml" => compare_clippy_toml(context, &change.path, approvals)?,
        "rust-toolchain" | "rust-toolchain.toml" => {
            approvals.push(Text::from_string(format!("TOOLCHAIN_CHANGED {name}")));
        }
        other if is_deny_config(&change.path) == Answer::Yes => {
            approvals.push(Text::from_string(format!("DENY_CONFIG_CHANGED {other}")));
        }
        other if other.starts_with(".github/") => {
            approvals.push(Text::from_string(format!("WORKFLOW_CHANGED {other}")));
        }
        other if other.contains(".cargo/") => {
            approvals.push(Text::from_string(format!("CARGO_CONFIG_CHANGED {other}")));
        }
        other if other.ends_with(".gitignore") => {
            approvals.push(Text::from_string(format!("GITIGNORE_CHANGED {other}")));
        }
        other => approvals.push(Text::from_string(format!(
            "JUDGE_MODIFIED {other} (modified)"
        ))),
    }
    Ok(())
}

/// Is `path` a cargo-deny configuration file, at any depth?
fn is_deny_config(path: &RelPath) -> Answer {
    matches!(path.file_name().as_str(), "deny.toml" | ".deny.toml").answer()
}

/// The pattern text of each glob.
fn glob_list(globs: &[Glob]) -> Vec<Text> {
    globs.iter().map(|glob| glob.text().clone()).collect()
}

/// Adds the path, attribute and list facts of `config` to `facts`.
fn path_facts(config: &Config, facts: &mut Facts) {
    match config.baseline_mode {
        BaselineMode::Forbidden => facts.restrict(Text::of("baseline-forbidden")),
        BaselineMode::ShrinkOnly => {}
    }
    for dir in &config.canary_dirs {
        // Listing a directory exempts the files added there; unlisting it stops
        // its canaries from running. Both directions are approvals.
        let fact = Text::from_string(format!("canary-dir:{}", dir.as_str()));
        facts.except(fact.clone());
        facts.restrict(fact);
    }
    if let Some(clippy) = &config.clippy_canaries {
        facts.restrict(Text::from_string(format!(
            "clippy-canaries:{}|{}",
            clippy.package.as_str(),
            clippy.dir.as_str()
        )));
    }
    for glob in glob_list(&config.arith_files) {
        facts.except(Text::from_string(format!("arith-file:{}", glob.as_str())));
    }
    for attr in &config.test_attributes {
        facts.except(Text::from_string(format!(
            "test-attribute:{}",
            attr.as_str()
        )));
    }
    for name in &config.silent_fallback_names {
        facts.restrict(Text::from_string(format!(
            "silent-fallback:{}",
            name.as_str()
        )));
    }
    for name in &config.fn_name_suffixes {
        facts.restrict(Text::from_string(format!(
            "fn-name-suffix:{}",
            name.as_str()
        )));
    }
    for name in &config.assertion_macros {
        facts.except(Text::from_string(format!(
            "assertion-macro:{}",
            name.as_str()
        )));
    }
    for glob in glob_list(&config.judge) {
        facts.restrict(Text::from_string(format!("judge:{}", glob.as_str())));
    }
    for glob in glob_list(&config.code) {
        facts.restrict(Text::from_string(format!("code:{}", glob.as_str())));
    }
    for glob in glob_list(&config.append_only) {
        facts.except(Text::from_string(format!("append-only:{}", glob.as_str())));
    }
    for entry in &config.cargo_exceptions {
        facts.except(Text::from_string(format!(
            "cargo-exception:{}",
            entry.as_str()
        )));
    }
}

/// Adds the per-member role, ambient and newtype facts of `config` to `facts`.
fn member_facts(config: &Config, facts: &mut Facts) {
    for member in &config.members {
        match member.role {
            Role::Pure => facts.restrict(Text::from_string(format!(
                "role-pure:{}",
                member.name.as_str()
            ))),
            Role::Shell => facts.except(Text::from_string(format!(
                "role-shell:{}",
                member.name.as_str()
            ))),
        }
        for allow in &member.ambient {
            facts.except(Text::from_string(format!(
                "ambient:{}:{}:{}",
                member.name.as_str(),
                allow.path.as_str(),
                glob_list(&allow.files)
                    .iter()
                    .map(Text::as_str)
                    .collect::<Vec<_>>()
                    .join(",")
            )));
        }
        for newtype in &member.newtypes {
            for method in &newtype.methods {
                facts.except(Text::from_string(format!(
                    "newtype:{}:{}:{}",
                    member.name.as_str(),
                    newtype.type_name.as_str(),
                    method.as_str()
                )));
            }
        }
    }
}

/// Adds the facts of every configured ban to `facts`.
fn ban_facts(config: &Config, facts: &mut Facts) {
    for ban in &config.bans {
        let id = ban.id.as_str();
        facts.restrict(Text::from_string(format!("ban:{id}")));
        for glob in glob_list(&ban.files) {
            facts.restrict(Text::from_string(format!(
                "ban-files:{id}:{}",
                glob.as_str()
            )));
        }
        for entry in &ban.paths {
            facts.restrict(Text::from_string(format!(
                "ban-path:{id}:{}",
                entry.as_str()
            )));
        }
        for entry in &ban.impl_traits {
            facts.restrict(Text::from_string(format!(
                "ban-impl:{id}:{}",
                entry.as_str()
            )));
        }
        for entry in &ban.derives {
            facts.restrict(Text::from_string(format!(
                "ban-derive:{id}:{}",
                entry.as_str()
            )));
        }
        for entry in &ban.macros {
            facts.restrict(Text::from_string(format!(
                "ban-macro:{id}:{}",
                entry.as_str()
            )));
        }
        for entry in &ban.methods {
            facts.restrict(Text::from_string(format!(
                "ban-method:{id}:{}",
                entry.as_str()
            )));
        }
    }
}

/// Adds the macro, dependency and reference rule facts and scalars to `facts`.
fn rule_facts(config: &Config, facts: &mut Facts) {
    for entry in &config.macro_comments {
        facts.restrict(Text::from_string(format!(
            "macro-comment:{}|{}|{}",
            entry.macro_name.as_str(),
            entry.prefix.as_str(),
            entry.within_lines.get()
        )));
    }
    for entry in &config.forbidden_dependencies {
        for (kind, targets) in [
            ("forbidden-dependency", &entry.must_not_reach),
            ("forbidden-dependency-never", &entry.must_never_reach),
        ] {
            for target in targets {
                facts.restrict(Text::from_string(format!(
                    "{kind}:{}->{}",
                    guards::canonical(&entry.crate_name).as_str(),
                    guards::canonical(target).as_str()
                )));
            }
        }
    }
    for entry in &config.forbidden_references {
        for name in &entry.must_not_name {
            for glob in glob_list(&entry.files) {
                facts.restrict(Text::from_string(format!(
                    "forbidden-reference:{}>>{}",
                    glob.as_str(),
                    name.as_str()
                )));
            }
        }
    }
    facts.scalars.extend([(
        Text::of("default-range"),
        match &config.default_range {
            Some(text) => text.clone(),
            None => Text::of(""),
        },
    )]);
}

/// Counts the baseline entries of `config` per (crate, rule) group into `facts`.
fn baseline_facts(
    config: &Config,
    members: &[MemberDir],
    facts: &mut Facts,
) -> Result<(), Failure> {
    for entry in &config.baseline {
        let group = Group {
            member: crate_of(members, &entry.file),
            rule: entry.rule.clone(),
        };
        let current = match facts.baseline.get(&group) {
            Some(existing) => *existing,
            None => Count::new(0),
        };
        facts.baseline.extend([(group, current.next()?)]);
    }
    Ok(())
}

/// Collects every comparable fact of `config`.
fn facts(config: &Config, members: &[MemberDir]) -> Result<Facts, Failure> {
    let mut collected = Facts {
        baseline: BTreeMap::new(),
        exceptions: BTreeSet::new(),
        restrictions: BTreeSet::new(),
        scalars: BTreeMap::new(),
    };
    path_facts(config, &mut collected);
    member_facts(config, &mut collected);
    ban_facts(config, &mut collected);
    rule_facts(config, &mut collected);
    baseline_facts(config, members, &mut collected)?;
    Ok(collected)
}

/// Reads `path` as it was at the base of the range.
fn old_source(context: &Context<'_>, path: &RelPath) -> Result<Source, Failure> {
    git::show(context.root, context.base, path)
}

/// Compares the config before and after, approving loosenings and new exceptions.
fn compare_config(context: &Context<'_>, approvals: &mut Vec<Text>) -> Result<(), Failure> {
    let old = context.before;
    let new = context.config;
    let before = facts(old, context.members)?;
    let after = facts(new, context.members)?;
    for fact in before.restrictions.difference(&after.restrictions) {
        approvals.push(Text::from_string(format!(
            "CONFIG_LOOSENED removed {}",
            fact.as_str()
        )));
    }
    for fact in after.exceptions.difference(&before.exceptions) {
        approvals.push(Text::from_string(format!(
            "EXCEPTION_ADDED {}",
            fact.as_str()
        )));
    }
    for (key, value) in &after.scalars {
        if before.scalars.get(key) != Some(value) {
            approvals.push(Text::from_string(format!(
                "CONFIG_CHANGED {}",
                key.as_str()
            )));
        }
    }
    for (group, count) in &after.baseline {
        let previous = match before.baseline.get(group) {
            Some(existing) => *existing,
            None => Count::new(0),
        };
        if *count > previous {
            approvals.push(Text::from_string(format!(
                "BASELINE_GREW {} in {} ({} -> {})",
                group.rule.as_str(),
                group.member.as_str(),
                previous.get(),
                count.get()
            )));
        }
    }
    match (old.baseline_mode, new.baseline_mode) {
        (BaselineMode::Forbidden, BaselineMode::ShrinkOnly) => {
            approvals.push(Text::of(
                "CONFIG_LOOSENED baseline forbidden -> shrink-only",
            ));
        }
        (BaselineMode::Forbidden, BaselineMode::Forbidden)
        | (BaselineMode::ShrinkOnly, BaselineMode::ShrinkOnly | BaselineMode::Forbidden) => {}
    }
    Ok(())
}

// ---- ledgers --------------------------------------------------------------

/// Compares the expect ledger and lint omissions before and after.
fn compare_ledger(context: &Context<'_>, approvals: &mut Vec<Text>) -> Result<(), Failure> {
    let old = ledger::parse(&old_source(context, &RelPath::of(LEDGER_FILE))?)?;
    let new = ledger::parse(&context.root.read(&RelPath::of(LEDGER_FILE))?)?;
    compare_rows(
        Table::Expects,
        &Tables {
            new: &expect_rows(&new),
            old: &expect_rows(&old),
        },
        approvals,
    );
    compare_rows(
        Table::Omissions,
        &Tables {
            new: &omission_rows(&new),
            old: &omission_rows(&old),
        },
        approvals,
    );
    Ok(())
}

/// The expect rows of `ledger`, keyed by file, item and lint.
fn expect_rows(ledger: &Ledger) -> Vec<KeyedRow> {
    ledger
        .expects
        .iter()
        .map(|row| KeyedRow {
            key: Text::from_string(format!(
                "{} {} {}",
                row.file.as_str(),
                row.item.as_str(),
                row.lint.as_str()
            )),
            reason: row.reason.clone(),
        })
        .collect()
}

/// The omission rows of `ledger`, keyed by lint and contradiction.
fn omission_rows(ledger: &Ledger) -> Vec<KeyedRow> {
    ledger
        .omissions
        .iter()
        .map(|row| KeyedRow {
            key: Text::from_string(format!(
                "{} {}",
                row.lint.as_str(),
                row.contradicts.as_str()
            )),
            reason: row.reason.clone(),
        })
        .collect()
}

/// Multiset comparison of two tables: a new key is an addition, a kept key
/// with different prose is a reason edit, and a removal is silent.
fn compare_rows(table: Table, rows: &Tables<'_>, approvals: &mut Vec<Text>) {
    let mut remaining: Vec<&KeyedRow> = rows.old.iter().collect();
    let mut unmatched: Vec<&KeyedRow> = Vec::new();
    for row in rows.new {
        let exact = remaining
            .iter()
            .position(|candidate| candidate.key == row.key && candidate.reason == row.reason);
        match exact {
            Some(index) => remove_index(&mut remaining, Count::new(index)),
            None => unmatched.push(row),
        }
    }
    for row in unmatched {
        let same_key = remaining
            .iter()
            .position(|candidate| candidate.key == row.key);
        match same_key {
            Some(index) => {
                remove_index(&mut remaining, Count::new(index));
                approvals.push(Text::from_string(format!(
                    "{} {}",
                    table.reason_changed().as_str(),
                    row.key.as_str()
                )));
            }
            None => approvals.push(Text::from_string(format!(
                "{} {}",
                table.added().as_str(),
                row.key.as_str()
            ))),
        }
    }
}

/// Compares the canary exemptions table before and after.
fn compare_canary_exemptions(
    context: &Context<'_>,
    approvals: &mut Vec<Text>,
) -> Result<(), Failure> {
    let heading = Text::of("## Canary exemptions");
    let old = ledger::table_rows(
        &old_source(context, &RelPath::of(CANARY_EXEMPTIONS_FILE))?,
        &heading,
    )?;
    let new = ledger::table_rows(
        &context.root.read(&RelPath::of(CANARY_EXEMPTIONS_FILE))?,
        &heading,
    )?;
    let keyed = |rows: &[ledger::TableRow]| -> Vec<KeyedRow> {
        rows.iter()
            .filter_map(|row| {
                let (lint, why) = (row.cells.first()?, row.cells.get(1)?);
                Some(KeyedRow {
                    key: lint.clone(),
                    reason: why.clone(),
                })
            })
            .collect()
    };
    compare_rows(
        Table::Exemptions,
        &Tables {
            new: &keyed(&new),
            old: &keyed(&old),
        },
        approvals,
    );
    Ok(())
}

// ---- root manifest and lint tables -----------------------------------------

/// Renders a TOML value as stable text for comparison.
fn render_value(value: &Value) -> Text {
    let text = match value {
        Value::Str(text) => format!("{:?}", text.as_str()),
        Value::Int(number) => number.to_string(),
        Value::Bool(flag) => flag.to_string(),
        Value::Raw(text) => text.as_str().to_owned(),
        Value::Array(items) => format!(
            "[{}]",
            items
                .iter()
                .map(|item| render_value(item).as_str().to_owned())
                .collect::<Vec<_>>()
                .join(",")
        ),
        Value::Table(entries) => format!(
            "{{{}}}",
            entries
                .iter()
                .map(|entry| format!(
                    "{}={}",
                    entry.key.dotted().as_str(),
                    render_value(&entry.value).as_str()
                ))
                .collect::<Vec<_>>()
                .join(",")
        ),
    };
    Text::from_string(text)
}

/// Renders each manifest section as text, skipping the workspace lint tables.
fn manifest_without_lints(doc: &Doc) -> Vec<Text> {
    doc.sections
        .iter()
        .filter(|section| {
            let workspace = section.header.first().map(Name::as_str) == Some("workspace");
            let lints = section.header.get(1).map(Name::as_str) == Some("lints");
            !(workspace && lints)
        })
        .map(|section| {
            let header: Vec<&str> = section.header.iter().map(Name::as_str).collect();
            let entries: Vec<String> = section
                .entries
                .iter()
                .map(|entry| {
                    format!(
                        "{}={}",
                        entry.key.dotted().as_str(),
                        render_value(&entry.value).as_str()
                    )
                })
                .collect();
            Text::from_string(format!(
                "{:?}|{}|{}",
                section.kind,
                header.join("."),
                entries.join(";")
            ))
        })
        .collect()
}

/// Compares the root `Cargo.toml`, approving changes outside and inside the lint tables.
fn compare_root_manifest(context: &Context<'_>, approvals: &mut Vec<Text>) -> Result<(), Failure> {
    let path = RelPath::of("Cargo.toml");
    let old = toml::parse(&path, &old_source(context, &path)?)?;
    let new = toml::parse(&path, &context.root.read(&path)?)?;
    if manifest_without_lints(&old) != manifest_without_lints(&new) {
        approvals.push(Text::of(
            "ROOT_MANIFEST_CHANGED Cargo.toml (outside the lint tables)",
        ));
    }
    let tables = DocPair {
        new: &new,
        old: &old,
    };
    compare_lint_tables(&tables, approvals);
    Ok(())
}

/// The qualified name of a lint, with `clippy::` for clippy lints.
fn lint_name(setting: &LintSetting) -> Text {
    if setting.tool.as_str() == "clippy" {
        Text::from_string(format!("clippy::{}", setting.name.as_str()))
    } else {
        Text::of(setting.name.as_str())
    }
}

/// Approves lint removals, lowered levels and priority changes between two manifests.
fn compare_lint_tables(tables: &DocPair<'_>, approvals: &mut Vec<Text>) {
    let before = lintcov::root_lint_settings(tables.old);
    let after = lintcov::root_lint_settings(tables.new);
    let after_map: BTreeMap<(Name, Name), &LintSetting> = after
        .iter()
        .map(|setting| ((setting.tool.clone(), setting.name.clone()), setting))
        .collect();
    for setting in &before {
        let full = lint_name(setting);
        let name = full.as_str();
        let key = (setting.tool.clone(), setting.name.clone());
        let Some(current) = after_map.get(&key) else {
            approvals.push(Text::from_string(format!(
                "LINT_REMOVED {name} (Cargo.toml)"
            )));
            continue;
        };
        if current.level.strength() < setting.level.strength() {
            approvals.push(Text::from_string(format!(
                "LINT_LOWERED {name} {:?}->{:?} (Cargo.toml:{})",
                setting.level,
                current.level,
                current.line.get()
            )));
        }
        if current.priority != setting.priority {
            approvals.push(Text::from_string(format!(
                "LINT_PRIORITY_CHANGED {name} {}->{} (Cargo.toml:{})",
                setting.priority.get(),
                current.priority.get(),
                current.line.get()
            )));
        }
    }
}

// ---- clippy.toml ----------------------------------------------------------

/// The comparable keys of a config document, by key: top-level entries (a dotted
/// key by its dotted name), `[table]` sections by their header, and the elements
/// of `[[array]]` sections gathered into one array per header. Reading only the
/// top-level entries would leave a deleted `[[disallowed-methods]]` block or a
/// deleted table unseen.
fn root_entries(doc: &Doc) -> BTreeMap<Name, Value> {
    let dotted = |names: &[Name]| {
        Name::from_string(names.iter().map(Name::as_str).collect::<Vec<_>>().join("."))
    };
    let mut found: BTreeMap<Name, Value> = BTreeMap::new();
    let mut arrays: BTreeMap<Name, Vec<Value>> = BTreeMap::new();
    for section in &doc.sections {
        match section.kind {
            SectionKind::Root => {
                for entry in &section.entries {
                    found.extend([(Name::of(entry.key.dotted().as_str()), entry.value.clone())]);
                }
            }
            SectionKind::Table => {
                found.extend([(
                    dotted(&section.header),
                    Value::Table(section.entries.clone()),
                )]);
            }
            SectionKind::ArrayOfTables => arrays
                .entry(dotted(&section.header))
                .or_default()
                .push(Value::Table(section.entries.clone())),
        }
    }
    for (key, items) in arrays {
        found.extend([(key, Value::Array(items))]);
    }
    found
}

/// The identity of an array entry: its string, or the `path` of a table.
fn entry_identity(value: &Value) -> Text {
    match value {
        Value::Str(text) => text.clone(),
        Value::Table(entries) => {
            let path = entries
                .iter()
                .find(|entry| entry.single_key().map(Name::as_str) == Some("path"))
                .and_then(|entry| entry.value.as_text());
            match path {
                Some(text) => text.clone(),
                None => render_value(value),
            }
        }
        Value::Int(_) | Value::Bool(_) | Value::Raw(_) | Value::Array(_) => render_value(value),
    }
}

/// The identities of every entry in an optional array value.
fn identities(value: Option<&Value>) -> BTreeSet<Text> {
    match value.and_then(Value::as_array) {
        Some(items) => items.iter().map(entry_identity).collect(),
        None => BTreeSet::new(),
    }
}

/// Renders an optional value, or `None` when it is absent.
#[must_use]
fn rendered(value: Option<&Value>) -> Option<Text> {
    value.map(render_value)
}

/// Compares `clippy.toml` before and after, approving removed bans and loosened knobs.
fn compare_clippy_toml(
    context: &Context<'_>,
    path: &RelPath,
    approvals: &mut Vec<Text>,
) -> Result<(), Failure> {
    let old = toml::parse(path, &old_source(context, path)?)?;
    let new = toml::parse(path, &context.root.read(path)?)?;
    let before = root_entries(&old);
    let after = root_entries(&new);
    let keys: BTreeSet<&Name> = before.keys().chain(after.keys()).collect();
    for key in keys {
        let old_value = before.get(key);
        let new_value = after.get(key);
        if rendered(old_value) == rendered(new_value) {
            continue;
        }
        if key.as_str().starts_with("disallowed-") {
            let old_ids = identities(old_value);
            let new_ids = identities(new_value);
            for removed in old_ids.difference(&new_ids) {
                approvals.push(Text::from_string(format!(
                    "DISALLOWED_REMOVED {} {}",
                    key.as_str(),
                    removed.as_str()
                )));
            }
            continue;
        }
        match knobs::find(key) {
            Some(knob) => {
                let change = Transition {
                    new: new_value,
                    old: old_value,
                };
                if knobs::at_least_as_strict(knob, &change) == Answer::No {
                    approvals.push(Text::from_string(format!(
                        "CLIPPY_KNOB_LOOSENED {}",
                        key.as_str()
                    )));
                }
            }
            None => approvals.push(Text::from_string(format!(
                "CLIPPY_CONFIG_CHANGED {}",
                key.as_str()
            ))),
        }
    }
    Ok(())
}
