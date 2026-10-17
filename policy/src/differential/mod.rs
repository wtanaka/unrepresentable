//! Differential check: every finding the BASE binary reports on an input must
//! also be reported by the HEAD binary (gaining findings is fine).
//!
//! Orchestration is base code. Both binaries only ever run through a sandbox
//! prefix (no network, no tokens, empty environment, unprivileged user) on
//! read-only inputs, because an input can come from a pull request and a
//! build script or cargo configuration in it would otherwise run as the
//! runner. An input from the pull request also never keeps its own
//! configuration or manifests (see [`workspace`]).
//!
//! The two binaries run as different users ([`BASE_UID`] and [`HEAD_UID`]),
//! each with a scratch directory of its own that only that user can enter
//! and that is made fresh for every input. The head binary can therefore
//! neither run the base binary (the workflow keeps it where the head user
//! cannot read it) nor leave a file for the base binary's next run. This
//! keeps a head binary that notices the comparison from handing the work to
//! the base binary. It does not stop a head binary that carries the base
//! rules itself and behaves differently when it recognises the comparison:
//! no black-box comparison can. That is why a change under `src/` needs a
//! human (`RULES_CODE_CHANGED`) and the comparison is a second check for a
//! change that loses findings by mistake.
//!
//! The hidden corpus is secret. It is compared in a run of its own, and a
//! finding lost on it is reported only as a number, so a pull request cannot
//! read the corpus out of the findings it makes the head lose.

pub(crate) mod corpus;
pub(crate) mod workspace;

#[cfg(test)]
mod tests;

use crate::failure::Failure;
use crate::io::{self, EntryKind};
use crate::process::{self, Args, EnvPair, Invocation, SandboxUser, Status, BASE_UID, HEAD_UID};
use crate::reason::{Code, Counts, Reason};
use crate::types::{Answer, Count, Holds as _, Name, RelPath, Text};
use alloc::collections::BTreeSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// The most reasons a comparison reports.
pub(crate) const MAX_REASONS: usize = 100;
/// The most characters of a finding message that are kept.
const MESSAGE_LIMIT: usize = 120;
/// The most characters of a rule name that are kept.
const NAME_LIMIT: usize = 16;
/// The source file shown when a rule is not found.
const FALLBACK_SOURCE: &str = "src/rules.rs";
/// The fewest findings the base binary must report on an input of each name.
/// An empty or unrecognised corpus, or a change of the output format, would
/// otherwise give no findings and so nothing to lose. Any other name needs at
/// least one finding. The numbers are far below what the inputs give today
/// (about a thousand on the fixtures), so they catch a vacuous run without
/// failing a run that merely finds less.
const MINIMUM_FINDINGS: [(&str, usize); 3] = [("fixtures", 100), ("hidden", 1), ("public", 1)];
/// The most lines of a tool's standard error that are shown in an error.
const STDERR_LINES: usize = 12;
/// The most characters of one line of a tool's standard error that are shown.
const STDERR_LINE_LIMIT: usize = 200;
/// The lines of the tool's output that are not findings.
const OTHER_LINE_PREFIXES: [&str; 2] = ["approval: ", "range: "];
/// The change range the tool is given: `HEAD` names no change, and a range
/// given on the command line may select nothing (only one read from the
/// config may not).
const RANGE: &str = "HEAD";

/// A finding with its line and column dropped, so moving code does not look
/// like losing a finding.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct FindingKey {
    /// The message of the finding.
    pub message: Text,
    /// The path of the finding.
    pub path: Text,
    /// The rule that produced the finding.
    pub rule: Text,
}

/// The findings the base and the head reported on one input.
pub(crate) struct Findings {
    /// The findings of the base binary.
    pub base: BTreeSet<FindingKey>,
    /// The findings of the head binary.
    pub head: BTreeSet<FindingKey>,
}

/// One input to compare on: its workspace and where each binary may build.
#[derive(Clone, Debug)]
pub(crate) struct Input {
    /// The name of the input.
    pub name: Name,
    /// Where the input came from.
    pub origin: Origin,
    /// The workspace to run on.
    pub root: PathBuf,
    /// The scratch build directories of the two binaries.
    pub targets: TargetDirs,
}

/// A line in a source file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Location {
    /// The line number.
    pub line: Count,
    /// The path of the file.
    pub path: RelPath,
}

