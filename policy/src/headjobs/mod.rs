//! Turn the (untrusted) output of head-code jobs into strict result
//! artifacts. Head code may write false results; these results are necessary, not sufficient.

#[cfg(test)]
mod tests;

use crate::artifacts;
use crate::exit::Report;
use crate::failure::Failure;
use crate::io::{self, EntryKind};
use crate::reason::{Code, Commits, Counts, Job, Reason};
use crate::types::{Answer, Count, Message, Name, RelPath, Text};
use std::path::{Path, PathBuf};

/// The most bytes read from a result file.
const READ_LIMIT: usize = 500_000;
/// The most lines read from a result file.
const LINE_LIMIT: usize = 2000;
/// The most reasons reported for one list.
const REASON_LIMIT: usize = 100;
/// The most characters of a plain line that are kept.
const PLAIN_LIMIT: usize = 200;
/// The most characters of a message that are kept.
const MESSAGE_LIMIT: usize = 300;
/// The checks the `head-checks` job must run, each exactly once, and every one
/// with exit 0. It is the list of `ci/checks.sh` less the check that needs the
/// network (`deny-advisories`) and with `clippy` standing for `clippy-main` and
/// `clippy-policy`. A test keeps it equal to `ci/checks.sh` and to the checks
/// the workflow runs.
pub(crate) const EXPECTED_CHECKS: [&str; 10] = [
    "audit-workflows",
    "canaries",
    "clippy",
    "deny",
    "fmt",
    "owntree",
    "package",
    "policytest",
    "selftest",
    "test",
];

/// The reasons the mutants lists give, and what is wrong with the lists.
struct Listed {
    /// Lines that say a list is missing or empty although the exit status says
    /// it is not.
    complaints: Vec<Text>,
    /// The reasons.
    reasons: Vec<Reason>,
}

/// A missed-mutant line: `src/<path>.rs:<line>:<col>: <message>`.
struct Missed {
    /// The line number.
    line: Count,
    /// The message about the mutant.
    message: Text,
    /// The path of the file.
    path: RelPath,
}

/// Where `cargo mutants` wrote its results, and what it exited with.
pub(crate) struct MutantsRun {
    /// The base and head commits.
    pub commits: Commits,
    /// The exit status of `cargo mutants`.
    pub exit: Count,
    /// Where to write the result.
    pub out: PathBuf,
    /// The directory `cargo mutants` wrote to.
    pub out_dir: PathBuf,
}

/// Which mutant list a line came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    /// The mutant survived the tests.
    Survived,
    /// The tests timed out on the mutant.
    Timeout,
}

impl Kind {
    /// The exit status of `cargo mutants` that says this kind is not empty.
    const fn exit(self) -> Count {
        Count::new(match self {
            Self::Survived => 2,
            Self::Timeout => 3,
        })
    }

    /// The name of the file that lists this kind.
    fn file(self) -> Text {
        Text::of(match self {
            Self::Survived => "missed.txt",
            Self::Timeout => "timeout.txt",
        })
    }

    /// The name of this kind in a message.
    fn name(self) -> Text {
        Text::of(match self {
            Self::Survived => "survived",
            Self::Timeout => "timeout",
        })
    }
}

/// The status file of the head checks, and where the result goes.
pub(crate) struct ChecksRun {
    /// The base and head commits.
    pub commits: Commits,
    /// Where to write the result.
    pub out: PathBuf,
    /// The file with the statuses of the checks.
    pub status_file: PathBuf,
}

/// What the status file says of the checks that must have run.
struct Standing {
    /// Lines naming a check that is missing, repeated, unknown, failed or
    /// garbled.
    complaints: Vec<Text>,
    /// How many lines are a check with an exit status.
    ran: Count,
}

