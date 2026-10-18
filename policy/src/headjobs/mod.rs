//! Turn the (untrusted) output of head-code jobs into strict result
//! artifacts. Head code may write false results; these results are necessary, not sufficient.

#[cfg(test)]
mod tests;

use crate::artifacts;
use crate::exit::Report;
use crate::failure::Failure;
use crate::io::{self, EntryKind};
use crate::reason::{Code, Commits, Counts, Job, Reason};
use crate::types::{Answer, Count, Holds as _, Message, Name, RelPath, Text};
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
/// The evidence of the reason a diff that changes Rust lines gets when
/// `cargo mutants` selected no mutant.
const MUTANTS_NOT_RUN: &str = "cargo-mutants selected no mutant for a diff that changes Rust lines; no mutation result exists";
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
/// The start of every line git writes outside a hunk of a text diff.
const DIFF_HEADERS: [&str; 14] = [
    "diff --git ",
    "index ",
    "--- ",
    "+++ ",
    "old mode ",
    "new mode ",
    "deleted file mode ",
    "new file mode ",
    "similarity index ",
    "dissimilarity index ",
    "rename from ",
    "rename to ",
    "copy from ",
    "copy to ",
];

/// The reasons the mutants lists give, and what is wrong with the lists.
struct Listed {
    /// Lines that say a list is missing or empty although the exit status says
    /// it is not.
    complaints: Vec<Text>,
    /// The reasons.
    reasons: Vec<Reason>,
}

/// What a unified diff does to Rust files. Only a diff that is proved to
/// change no line of a `.rs` file says `Nothing`; a diff that cannot be read to
/// its end says `Unknown`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Touched {
    /// No line of a `.rs` file is added or removed: the diff is empty, or only
    /// renames files, or changes modes, or changes files that are not `.rs`.
    Nothing,
    /// A line of a `.rs` file is added or removed. A removed line counts:
    /// `cargo mutants` also tests the lines next to a deleted line, so a diff
    /// that only removes lines inside a function selects mutants, and so does a
    /// deleted file.
    RustLines,
    /// The diff is not understood.
    Unknown,
}

/// What the line before the one being read was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Before {
    /// A line of a hunk that is not a `\` marker.
    Content,
    /// A `\` marker (`\ No newline at end of file`).
    Marker,
    /// Anything else, including a hunk header and the start of the diff.
    Other,
}

/// How many lines of a hunk are still to come, on the old and on the new side.
#[derive(Clone, Copy)]
struct Hunk {
    /// Lines of the new side still to read.
    new: Count,
    /// Lines of the old side still to read.
    old: Count,
}

/// The state of reading a unified diff line by line.
struct Reading {
    /// What the line before was.
    before: Before,
    /// The hunk that is being read, if any.
    inside: Option<Hunk>,
    /// Whether a `---` or `+++` header of the current file was read.
    named: Answer,
    /// Whether the current file is a `.rs` file. A file whose name is not read
    /// yet counts as one.
    rust: Answer,
    /// Whether a line of a `.rs` file was added or removed.
    touched: Answer,
}

impl Reading {
    /// Notes a header line of git outside a hunk. `diff --git` starts a new
    /// file whose name is not read yet; `---` and `+++` name the file, and a
    /// rename to or from a `.rs` file counts as one.
    fn header(&mut self, line: &Text) {
        let text = line.as_str();
        if text.starts_with("diff --git ") {
            self.named = Answer::No;
            self.rust = Answer::Yes;
            return;
        }
        let ((Some(path), _) | (None, Some(path))) =
            (text.strip_prefix("--- "), text.strip_prefix("+++ "))
        else {
            return;
        };
        let rust = rust_path(&Text::of(path));
        self.rust = if self.named == Answer::Yes {
            (self.rust == Answer::Yes || rust == Answer::Yes).answer()
        } else {
            rust
        };
        self.named = Answer::Yes;
    }

