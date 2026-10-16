//! Command line: argument parsing, orchestration, exit codes.
use clap::builder::{PathBufValueParser, StringValueParser, TypedValueParser as _};

use clap::error::ErrorKind;

use clap::parser::MatchesError;

use clap::{Arg, ArgAction, ArgMatches, Command as ClapCommand};

use std::ffi::OsString;

use std::path::PathBuf;

use crate::against;

use crate::analyze;

use crate::baseline::{self, MemberDir};

use crate::canaries;

use crate::config::{self, Config, CONFIG_FILE, LEDGER_FILE};

use crate::failure::Failure;

use crate::findings::{self, Finding};

use crate::git;

use crate::guards;

use crate::io::{self, Root};

use crate::ledger::{self, CANARY_EXEMPTIONS_FILE};

use crate::lintcov;

use crate::selftest;

use crate::toml::{self, SectionKind};

use crate::types::{Answer, Count, Message, Name, RelPath, Source, Text};

use crate::workspace::Workspace;

/// The process outcome of a run. There is no variant for exit 2: `main` maps a
/// `Failure` from `run` to it, which keeps "the tool could not decide" apart from
/// "the tool decided and said no".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExitStatus {
    /// No findings, but changes need human approval.
    Approval,
    /// No findings and nothing to approve.
    Clean,
    /// At least one finding was reported.
    Findings,
}

/// Whether a command-line switch was given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    /// The switch was not given.
    Off,
    /// The switch was given.
    On,
}

/// Which change a run examines, as given on the command line. At most one of
/// `--against` and `--range` can be given, so the two are one value.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Change {
    /// `--against REF`: the range is `REF...HEAD`.
    Against(Text),
    /// `--range RANGE`.
    Range(Text),
    /// Neither flag was given.
    Unspecified,
}

/// The subcommand and the arguments only that subcommand takes.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Command {
    /// `canaries`: checks the rule and clippy canaries.
    Canaries,
    /// `check`: all checks, the baseline and the change range.
    Check {
        /// The change to examine.
        change: Change,
        /// Whether to rewrite the baseline first.
        ratchet: Mode,
    },
    /// `init`: writes a config and stub ledgers.
    Init {
        /// The change whose base must not hold a config.
        change: Change,
    },
    /// `self-test`: the built-in canaries.
    SelfTest,
}

/// What the command line asks for.
enum Parsed {
    /// Text for standard output (`--help`, `--version`); the run is complete.
    Display(Message),
    /// A run to perform.
    Run(Options),
}

/// Whether a positional argument must be given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Presence {
    /// A default value is used when it is absent.
    Optional,
    /// Absence is a usage error.
    Required,
}

/// The parsed command line.
struct Options {
    /// The subcommand.
    command: Command,
    /// The workspace root path.
    root: PathBuf,
}

/// Everything one run produces.
pub(crate) struct Report {
    /// Reasons a human must approve the change.
    pub approvals: Vec<Text>,
    /// Findings that fail the run.
    pub findings: Vec<Finding>,
    /// Informational findings that do not fail the run.
    pub notes: Vec<Finding>,
    /// The change range that was checked.
    pub range: Option<Text>,
}

/// Every check's findings, before the baseline.
struct Raw {
    /// Normalised findings that fail the run.
    findings: Vec<Finding>,
    /// Informational findings that do not fail the run.
    notes: Vec<Finding>,
}

/// Where the change range of a run came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Origin {
    /// `--range` or `--against` of `check` on the command line.
    Command,
    /// `default-range` of the config.
    Config,
}

/// The change range of a run and where it came from.
struct Chosen {
    /// Where the range came from.
    origin: Origin,
    /// The range text.
    range: Text,
}

/// The workspace root argument.
fn root_arg(presence: Presence) -> Arg {
    let arg = Arg::new("root")
        .value_name("ROOT")
        .value_parser(PathBufValueParser::new());
    match presence {
        Presence::Optional => arg
            .default_value(".")
            .help("Workspace root (default: the current directory)"),
        Presence::Required => arg.required(true).help("Workspace root"),
    }
}