/// The first up-to-2000 lines of the file `kind` names in `out_dir`; none when
/// the directory has no such file. A file that is not a plain file (a link
/// could point anywhere on the runner), or that cannot be read, is an error.
fn read_lines(out_dir: &Path, kind: Kind) -> Result<Option<Vec<Text>>, Failure> {
    let wanted = kind.file();
    if io::is_link(out_dir) == Answer::Yes {
        return Err(Failure::refused(&Message::of(
            "the output directory is a link",
        )));
    }
    let entries = io::list_dir(out_dir)?;
    let Some(entry) = entries
        .iter()
        .find(|item| item.name.as_str() == wanted.as_str())
    else {
        return Ok(None);
    };
    match entry.kind {
        EntryKind::File => {}
        EntryKind::Dir | EntryKind::Other => {
            return Err(Failure::refused(&Message::from_string(format!(
                "{} is not a plain file",
                wanted.as_str()
            ))))
        }
    }
    let bytes = io::read_bytes_limited(&out_dir.join(wanted.as_str()), Count::new(READ_LIMIT))?;
    Ok(Some(
        bytes
            .lossy_text()
            .as_str()
            .lines()
            .take(LINE_LIMIT)
            .map(Text::of)
            .collect(),
    ))
}

/// Parses a missed-mutant line.
#[must_use]
fn missed_of(line: &Text) -> Option<Missed> {
    let (path, rest) = line.as_str().split_once(':')?;
    let middle = path.strip_prefix("src/")?.strip_suffix(".rs")?;
    let middle_ok = !middle.is_empty()
        && middle.len() <= 200
        && middle
            .chars()
            .all(|found| found.is_ascii_alphanumeric() || matches!(found, '_' | '.' | '/' | '-'));
    let (row, after_row) = rest.split_once(':')?;
    let (column, message) = after_row.split_once(": ")?;
    let digits = |text: &str, longest: usize| {
        !text.is_empty()
            && text.len() <= longest
            && text.chars().all(|found| found.is_ascii_digit())
    };
    if !(middle_ok
        && digits(row, 6)
        && digits(column, 6)
        && message.chars().count() <= MESSAGE_LIMIT)
    {
        return None;
    }
    Some(Missed {
        line: Count::parse(&Text::of(row))?,
        message: Text::of(message),
        path: RelPath::of(path),
    })
}

/// The reason a line of a mutants list stands for, if it says anything.
#[must_use]
fn line_reason(kind: Kind, line: &Text) -> Option<Reason> {
    if let Some(found) = missed_of(line) {
        return Some(Reason::at(
            Code::MutantSurvived,
            &Text::from_string(format!(
                "{}: {}",
                kind.name().as_str(),
                found.message.as_str()
            )),
            &found.path,
            found.line,
        ));
    }
    if line.as_str().trim().is_empty() {
        return None;
    }
    let shown: String = line.as_str().chars().take(PLAIN_LIMIT).collect();
    Some(Reason::general(
        Code::MutantSurvived,
        &Text::from_string(format!("{}: {shown}", kind.name().as_str())),
    ))
}

/// The reasons for the mutants of one list; none when the list is missing.
fn list_reasons(out_dir: &Path, kind: Kind) -> Result<Vec<Reason>, Failure> {
    let mut reasons = Vec::new();
    for line in read_lines(out_dir, kind)?.iter().flatten() {
        reasons.extend(line_reason(kind, line));
    }
    Ok(reasons)
}

/// The reasons for the mutants that `out_dir` lists, and the lines that say a
/// list the exit status promises is empty or missing.
fn mutant_reasons(out_dir: &Path, exit: Count) -> Result<Listed, Failure> {
    let mut reasons = Vec::new();
    let mut complaints = Vec::new();
    for kind in [Kind::Survived, Kind::Timeout] {
        let listed = list_reasons(out_dir, kind)?;
        let promised = exit.get() == kind.exit().get();
        if promised && listed.is_empty() {
            complaints.push(Text::from_string(format!(
                "cargo mutants exited {} but {} lists no mutant",
                exit.get(),
                kind.file().as_str()
            )));
        }
        reasons.extend(listed);
    }
    reasons.truncate(REASON_LIMIT);
    Ok(Listed {
        complaints,
        reasons,
    })
}