    /// Reads one line. An error means the line is not understood: a hunk that
    /// is longer than its header says, a line outside a hunk that is not a
    /// header git writes there, or a `\` marker anywhere but directly after
    /// a line of a hunk (inside it, or right after its last line); a marker
    /// takes nothing from the sizes of the hunk.
    fn read(&mut self, line: &Text) -> Result<(), Failure> {
        let text = line.as_str();
        let first = text.chars().next();
        if first == Some('\\') {
            if self.before != Before::Content {
                return Err(Failure::refused(&Message::of(
                    "a `\\` marker is not after a line of a hunk",
                )));
            }
            self.before = Before::Marker;
            return Ok(());
        }
        match self.inside {
            None if text.starts_with("@@") => {
                self.inside = Some(hunk_of(line).ok_or_else(|| {
                    Failure::refused(&Message::of("a hunk header is not understood"))
                })?);
                self.before = Before::Other;
            }
            None if DIFF_HEADERS.iter().any(|header| text.starts_with(header)) => {
                self.header(line);
                self.before = Before::Other;
            }
            None => {
                return Err(Failure::refused(&Message::of(
                    "a line outside a hunk is not a header",
                )))
            }
            Some(left) => {
                if matches!(first, Some('+' | '-')) && self.rust == Answer::Yes {
                    self.touched = Answer::Yes;
                }
                self.inside = hunk_step(left, line)?;
                self.before = Before::Content;
            }
        }
        Ok(())
    }

    /// The state before the first line.
    const fn start() -> Self {
        Self {
            before: Before::Other,
            inside: None,
            named: Answer::No,
            rust: Answer::Yes,
            touched: Answer::No,
        }
    }
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
    /// The diff of the pull request that `cargo mutants --in-diff` was given,
    /// as the runner made it from git before any head code ran. The sandbox
    /// has a copy that head code cannot change (it is root-owned, mode 0644, in
    /// a root-owned directory); this file is the original, read by the tool on
    /// the runner without going through the sandbox directory, so the decision
    /// does not depend on the permissions of that directory.
    pub diff: PathBuf,
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

/// The sizes of a hunk from its header `@@ -a[,b] +c[,d] @@`; a missing size
/// is 1. None when the header is not that.
#[must_use]
fn hunk_of(header: &Text) -> Option<Hunk> {
    let ranges = header.as_str().strip_prefix("@@ -")?;
    let (old, after_old) = ranges.split_once(" +")?;
    let (new, _) = after_old.split_once(" @@")?;
    let size = |range: &str| {
        let (start, length) = match range.split_once(',') {
            Some((first, second)) => (first, Some(second)),
            None => (range, None),
        };
        Count::parse(&Text::of(start)).map(|_start| ())?;
        match length {
            Some(digits) => Count::parse(&Text::of(digits)),
            None => Some(Count::new(1)),
        }
    };
    Some(Hunk {
        new: size(new)?,
        old: size(old)?,
    })
}

/// Reads one line of a hunk: the hunk that is left, or none when this line
/// ends it. A `+` line takes one from the new side, a `-` line one from the
/// old side, and any other line (context, or a context line whose space was
/// lost) one from each side. A `\` marker line is the caller's: it takes none.
/// More lines than the header says is an error.
fn hunk_step(left: Hunk, line: &Text) -> Result<Option<Hunk>, Failure> {
    let (old, new) = match line.as_str().chars().next() {
        Some('-') => (1, 0),
        Some('+') => (0, 1),
        Some(_) | None => (1, 1),
    };
    let rest = Hunk {
        new: left.new.minus(Count::new(new))?,
        old: left.old.minus(Count::new(old))?,
    };
    Ok(if rest.new.get() == 0 && rest.old.get() == 0 {
        None
    } else {
        Some(rest)
    })
}

/// Whether the path after `--- ` or `+++ ` in a diff header is a `.rs` file.
/// Git ends a path that has a space with a tab and quotes a path with unusual
/// characters, so both are removed first.
fn rust_path(rest: &Text) -> Answer {
    let path = match rest.as_str().split_once('\t') {
        Some((before, _)) => before,
        None => rest.as_str(),
    };
    Path::new(path.trim_end_matches('"'))
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("rs"))
        .answer()
}