/// Adds the mutually exclusive `--against` and `--range` to `command`.
fn with_change_args(command: ClapCommand) -> ClapCommand {
    command
        .arg(
            Arg::new("against")
                .long("against")
                .value_name("REF")
                .value_parser(StringValueParser::new().map(Text::from_string))
                .conflicts_with("range")
                .help("Base git ref; the change range is REF...HEAD"),
        )
        .arg(
            Arg::new("range")
                .long("range")
                .value_name("RANGE")
                .value_parser(StringValueParser::new().map(Text::from_string))
                .conflicts_with("against")
                .help("Change range: A..B (base A) or A...B (merge base of A and B)"),
        )
}

/// The `check` subcommand.
fn check_command() -> ClapCommand {
    with_change_args(
        ClapCommand::new("check")
            .about("Run all checks over a change range and apply the baseline")
            .arg(
                Arg::new("ratchet")
                    .long("ratchet")
                    .action(ArgAction::SetTrue)
                    .help("Rewrite the baseline to drop stale entries first"),
            ),
    )
    .arg(root_arg(Presence::Required))
}

/// The `init` subcommand.
fn init_command() -> ClapCommand {
    with_change_args(
        ClapCommand::new("init")
            .about("Write a config with a baseline of current findings and stub ledgers"),
    )
    .arg(root_arg(Presence::Required))
}

/// The `self-test` subcommand.
fn self_test_command() -> ClapCommand {
    ClapCommand::new("self-test")
        .about("Run the canaries built into the binary; needs no git or cargo")
        .arg(root_arg(Presence::Optional))
}

/// The `canaries` subcommand.
fn canaries_command() -> ClapCommand {
    ClapCommand::new("canaries")
        .about("Check the rule canaries and, when configured, the clippy canaries")
        .arg(root_arg(Presence::Required))
}

/// The command line grammar: one subcommand per mode.
fn grammar() -> ClapCommand {
    ClapCommand::new("unrepresentable")
        .version(clap::crate_version!())
        .about(
            "An AST linter that rejects Rust code whose types and signatures allow invalid states",
        )
        .subcommand_required(true)
        .subcommand(check_command())
        .subcommand(init_command())
        .subcommand(self_test_command())
        .subcommand(canaries_command())
}

/// Turns a lookup of an optional string argument into text.
fn text_of(found: Result<Option<&Text>, MatchesError>) -> Result<Option<Text>, Failure> {
    match found {
        Ok(value) => Ok(value.cloned()),
        Err(error) => Err(Failure::internal(&Message::from_string(format!(
            "command-line argument of unexpected type: {error}"
        )))),
    }
}

/// The change selected by `--against` or `--range`.
fn change_of(matches: &ArgMatches) -> Result<Change, Failure> {
    match (
        text_of(matches.try_get_one::<Text>("against"))?,
        text_of(matches.try_get_one::<Text>("range"))?,
    ) {
        (Some(reference), None) => Ok(Change::Against(reference)),
        (None, Some(range)) => Ok(Change::Range(range)),
        (None, None) => Ok(Change::Unspecified),
        (Some(_), Some(_)) => Err(Failure::internal(&Message::of(
            "--against and --range were both accepted",
        ))),
    }
}

/// The workspace root of a subcommand.
fn root_of(matches: &ArgMatches) -> Result<PathBuf, Failure> {
    match matches.try_get_one::<PathBuf>("root") {
        Ok(Some(path)) => Ok(path.clone()),
        Ok(None) => Err(Failure::internal(&Message::of(
            "the root argument is missing",
        ))),
        Err(error) => Err(Failure::internal(&Message::from_string(format!(
            "command-line argument of unexpected type: {error}"
        )))),
    }
}