/// Where an input came from, which decides how it is prepared and reported.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Origin {
    /// Taken from the pull request: never trusted, so its configuration and
    /// manifests are replaced.
    Head,
    /// The secret corpus: lost findings are only counted.
    Hidden,
    /// A pinned public corpus.
    Public,
}

/// The findings in the output of a binary, and the lines that are neither a
/// finding nor a known other line.
pub(crate) struct Parsed {
    /// The findings.
    pub findings: BTreeSet<FindingKey>,
    /// How many lines are not findings and not one of the tool's other lines.
    pub unparsed: Count,
}

/// A directory to search, how it is shown, and the text to look for.
struct Search {
    /// The directory to search.
    dir: PathBuf,
    /// The text to look for.
    needle: Text,
    /// The directory name to show in the result.
    shown: RelPath,
}

/// A finding the head lost, on a named input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LostFinding {
    /// The input the finding was lost on.
    pub input: Name,
    /// The finding that was lost.
    pub key: FindingKey,
}

/// The writable build directories of one input, one for each binary. The head
/// binary may run as another user, so the two never share a directory.
#[derive(Clone, Debug)]
pub(crate) struct TargetDirs {
    /// The build directory of the base binary.
    pub base: PathBuf,
    /// The build directory of the head binary.
    pub head: PathBuf,
}

/// The binaries and the sandbox prefix to compare.
pub(crate) struct Tools {
    /// The base binary.
    pub base: PathBuf,
    /// The head binary.
    pub head: PathBuf,
    /// The sandbox words to put in front of each run of either binary.
    pub prefix: Vec<Text>,
}

/// What comparing both binaries over all inputs found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Comparison {
    /// The counts of inputs and findings.
    pub counts: Counts,
    /// The inputs on which a binary failed to run, or on which the base
    /// binary's output cannot be trusted to be complete.
    pub errors: Vec<Text>,
    /// How many findings the head lost on the hidden corpus.
    pub hidden_lost: Count,
    /// The findings the head lost.
    pub lost: Vec<LostFinding>,
}

impl Comparison {
    /// A comparison that has found nothing yet.
    const fn empty() -> Self {
        Self {
            counts: Counts::empty(),
            errors: Vec::new(),
            hidden_lost: Count::new(0),
            lost: Vec::new(),
        }
    }
}

/// How many findings the base binary reported on an input, and how many it
/// had to.
struct Shortfall {
    /// What the base binary reported.
    found: Count,
    /// The fewest it had to report.
    least: Count,
}

/// What one run of a tool printed and how it ended.
struct ToolOutput {
    /// How the tool ended.
    status: Status,
    /// What the tool printed on standard error.
    stderr: Text,
    /// What the tool printed on standard output.
    stdout: Text,
}

/// A program, the sandbox words to put in front of it and the input to run on.
struct ToolRun<'run> {
    /// The scratch directory the program may build in.
    build: &'run Path,
    /// The sandbox words to put in front of the program.
    prefix: &'run [Text],
    /// The program to run.
    program: &'run Path,
    /// The input directory to run on.
    root: &'run Path,
    /// The user id to run as.
    uid: Count,
}

/// Parses `path:line:col: rule: message`.
#[must_use]
fn finding_of(line: &Text) -> Option<FindingKey> {
    let digits = |text: &str| !text.is_empty() && text.chars().all(|found| found.is_ascii_digit());
    let (path, after_path) = line.as_str().split_once(':')?;
    if path.is_empty() {
        return None;
    }
    let (row, after_row) = after_path.split_once(':')?;
    let (column, after_column) = after_row.split_once(": ")?;
    if !(digits(row) && digits(column)) {
        return None;
    }
    let (head, message) = after_column.split_once(' ')?;
    let rule = head.strip_suffix(':')?;
    let rule_ok = !rule.is_empty()
        && rule
            .chars()
            .all(|found| found.is_ascii_alphanumeric() || matches!(found, '_' | '-' | ':' | '.'));
    rule_ok.then(|| FindingKey {
        message: Text::of(message),
        path: Text::of(path),
        rule: Text::of(rule),
    })
}