/// What the unified diff `diff` does to Rust files. The hunk headers give the
/// size of every hunk, so a line that begins with `+` outside a hunk (`+++
/// b/path`) is not mistaken for an added line, and a line inside a hunk is
/// never taken for a header. A line outside a hunk that is not a header git
/// writes there (so a hunk that is longer than its header says, or text that
/// is not a diff), a `\ No newline at end of file` marker anywhere but
/// directly after a line of a hunk, and a diff that is cut short, are not
/// understood and so are `Unknown`, as is `Binary files ... differ`: the
/// workflow makes the diff with `--text`, so a file that git shows that way is
/// one head code marked binary (`.gitattributes`) to hide its changes. The
/// whole diff is read, so a diff that touches Rust lines and is garbled later
/// is `Unknown`, not `RustLines`. This function is the only way a missing
/// `mutants.out` can be a clean result, and it must lean to the side that
/// does not say `Nothing`.
fn touched_of(diff: &Text) -> Touched {
    let mut reading = Reading::start();
    for line in diff.as_str().lines() {
        if reading.read(&Text::of(line)).is_err() {
            return Touched::Unknown;
        }
    }
    if reading.inside.is_some() {
        Touched::Unknown
    } else if reading.touched == Answer::Yes {
        Touched::RustLines
    } else {
        Touched::Nothing
    }
}

/// The result when `cargo mutants` wrote no output directory. `cargo mutants`
/// writes none when it selects no mutant. It selects none when the diff it is
/// given changes no line of a `.rs` file, but also when the changed lines hold
/// no mutable code (comments, test code under `#[cfg(test)]`), and when head
/// code makes it select none (a `.cargo/mutants.toml` that excludes `src`, for
/// one). The output cannot tell these apart, and it is head code's to leave
/// out. So "no mutant" is believed as a clean result only when `cargo mutants`
/// exited 0 and the diff, which the runner made from git outside the sandbox,
/// adds and removes no line of a `.rs` file (so no mutant could have been
/// selected whatever head code did). When it exited 0 and the diff does change
/// Rust lines, the job succeeds with the reason `MUTANTS_NOT_RUN`, which a
/// human must approve: no mutation result exists for those lines. Any other
/// exit status, or a diff that is not understood, is a failed job.
fn no_output(run: &MutantsRun, touched: Touched) -> Result<Report, Failure> {
    if run.exit.get() != 0 || touched == Touched::Unknown {
        return Ok(Report::failed(vec![Text::from_string(format!(
            "cargo mutants exited {} and wrote no output directory, and the diff {}: \
             a missing result is believed only for exit 0 and a diff that is understood",
            run.exit.get(),
            match touched {
                Touched::Nothing => "changes no Rust line",
                Touched::RustLines => "changes Rust lines",
                Touched::Unknown => "is not understood",
            }
        ))]));
    }
    let reasons = match touched {
        Touched::RustLines => vec![Reason::general(
            Code::MutantsNotRun,
            &Text::of(MUTANTS_NOT_RUN),
        )],
        Touched::Nothing | Touched::Unknown => Vec::new(),
    };
    let counts = Counts::empty()
        .with(&Name::of("exit"), run.exit)
        .with(&Name::of("reported"), Count::new(reasons.len()));
    let text = artifacts::emit(Job::Mutants, &run.commits, &reasons, &counts);
    io::write_text(&run.out, &text)?;
    Ok(Report::clean())
}

/// Writes the `mutants` artifact. Exit codes 0 (clean), 2 (missed) and 3
/// (timeouts) are reportable; any other exit is a failed job. So is an exit 2
/// or 3 whose list is missing or empty, and a list that cannot be read: head
/// code wrote them, and a result that says less than the exit status is not a
/// clean one. A missing output directory is a clean result, or the approvable
/// reason `MUTANTS_NOT_RUN`, only in the cases that [`no_output`] describes. The diff is read first, so a missing diff file
/// is an error whatever else is true.
pub(crate) fn mutants(run: &MutantsRun) -> Result<Report, Failure> {
    let touched = touched_of(&io::read_bytes(&run.diff)?.lossy_text());
    if io::exists_no_follow(&run.out_dir) == Answer::No {
        return no_output(run, touched);
    }
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