/// Whether `--ratchet` was given.
fn ratchet_of(matches: &ArgMatches) -> Result<Mode, Failure> {
    match matches.try_get_one::<bool>("ratchet") {
        Ok(Some(true)) => Ok(Mode::On),
        Ok(Some(false) | None) => Ok(Mode::Off),
        Err(error) => Err(Failure::internal(&Message::from_string(format!(
            "command-line argument of unexpected type: {error}"
        )))),
    }
}

/// Parses the command line into options. `--help` and `--version` come back as
/// text to print; every other parse error is a usage failure (exit 2).
fn parse_args(args: Vec<OsString>) -> Result<Parsed, Failure> {
    let mut full = vec![OsString::from("unrepresentable")];
    full.extend(args);
    let matches = match grammar().try_get_matches_from(full) {
        Ok(found) => found,
        Err(error) => {
            let text = Message::from_string(error.to_string());
            return match error.kind() {
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => Ok(Parsed::Display(text)),
                _ => Err(Failure::usage(&Message::from_string(
                    text.as_str().trim_end().to_owned(),
                ))),
            };
        }
    };
    let (command, root) = match matches.subcommand() {
        Some(("check", sub)) => (
            Command::Check {
                change: change_of(sub)?,
                ratchet: ratchet_of(sub)?,
            },
            root_of(sub)?,
        ),
        Some(("init", sub)) => (
            Command::Init {
                change: change_of(sub)?,
            },
            root_of(sub)?,
        ),
        Some(("self-test", sub)) => (Command::SelfTest, root_of(sub)?),
        Some(("canaries", sub)) => (Command::Canaries, root_of(sub)?),
        Some((other, _)) => {
            return Err(Failure::internal(&Message::from_string(format!(
                "subcommand {other} was accepted but is not handled"
            ))));
        }
        None => {
            return Err(Failure::internal(&Message::of(
                "no subcommand was selected",
            )))
        }
    };
    Ok(Parsed::Run(Options { command, root }))
}

/// The directory and name of each workspace member.
fn member_dirs(ws: &Workspace) -> Vec<MemberDir> {
    ws.members
        .iter()
        .map(|member| MemberDir {
            dir: member.dir.clone(),
            name: member.name.clone(),
        })
        .collect()
}

/// Reads and parses the ledger, or fails when it is missing.
fn read_ledger(root: &Root) -> Result<ledger::Ledger, Failure> {
    let path = RelPath::of(LEDGER_FILE);
    if root.is_file(&path) == Answer::No {
        return Err(Failure::usage(&Message::from_string(format!(
            "missing {LEDGER_FILE} (run `unrepresentable init`)"
        ))));
    }
    ledger::parse(&root.read(&path)?)
}

/// Runs every check and returns the normalised findings and notes.
fn collect(root: &Root, config: &Config, ws: &Workspace) -> Result<Raw, Failure> {
    let analysis = analyze::analyze(root, config, ws)?;
    let ledger = read_ledger(root)?;
    let mut all = analysis.findings;
    all.extend(ledger::check_expects(&ledger, &analysis.expects));
    all.extend(guards::manifest_lints(ws));
    let found = guards::all_files(root)?;
    all.extend(guards::stray_lint_config(root, config, &found)?);
    all.extend(guards::forbidden_dependencies(root, config, ws)?);
    let universe = lintcov::load_universe(root, ws)?;
    let coverage = lintcov::coverage(root, ws, &universe, &ledger)?;
    all.extend(coverage.findings);
    all.extend(lintcov::toolchain_findings(root)?);
    Ok(Raw {
        findings: findings::normalise(all),
        notes: coverage.notes,
    })
}