/// The findings in a binary's output, and how many of its lines are something
/// else than a finding, a blank line or one of the tool's other lines.
pub(crate) fn parse_output(output: &Text) -> Result<Parsed, Failure> {
    let mut findings = BTreeSet::new();
    let mut unparsed = Count::new(0);
    for line in output.as_str().lines() {
        if let Some(found) = finding_of(&Text::of(line)) {
            match findings.insert(found) {
                true | false => {}
            }
            continue;
        }
        let known = line.trim().is_empty()
            || OTHER_LINE_PREFIXES
                .iter()
                .any(|prefix| line.starts_with(prefix));
        if !known {
            unparsed = unparsed.next()?;
        }
    }
    Ok(Parsed { findings, unparsed })
}

/// The fewest findings the base binary must report on the input `name`.
fn minimum_findings(name: &Name) -> Count {
    match MINIMUM_FINDINGS
        .iter()
        .find(|(known, _)| *known == name.as_str())
    {
        Some((_, least)) => Count::new(*least),
        None => Count::new(1),
    }
}

/// Findings the base reported that the head did not, in order.
pub(crate) fn lost(found: &Findings) -> Vec<FindingKey> {
    found.base.difference(&found.head).cloned().collect()
}

/// The file and line of the string literal naming `rule` under `base_dir/src`,
/// else a fallback.
pub(crate) fn rule_source(base_dir: &Path, rule: &Text) -> Result<Location, Failure> {
    let search = Search {
        dir: base_dir.join("src"),
        needle: Text::from_string(format!("\"{}\"", rule.as_str())),
        shown: RelPath::of("src"),
    };
    Ok(match search_dir(&search)? {
        Some(location) => location,
        None => Location {
            line: Count::new(1),
            path: RelPath::of(FALLBACK_SOURCE),
        },
    })
}

/// Whether `name` is a Rust source file name.
fn is_rust_file(name: &Name) -> Answer {
    (Path::new(name.as_str()).extension() == Some(OsStr::new("rs"))).answer()
}

/// Finds the first line of a file in the directory that has the needle.
fn search_dir(search: &Search) -> Result<Option<Location>, Failure> {
    let Ok(entries) = io::list_dir(&search.dir) else {
        return Ok(None);
    };
    for entry in entries
        .iter()
        .filter(|item| item.kind == EntryKind::File && is_rust_file(&item.name) == Answer::Yes)
    {
        let Ok(text) = io::read_text(&search.dir.join(entry.name.as_str())) else {
            continue;
        };
        for (index, text_line) in text.as_str().lines().enumerate() {
            if text_line.contains(search.needle.as_str()) {
                return Ok(Some(Location {
                    line: Count::new(index).next()?,
                    path: RelPath::from_string(format!(
                        "{}/{}",
                        search.shown.as_str(),
                        entry.name.as_str()
                    )),
                }));
            }
        }
    }
    for entry in entries.iter().filter(|item| item.kind == EntryKind::Dir) {
        let child = Search {
            dir: search.dir.join(entry.name.as_str()),
            needle: search.needle.clone(),
            shown: RelPath::from_string(format!(
                "{}/{}",
                search.shown.as_str(),
                entry.name.as_str()
            )),
        };
        if let Some(found) = search_dir(&child)? {
            return Ok(Some(found));
        }
    }
    Ok(None)
}

/// The reasons for the first hundred lost findings, each pointing at the base
/// source line that names the rule.
pub(crate) fn to_reasons(
    lost_all: &[LostFinding],
    base_dir: &Path,
) -> Result<Vec<Reason>, Failure> {
    let mut out = Vec::new();
    for finding in lost_all.iter().take(MAX_REASONS) {
        let source = rule_source(base_dir, &finding.key.rule)?;
        let message: String = finding
            .key
            .message
            .as_str()
            .chars()
            .take(MESSAGE_LIMIT)
            .collect();
        let evidence = format!(
            "rule {} lost on {}:{}: {message}",
            finding.key.rule.as_str(),
            finding.input.as_str(),
            finding.key.path.as_str()
        );
        out.push(Reason::at(
            Code::LostFinding,
            &Text::from_string(evidence),
            &source.path,
            source.line,
        ));
    }
    Ok(out)
}