/// Writes the `mutants` artifact. Exit codes 0 (clean), 2 (missed) and 3
/// (timeouts) are reportable; any other exit is a failed job. So is an exit 2
/// or 3 whose list is missing or empty, and a list that cannot be read: head
/// code wrote them, and a result that says less than the exit status is not a
/// clean one.
pub(crate) fn mutants(run: &MutantsRun) -> Result<Report, Failure> {
    let Listed {
        complaints,
        reasons,
    } = mutant_reasons(&run.out_dir, run.exit)?;
    let counts = Counts::empty()
        .with(&Name::of("exit"), run.exit)
        .with(&Name::of("reported"), Count::new(reasons.len()));
    let text = artifacts::emit(Job::Mutants, &run.commits, &reasons, &counts);
    io::write_text(&run.out, &text)?;
    if !complaints.is_empty() {
        return Ok(Report::failed(complaints));
    }
    Ok(match run.exit.get() {
        0 | 2 | 3 => Report::clean(),
        _ => Report::clean().into_failed(),
    })
}

/// One `<name> <exit code>` line of the status file.
#[must_use]
fn check_exit(line: &Text) -> Option<Count> {
    let (name, code) = line.as_str().split_once(' ')?;
    let name_ok = !name.is_empty()
        && name.len() <= 40
        && name.chars().all(|found| {
            found.is_ascii_lowercase() || found.is_ascii_digit() || matches!(found, '_' | '-')
        });
    let code_ok =
        !code.is_empty() && code.len() <= 3 && code.chars().all(|found| found.is_ascii_digit());
    if name_ok && code_ok {
        Count::parse(&Text::of(code))
    } else {
        None
    }
}

/// Compares the lines of the status file with `EXPECTED_CHECKS`: every
/// expected check exactly once with exit 0, nothing else.
fn standing_of(lines: &[Text]) -> Result<Standing, Failure> {
    let mut complaints = Vec::new();
    let mut seen: Vec<(Text, Count)> = Vec::new();
    let mut ran = Count::new(0);
    for line in lines {
        let (Some((word, _)), Some(code)) = (line.as_str().split_once(' '), check_exit(line))
        else {
            complaints.push(Text::of("a line of the status file is not `name exit`"));
            continue;
        };
        let name = Text::of(word);
        ran = ran.next()?;
        if code.get() != 0 {
            complaints.push(Text::from_string(format!(
                "check {} exited {}",
                name.as_str(),
                code.get()
            )));
        }
        seen.push((name, code));
    }
    for wanted in EXPECTED_CHECKS {
        let times = seen
            .iter()
            .filter(|(name, _)| name.as_str() == wanted)
            .count();
        if times != 1 {
            complaints.push(Text::from_string(format!(
                "check {wanted} appears {times} times, not once"
            )));
        }
    }
    for (name, _) in &seen {
        if !EXPECTED_CHECKS.contains(&name.as_str()) {
            complaints.push(Text::from_string(format!(
                "check {} is not one of the expected checks",
                name.as_str()
            )));
        }
    }
    Ok(Standing { complaints, ran })
}

/// Writes the `head-checks` artifact. The job passes only when every check of
/// `EXPECTED_CHECKS` is in the status file exactly once with exit 0 and the
/// file holds nothing else: a missing or unreadable file, a missing, repeated,
/// unknown, failed or garbled line all fail it.
pub(crate) fn checks(run: &ChecksRun) -> Result<Report, Failure> {
    let lines = status_lines(&run.status_file)?;
    let standing = standing_of(&lines)?;
    let counts = Counts::empty()
        .with(&Name::of("checks"), standing.ran)
        .with(&Name::of("failed"), Count::new(standing.complaints.len()));
    let text = artifacts::emit(Job::HeadChecks, &run.commits, &[], &counts);
    io::write_text(&run.out, &text)?;
    Ok(if standing.complaints.is_empty() {
        Report::clean()
    } else {
        Report::failed(standing.complaints)
    })
}

/// The lines of the status file; a file that cannot be read is an error.
fn status_lines(path: &Path) -> Result<Vec<Text>, Failure> {
    let bytes = io::read_bytes_limited(path, Count::new(READ_LIMIT))?;
    Ok(bytes
        .lossy_text()
        .as_str()
        .lines()
        .take(LINE_LIMIT)
        .map(Text::of)
        .collect())
}