/// The change range of `check`: `--range`, else `--against REF` (as `REF...HEAD`), else
/// `default-range`, else exit 2.
///
/// A run never falls back to scanning without a range. No range would mean no
/// judge or pairing check, and a pass that skipped a check is what this tool
/// exists to prevent. `init` takes an optional range for its base check; `canaries` and `self-test` take none.
/// `A..B` takes `A` as the base and `A...B` the merge base; a bare revision is
/// the base. A normal run also needs a full clone (`resolve_base` refuses
/// shallow repositories) and clippy of exactly the pinned version (lint coverage
/// runs `cargo clippy -- -W help`), so it is slow and not hermetic.
///
/// `default-range` is read from the config of the change under review, so the
/// change could rewrite it. The range it names is therefore only a pointer to a
/// base: the value used is the `default-range` of the config at that base (the
/// written one when the base has none), and `check` refuses a config range that
/// selects no changes at all.
fn determine_range(root: &Root, change: &Change, config: &Config) -> Result<Chosen, Failure> {
    if let Some(range) = explicit_range(change) {
        return Ok(Chosen {
            origin: Origin::Command,
            range,
        });
    }
    let Some(written) = &config.default_range else {
        return Err(Failure::usage(&Message::of(
            "no change range: pass --against REF or --range RANGE, or set default-range in .unrepresentable/config",
        )));
    };
    let base = git::resolve_base(root, written)?;
    let trusted = against::base_config(root, &base)?;
    let range = match trusted.default_range {
        Some(at_base) => at_base,
        None => written.clone(),
    };
    Ok(Chosen {
        origin: Origin::Config,
        range,
    })
}

/// The range named on the command line: `--range` as written, `--against REF`
/// as `REF...HEAD`.
#[must_use]
fn explicit_range(change: &Change) -> Option<Text> {
    match change {
        Change::Range(range) => Some(range.clone()),
        Change::Against(reference) => {
            Some(Text::from_string(format!("{}...HEAD", reference.as_str())))
        }
        Change::Unspecified => None,
    }
}

/// Renders the report as text for standard output.
fn render(report: &Report) -> Message {
    let mut text = String::new();
    if let Some(range) = &report.range {
        text.extend([format!("range: {}\n", range.as_str())]);
    }
    for finding in report.findings.iter().chain(report.notes.iter()) {
        text.push_str(finding.render().as_str());
        text.push('\n');
    }
    for reason in &report.approvals {
        text.extend([format!("approval: {}\n", reason.as_str())]);
    }
    Message::from_string(text)
}

/// Runs the program for the given arguments and returns the exit status.
pub(crate) fn run(args: Vec<OsString>) -> Result<ExitStatus, Failure> {
    let options = match parse_args(args)? {
        Parsed::Display(text) => {
            io::write_stdout(&text)?;
            return Ok(ExitStatus::Clean);
        }
        Parsed::Run(options) => options,
    };
    let root = Root::new(options.root);
    match options.command {
        Command::Canaries => run_canaries(&root),
        Command::Check { change, ratchet } => check(&root, &change, ratchet),
        Command::Init { change } => init(&root, &change),
        Command::SelfTest => print_canary_findings(&selftest::run()?),
    }
}

/// `check`: runs all checks, applies the baseline and the change range.
fn check(root: &Root, change: &Change, ratchet: Mode) -> Result<ExitStatus, Failure> {
    let config_path = RelPath::of(CONFIG_FILE);
    let mut config = config::parse(&config_path, &root.read(&config_path)?)?;
    let ws = Workspace::load(root)?;
    let dirs = member_dirs(&ws);
    let mut raw = collect(root, &config, &ws)?;
    match ratchet {
        Mode::On => {
            let applied = baseline::apply(&config, &raw.findings)?;
            let entries = baseline::ratchet_entries(&applied, &dirs)?;
            rewrite_baseline(root, &entries)?;
            config = config::parse(&config_path, &root.read(&config_path)?)?;
            raw = collect(root, &config, &ws)?;
        }
        Mode::Off => {}
    }
    let applied = baseline::apply(&config, &raw.findings)?;
    let mut all_findings = applied.remaining;
    all_findings.extend(applied.baseline_findings);
    let chosen = determine_range(root, change, &config)?;
    let range = chosen.range;
    let base = git::resolve_base(root, &range)?;
    let changes = git::changes(root, &range)?;
    match (chosen.origin, changes.is_empty()) {
        (Origin::Config, true) => {
            return Err(Failure::usage(&Message::from_string(format!(
                "default-range `{}` selects no changes, so nothing would be checked; a config cannot vouch for its own range (a change may have rewritten it): pass --against REF or --range RANGE",
                range.as_str()
            ))));
        }
        (Origin::Command | Origin::Config, false | true) => {}
    }
    let outcome = against::evaluate(root, &config, &base, &changes, &dirs)?;
    all_findings.extend(outcome.findings);
    let report = Report {
        approvals: outcome.approvals,
        findings: findings::normalise(all_findings),
        notes: raw.notes,
        range: Some(range),
    };
    io::write_stdout(&render(&report))?;
    if !report.findings.is_empty() {
        Ok(ExitStatus::Findings)
    } else if !report.approvals.is_empty() {
        Ok(ExitStatus::Approval)
    } else {
        Ok(ExitStatus::Clean)
    }
}