/// The environment a tool gets: its own build directory, a home, a temporary
/// directory and a cargo home of its own inside it (so one run cannot leave a
/// cargo configuration for the next), and trust for the workspace repository, which belongs to another
/// user than a sandboxed binary. Only the child gets these; this process's
/// environment is unchanged.
fn tool_env(tool: &ToolRun<'_>) -> Vec<EnvPair> {
    let pair = |name: &str, value: &str| EnvPair {
        name: Name::of(name),
        value: Text::of(value),
    };
    let build = tool.build.to_string_lossy();
    vec![
        pair("CARGO_HOME", &format!("{build}/cargo-home")),
        pair("CARGO_TARGET_DIR", &build),
        pair("HOME", &format!("{build}/home")),
        pair("TMPDIR", &format!("{build}/tmp")),
        pair("GIT_CONFIG_COUNT", "1"),
        pair("GIT_CONFIG_KEY_0", "safe.directory"),
        pair("GIT_CONFIG_VALUE_0", &tool.root.to_string_lossy()),
    ]
}

/// Runs one tool on one input, with its exit status and what it printed on
/// standard output (the findings go there) and on standard error (only the
/// tool's own errors).
fn run_tool(tool: &ToolRun<'_>) -> Result<ToolOutput, Failure> {
    let args = Args::empty()
        .with("check")
        .with("--range")
        .with(RANGE)
        .with(&tool.root.to_string_lossy());
    let program = Name::of(&tool.program.to_string_lossy());
    let invocation = Invocation::new(&program, &args)
        .in_dir(tool.root)
        .behind(tool.prefix, &tool_env(tool))
        .run_as(&SandboxUser {
            scratch: tool.build.to_path_buf(),
            uid: tool.uid,
        });
    let captured = process::run(&invocation)?;
    Ok(ToolOutput {
        status: captured.status,
        stderr: captured.stderr.lossy_text(),
        stdout: captured.stdout.lossy_text(),
    })
}

/// The first lines of a tool's standard error, made safe to print in a job log.
/// The text comes from a binary that may be the head binary, or from a binary
/// run on a pull request's files, so it is never trusted: only printable ASCII
/// is kept (no escape sequence), a line is cut short, and the lines are
/// joined with ` | ` so the whole stays one line of the report. The report line
/// starts with fixed text and holds no newline, so no part of it can start a
/// workflow command (`::error::`, `##[add-mask]`), which the runner reads only
/// at the start of a line.
fn stderr_excerpt(stderr: &Text) -> Text {
    let lines: Vec<String> = stderr
        .as_str()
        .lines()
        .map(|line| {
            line.chars()
                .filter(|found| found.is_ascii_graphic() || *found == ' ')
                .take(STDERR_LINE_LIMIT)
                .collect::<String>()
                .trim()
                .to_owned()
        })
        .filter(|line| !line.is_empty())
        .take(STDERR_LINES)
        .collect();
    Text::from_string(lines.join(" | "))
}

/// The count key for an input.
fn counts_key(input: &Name) -> Name {
    let cleaned: String = input
        .as_str()
        .to_lowercase()
        .chars()
        .map(|found| {
            if found.is_ascii_lowercase() {
                found
            } else {
                '_'
            }
        })
        .take(NAME_LIMIT)
        .collect();
    Name::from_string(format!("base_findings_{cleaned}"))
}

/// Whether a tool exit status is one a run may report.
const fn is_reportable(status: Status) -> Answer {
    match status {
        Status::Code(code) if code.get() <= 1 => Answer::Yes,
        Status::Code(_) | Status::Signal => Answer::No,
    }
}

/// Describes an exit status for a message.
fn describe(status: Status) -> Text {
    match status {
        Status::Code(code) => code.text(),
        Status::Signal => Text::of("signal"),
    }
}

/// The error for a binary whose run on an input cannot be reported. Why it
/// failed (the start of its standard error) is added for every input but the
/// hidden corpus, whose content must not reach the log, not even in an error
/// message of the tool.
fn failed_run(which: &Name, run: &ToolOutput, input: &Input) -> Text {
    let head = format!(
        "{} binary exit {} on input {}",
        which.as_str(),
        describe(run.status).as_str(),
        input.name.as_str()
    );
    let why = stderr_excerpt(&run.stderr);
    match input.origin {
        Origin::Hidden => Text::from_string(head),
        Origin::Head | Origin::Public if why.as_str().is_empty() => Text::from_string(head),
        Origin::Head | Origin::Public => Text::from_string(format!("{head}: {}", why.as_str())),
    }
}