/// Prints canary findings and maps them to an exit status.
fn print_canary_findings(found: &[Finding]) -> Result<ExitStatus, Failure> {
    let mut text = String::new();
    for finding in found {
        text.push_str(finding.render().as_str());
        text.push('\n');
    }
    io::write_stdout(&Message::from_string(text))?;
    if found.is_empty() {
        Ok(ExitStatus::Clean)
    } else {
        Ok(ExitStatus::Findings)
    }
}

/// Runs `canaries`: rule canaries and, when configured, clippy canaries.
fn run_canaries(root: &Root) -> Result<ExitStatus, Failure> {
    let config_path = RelPath::of(CONFIG_FILE);
    let config = config::parse(&config_path, &root.read(&config_path)?)?;
    let ws = Workspace::load(root)?;
    let mut found = canaries::run_rule_canaries(root, &config)?;
    if config.clippy_canaries.is_some() {
        let ledger = read_ledger(root)?;
        let universe = lintcov::load_universe(root, &ws)?;
        let coverage = lintcov::coverage(root, &ws, &universe, &ledger)?;
        found.extend(canaries::run_clippy_canaries(
            root,
            &config,
            &ws,
            &universe,
            &coverage.denied,
        )?);
    }
    print_canary_findings(&findings::normalise(found))
}

/// Replaces every `[[baseline-entry]]` block of the config with `entries`.
fn rewrite_baseline(root: &Root, entries: &[baseline::Fingerprint]) -> Result<(), Failure> {
    let path = RelPath::of(CONFIG_FILE);
    let source = root.read(&path)?;
    let doc = toml::parse(&path, &source)?;
    let lines: Vec<&str> = source.as_str().lines().collect();
    let mut keep = vec![true; lines.len()];
    let starts: Vec<(usize, bool)> = doc
        .sections
        .iter()
        .map(|section| {
            let is_baseline = section.kind == SectionKind::ArrayOfTables
                && section.header.first().map(Name::as_str) == Some("baseline-entry");
            (section.line.get(), is_baseline)
        })
        .collect();
    for (index, (start, is_baseline)) in starts.iter().enumerate() {
        if !is_baseline {
            continue;
        }
        let end = match starts.get(Count::new(index).next()?.get()) {
            Some(next) => next.0,
            None => Count::new(lines.len()).next()?.get(),
        };
        for line in *start..end {
            if let Some(slot) = keep.get_mut(Count::new(line).prev()?.get()) {
                *slot = false;
            }
        }
    }
    let mut text = String::new();
    for (line, kept) in lines.iter().zip(keep.iter()) {
        if *kept {
            text.push_str(line);
            text.push('\n');
        }
    }
    let mut out = text.trim_end().to_owned();
    out.push_str("\n\n");
    out.push_str(baseline::render(entries).as_str());
    root.write(&path, &Source::from_string(format!("{}\n", out.trim_end())))
}

/// `init` refuses to overwrite a config, here or at the base of the range.
fn init_preconditions(root: &Root, change: &Change) -> Result<(), Failure> {
    let config_path = RelPath::of(CONFIG_FILE);
    if root.is_file(&config_path) == Answer::Yes {
        return Err(Failure::usage(&Message::of(
            "the config file .unrepresentable/config already exists; init refuses to overwrite it",
        )));
    }
    if let Some(range) = explicit_range(change) {
        let base = git::resolve_base(root, &range)?;
        if git::exists_at(root, &base, &config_path) == Answer::Yes {
            return Err(Failure::usage(&Message::of(
                "the config file .unrepresentable/config exists at the base ref; init is refused",
            )));
        }
    }
    Ok(())
}

/// Writes the empty ledgers `init` leaves behind, unless they exist.
fn write_stub_ledgers(root: &Root) -> Result<(), Failure> {
    let ledger_text = format!(
        "# {LEDGER_FILE}\n\n## Expect ledger\n\n| File | Item | Lint | Reason |\n| --- | --- | --- | --- |\n| \u{2014} | \u{2014} | \u{2014} | \u{2014} |\n\n## Lint omissions\n\n| Lint | Contradicts | Reason |\n| --- | --- | --- |\n| \u{2014} | \u{2014} | \u{2014} |\n"
    );
    let ledger_path = RelPath::of(LEDGER_FILE);
    if root.is_file(&ledger_path) == Answer::No {
        root.write(&ledger_path, &Source::from_string(ledger_text))?;
    }
    let exemptions_path = RelPath::of(CANARY_EXEMPTIONS_FILE);
    if root.is_file(&exemptions_path) == Answer::No {
        root.write(
            &exemptions_path,
            &Source::from_string(
                "# Canary exemptions\n\n## Canary exemptions\n\n| Lint | Why it cannot be triggered |\n| --- | --- |\n| \u{2014} | \u{2014} |\n"
                    .to_owned(),
            ),
        )?;
    }
    Ok(())
}

/// The `code = [..]` line `init` writes: the `src` and `tests` directories of
/// every member. `code` is required, because `judge-code-pairing` has no code
/// paths to pair with otherwise; the line is a starting point to edit.
fn init_code_line(ws: &Workspace) -> Text {
    let mut globs: Vec<String> = Vec::new();
    for member in &ws.members {
        for dir in ["src", "tests"] {
            globs.push(format!("\"{}/**\"", member.dir.join(dir).as_str()));
        }
    }
    globs.sort();
    globs.dedup();
    Text::from_string(format!("code = [{}]\n", globs.join(", ")))
}

/// Runs `init`: writes a config with a baseline of current findings and stub ledgers.
fn init(root: &Root, change: &Change) -> Result<ExitStatus, Failure> {
    let config_path = RelPath::of(CONFIG_FILE);
    init_preconditions(root, change)?;
    let ws = Workspace::load(root)?;
    let mut text = String::from(
        "# unrepresentable configuration: facts about the project and recorded exceptions.\n# Every rule is on. There is no option to turn one off.\n\nbaseline = \"shrink-only\"\n",
    );
    let mut config = Config::empty();
    text.push_str(init_code_line(&ws).as_str());
    for member in &ws.members {
        text.extend([format!(
            "\n[member.{}]\nrole = \"pure\"\n",
            member.name.as_str()
        )]);
        config.members.push(config::MemberConfig {
            ambient: Vec::new(),
            name: member.name.clone(),
            newtypes: Vec::new(),
            role: config::Role::Pure,
        });
    }
    write_stub_ledgers(root)?;
    let analysis = analyze::analyze(root, &config, &ws)?;
    let applied = baseline::apply(&config, &analysis.findings)?;
    // The initial baseline records everything currently found.
    let all: Vec<baseline::Fingerprint> = applied
        .unbaselined
        .iter()
        .map(|positioned| positioned.print.clone())
        .collect();
    text.push('\n');
    text.push_str(baseline::render(&all).as_str());
    root.write(
        &config_path,
        &Source::from_string(format!("{}\n", text.trim_end())),
    )?;
    Ok(ExitStatus::Clean)
}