/// The error for an input on which the base binary's output cannot be trusted
/// to be complete. The numbers of the hidden corpus are not given.
fn vacuous_run(input: &Input, shortfall: &Shortfall) -> Text {
    match input.origin {
        Origin::Hidden => Text::from_string(format!(
            "base binary reported too few findings on input {}: an empty corpus or an output format that changed",
            input.name.as_str()
        )),
        Origin::Head | Origin::Public => Text::from_string(format!(
            "base binary reported {} findings on input {}, fewer than the {} expected: an empty corpus or an output format that changed",
            shortfall.found.get(),
            input.name.as_str(),
            shortfall.least.get()
        )),
    }
}

/// Runs both binaries on each input. A binary that fails on an input (an exit
/// other than 0 or 1, or a signal) is an error for that input, never a pass,
/// whatever the other binary did. So is a base binary whose output has lines
/// that are not findings, or fewer findings than the input must give: with
/// nothing found there is nothing to lose. Both run behind the sandbox prefix.
pub(crate) fn compare(tools: &Tools, inputs: &[Input]) -> Result<Comparison, Failure> {
    let mut comparison = Comparison::empty();
    for input in inputs {
        compare_input(tools, input, &mut comparison)?;
    }
    Ok(comparison)
}

/// Runs the base binary on one input. Its findings, or none when its run is an
/// error, which is then in `comparison`.
fn base_findings(
    tools: &Tools,
    input: &Input,
    comparison: &mut Comparison,
) -> Result<Option<BTreeSet<FindingKey>>, Failure> {
    let ran = run_tool(&ToolRun {
        build: &input.targets.base,
        prefix: &tools.prefix,
        program: &tools.base,
        root: &input.root,
        uid: BASE_UID,
    })?;
    if is_reportable(ran.status) == Answer::No {
        comparison
            .errors
            .push(failed_run(&Name::of("base"), &ran, input));
        return Ok(None);
    }
    let parsed = parse_output(&ran.stdout)?;
    if parsed.unparsed.get() > 0 {
        comparison.errors.push(Text::from_string(format!(
            "base binary printed {} lines that are not findings on input {}",
            parsed.unparsed.get(),
            input.name.as_str()
        )));
        return Ok(None);
    }
    let least = minimum_findings(&input.name);
    let found = Count::new(parsed.findings.len());
    if found.get() < least.get() {
        comparison
            .errors
            .push(vacuous_run(input, &Shortfall { found, least }));
        return Ok(None);
    }
    if input.origin != Origin::Hidden {
        comparison.counts = comparison
            .counts
            .clone()
            .with(&counts_key(&input.name), found);
    }
    Ok(Some(parsed.findings))
}

/// Runs both binaries on one input and adds what it finds to `comparison`.
fn compare_input(tools: &Tools, input: &Input, comparison: &mut Comparison) -> Result<(), Failure> {
    let Some(base) = base_findings(tools, input, comparison)? else {
        return Ok(());
    };
    let ran = run_tool(&ToolRun {
        build: &input.targets.head,
        prefix: &tools.prefix,
        program: &tools.head,
        root: &input.root,
        uid: HEAD_UID,
    })?;
    if is_reportable(ran.status) == Answer::No {
        comparison
            .errors
            .push(failed_run(&Name::of("head"), &ran, input));
        return Ok(());
    }
    let gone = lost(&Findings {
        base,
        head: parse_output(&ran.stdout)?.findings,
    });
    if input.origin == Origin::Hidden {
        comparison.hidden_lost = comparison.hidden_lost.plus(Count::new(gone.len()))?;
        return Ok(());
    }
    for key in gone {
        comparison.lost.push(LostFinding {
            input: input.name.clone(),
            key,
        });
    }
    Ok(())
}

/// The reasons for the findings the head lost on the hidden corpus: one
/// reason with the number and nothing of the findings.
pub(crate) fn hidden_reasons(lost_count: Count) -> Vec<Reason> {
    if lost_count.get() == 0 {
        return Vec::new();
    }
    vec![Reason::general(
        Code::LostFindingHidden,
        &Text::from_string(format!(
            "{} findings lost on the hidden corpus",
            lost_count.get()
        )),
    )]
}
