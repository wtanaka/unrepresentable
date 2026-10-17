use super::{checks, mutants, touched_of, ChecksRun, MutantsRun, Touched, EXPECTED_CHECKS};
use crate::artifacts::parse;
use crate::bytes::Bytes;
use crate::exit::{ExitStatus, Report};
use crate::io;
use crate::reason::{Code, Commits, Job, Reason};
use crate::testkit::{Fail, Scratch};
use crate::types::{Answer, Count, Name, RelPath, Text};
use std::path::PathBuf;

/// Checks that `touched_of` gives the wanted answer for every `(diff, answer,
/// why)` of the array, in a check that returns a `Result`.
macro_rules! ensure_touched {
    ($cases:expr_2021) => {
        for (diff, want, why) in $cases {
            ensure_eq!(
                touched_of(&Text::of(AsRef::<str>::as_ref(&diff))),
                want,
                "{why}"
            );
        }
    };
}

/// A diff and what it is a case of.
struct Described {
    /// The diff.
    diff: Text,
    /// What the case is, for the message of a failure.
    why: Text,
}

/// A diff that adds one line.
fn adding_diff() -> Text {
    Text::of("diff --git a/src/a.rs b/src/a.rs\nindex 1111111..2222222 100644\n--- a/src/a.rs\n+++ b/src/a.rs\n@@ -1,2 +1,3 @@\n fn a() {}\n+fn b() {}\n fn c() {}\n")
}
/// A diff that only removes a line inside a function: `cargo mutants` also
/// tests the lines next to a deleted line, so this one selects mutants.
fn removing_diff() -> Text {
    Text::of("diff --git a/src/a.rs b/src/a.rs\nindex 1111111..2222222 100644\n--- a/src/a.rs\n+++ b/src/a.rs\n@@ -1,3 +1,2 @@\n fn a() {}\n-fn b() {}\n fn c() {}\n")
}

fn commits() -> Commits {
    Commits {
        base: sha!('b'),
        head: sha!('a'),
    }
}

fn status(report: &Report) -> ExitStatus {
    report.status
}

/// Writes the diff file the runner would have made, and says where it is.
fn write_diff(scratch: &Scratch, text: &Text) -> Result<PathBuf, Fail> {
    scratch.write(&RelPath::of("pr.diff"), text)?;
    Ok(scratch.path().join("pr.diff"))
}

#[test]
fn mutants_become_reasons() {
    assert_eq!(
        check_mutants_become_reasons(),
        Ok(()),
        "mutants_become_reasons"
    );
}

fn check_mutants_become_reasons() -> Result<(), Fail> {
    let scratch = Scratch::repository()?;
    scratch.write(
        &RelPath::of("mut/missed.txt"),
        &Text::of("src/a.rs:12:5: replace f with ()\nodd line\n\n"),
    )?;
    scratch.write(
        &RelPath::of("mut/timeout.txt"),
        &Text::of("src/b.rs:3:1: slow\n"),
    )?;
    let out = scratch.path().join("out.txt");
    let report = mutants(&MutantsRun {
        commits: commits(),
        diff: write_diff(&scratch, &adding_diff())?,
        exit: Count::new(2),
        out: out.clone(),
        out_dir: scratch.path().join("mut"),
    })?;
    ensure_eq!(status(&report), ExitStatus::Clean, "exit 2 is reportable");
    let parsed = parse(&io::read_bytes(&out)?, Job::Mutants, &commits())?;
    let evidence: Vec<&str> = parsed
        .reasons
        .iter()
        .map(|reason| reason.evidence.as_str())
        .collect();
    ensure_eq!(
        evidence,
        vec![
            "survived: replace f with ()",
            "survived: odd line",
            "timeout: slow"
        ],
        "one reason per non-blank line"
    );
    ensure_eq!(
        parsed
            .reasons
            .first()
            .map(|reason| (reason.code, reason.path.clone(), reason.line)),
        Some((
            Code::MutantSurvived,
            Some(RelPath::of("src/a.rs")),
            Count::new(12)
        )),
        "located lines keep their place"
    );
    Ok(())
}

#[test]
fn mutants_exit_codes() {
    assert_eq!(check_mutants_exit_codes(), Ok(()), "mutants_exit_codes");
}

fn check_mutants_exit_codes() -> Result<(), Fail> {
    let scratch = Scratch::repository()?;
    scratch.write(
        &RelPath::of("mut/missed.txt"),
        &Text::of("src/a.rs:1:1: replace f with ()\n"),
    )?;
    scratch.write(
        &RelPath::of("mut/timeout.txt"),
        &Text::of("src/b.rs:2:1: slow\n"),
    )?;
    for (exit, want) in [
        (0, ExitStatus::Clean),
        (2, ExitStatus::Clean),
        (3, ExitStatus::Clean),
        (1, ExitStatus::Failed),
        (4, ExitStatus::Failed),
    ] {
        let report = mutants(&MutantsRun {
            commits: commits(),
            diff: write_diff(&scratch, &adding_diff())?,
            exit: Count::new(exit),
            out: scratch.path().join("o.txt"),
            out_dir: scratch.path().join("mut"),
        })?;
        ensure_eq!(status(&report), want, "exit {exit}");
    }
    Ok(())
}

#[test]
fn a_mutants_exit_without_its_list_is_not_a_clean_result() {
    assert_eq!(
        check_a_mutants_exit_without_its_list_is_not_a_clean_result(),
        Ok(()),
        "a_mutants_exit_without_its_list_is_not_a_clean_result"
    );
}

fn run_mutants(scratch: &Scratch, exit: Count) -> Result<ExitStatus, Fail> {
    let report = mutants(&MutantsRun {
        commits: commits(),
        diff: write_diff(scratch, &adding_diff())?,
        exit,
        out: scratch.path().join("o.txt"),
        out_dir: scratch.path().join("mut"),
    })?;
    Ok(report.status)
}

fn check_a_mutants_exit_without_its_list_is_not_a_clean_result() -> Result<(), Fail> {
    let scratch = Scratch::repository()?;
    scratch.write(&RelPath::of("mut/caught.txt"), &Text::of("x\n"))?;
    ensure_eq!(
        run_mutants(&scratch, Count::new(2)),
        Ok(ExitStatus::Failed),
        "exit 2 with no missed.txt fails"
    );
    ensure_eq!(
        run_mutants(&scratch, Count::new(3)),
        Ok(ExitStatus::Failed),
        "exit 3 with no timeout.txt fails"
    );
    ensure_eq!(
        run_mutants(&scratch, Count::new(0)),
        Ok(ExitStatus::Clean),
        "exit 0 needs no list"
    );
    scratch.write(&RelPath::of("mut/missed.txt"), &Text::of("\n\n"))?;
    scratch.write(&RelPath::of("mut/timeout.txt"), &Text::of(""))?;
    ensure_eq!(
        run_mutants(&scratch, Count::new(2)),
        Ok(ExitStatus::Failed),
        "exit 2 with an empty missed.txt fails"
    );
    ensure_eq!(
        run_mutants(&scratch, Count::new(3)),
        Ok(ExitStatus::Failed),
        "exit 3 with an empty timeout.txt fails"
    );
    ensure_eq!(
        run_mutants(&scratch, Count::new(0)),
        Ok(ExitStatus::Clean),
        "exit 0 with empty lists is clean"
    );
    Ok(())
}

#[test]
fn an_unreadable_mutants_list_is_an_error() {
    assert_eq!(
        check_an_unreadable_mutants_list_is_an_error(),
        Ok(()),
        "an_unreadable_mutants_list_is_an_error"
    );
}

fn check_an_unreadable_mutants_list_is_an_error() -> Result<(), Fail> {
    let scratch = Scratch::repository()?;
    scratch.write(&RelPath::of("target.txt"), &Text::of("src/a.rs:1:1: m\n"))?;
    scratch.write(&RelPath::of("mut/timeout.txt"), &Text::of(""))?;
    scratch.symlink(&Text::of("../target.txt"), &RelPath::of("mut/missed.txt"))?;
    for exit in [0, 2, 3] {
        ensure!(
            run_mutants(&scratch, Count::new(exit)).is_err(),
            "a link in place of missed.txt is refused at exit {exit}"
        );
    }
    let loose = Scratch::repository()?;
    loose.write(&RelPath::of("mut/missed.txt/inner"), &Text::of("x"))?;
    ensure!(
        run_mutants(&loose, Count::new(2)).is_err(),
        "a directory in place of missed.txt is refused"
    );
    let linked = Scratch::repository()?;
    linked.write(
        &RelPath::of("real/missed.txt"),
        &Text::of("src/a.rs:1:1: m\n"),
    )?;
    linked.symlink(&Text::of("real"), &RelPath::of("mut"))?;
    ensure!(
        run_mutants(&linked, Count::new(2)).is_err(),
        "a link in place of the output directory is refused"
    );
    let absent = Scratch::repository()?;
    ensure_eq!(
        run_mutants(&absent, Count::new(1)),
        Ok(ExitStatus::Failed),
        "a missing output directory does not pass at exit 1, whatever the diff"
    );
    let dangling = Scratch::repository()?;
    dangling.symlink(&Text::of("nowhere"), &RelPath::of("mut"))?;
    ensure!(
        run_mutants(&dangling, Count::new(0)).is_err(),
        "a dangling link in place of the output directory is refused, not taken for a missing one"
    );
    Ok(())
}

/// Runs `mutants` in `scratch`, which has no output directory, with the diff
/// `diff` and exit `exit`.
fn run_without_output(scratch: &Scratch, diff: &Text, exit: Count) -> Result<ExitStatus, Fail> {
    let report = mutants(&MutantsRun {
        commits: commits(),
        diff: write_diff(scratch, diff)?,
        exit,
        out: scratch.path().join("o.txt"),
        out_dir: scratch.path().join("mut"),
    })?;
    Ok(report.status)
}

#[test]
fn a_pull_request_without_mutation_testable_changes_is_clean() {
    assert_eq!(
        check_a_pull_request_without_mutation_testable_changes_is_clean(),
        Ok(()),
        "a_pull_request_without_mutation_testable_changes_is_clean"
    );
}

/// The failure of the first pull-request run: `cargo mutants` printed "Diff file
/// is empty", exited 0 and wrote no `mutants.out`, and the job failed with
/// "No such file or directory".
fn check_a_pull_request_without_mutation_testable_changes_is_clean() -> Result<(), Fail> {
    for (diff, why) in [
        ("", "an empty diff"),
        (
            "diff --git a/src/a.rs b/src/a.rs\nsimilarity index 100%\nrename from src/a.rs\nrename to src/b.rs\n",
            "a pure rename",
        ),
        (
            "diff --git a/src/a.rs b/src/a.rs\nold mode 100644\nnew mode 100755\n",
            "a mode change",
        ),
        (
            "diff --git a/src/data.txt b/src/data.txt\nindex 1..2 100644\n--- a/src/data.txt\n+++ b/src/data.txt\n@@ -1 +1,2 @@\n a\n+b\n",
            "a change of a file that is not Rust",
        ),
    ] {
        let scratch = Scratch::repository()?;
        let ended = run_without_output(&scratch, &Text::of(diff), Count::new(0))?;
        ensure_eq!(ended, ExitStatus::Clean, "{why} with exit 0 is clean");
        let parsed = parse(
            &io::read_bytes(&scratch.path().join("o.txt"))?,
            Job::Mutants,
            &commits(),
        )?;
        ensure!(parsed.reasons.is_empty(), "{why} gives no reason");
        ensure_eq!(
            parsed.counts.get(&Name::of("reported")),
            Count::new(0),
            "{why} reports no mutant"
        );
        ensure_eq!(
            parsed.counts.get(&Name::of("exit")),
            Count::new(0),
            "{why} records exit 0"
        );
    }
    Ok(())
}

#[test]
fn a_diff_of_rust_lines_with_no_selected_mutant_needs_a_human() {
    assert_eq!(
        check_a_diff_of_rust_lines_with_no_selected_mutant_needs_a_human(),
        Ok(()),
        "a_diff_of_rust_lines_with_no_selected_mutant_needs_a_human"
    );
}

/// A test-only or comment-only change under `src/` (and a diff that only
/// removes lines) has no mutant and so no `mutants.out`; `cargo mutants` exits
/// 0. The job succeeds with the approvable reason `MUTANTS_NOT_RUN`, and the
/// reason is never given for another exit status.
fn check_a_diff_of_rust_lines_with_no_selected_mutant_needs_a_human() -> Result<(), Fail> {
    let removing = removing_diff();
    let adding = adding_diff();
    for (diff, why) in [
        (adding.as_str(), "an added line"),
        (removing.as_str(), "a diff that only removes lines"),
        (
            "diff --git a/src/a.rs b/src/a.rs\ndeleted file mode 100644\nindex 1111111..0000000\n--- a/src/a.rs\n+++ /dev/null\n@@ -1,2 +0,0 @@\n-fn a() {}\n-fn b() {}\n",
            "a deleted file",
        ),
        (
            "diff --git a/src/a.rs b/src/b.txt\nsimilarity index 80%\nrename from src/a.rs\nrename to src/b.txt\nindex 1..2 100644\n--- a/src/a.rs\n+++ b/src/b.txt\n@@ -1 +1 @@\n-x\n+y\n",
            "a file renamed away from .rs with changes",
        ),
        (
            "diff --git a/src/w.rs b/src/w.rs\nindex 1..2 100644\n--- \"a/src/w x.rs\"\n+++ \"b/src/w x.rs\"\n@@ -1 +1 @@\n-x\n+y\n",
            "a quoted path",
        ),
        (
            "diff --git a/src/data.txt b/src/data.txt\n--- a/src/data.txt\n+++ b/src/data.txt\n@@ -1 +1 @@\n-a\n+b\ndiff --git a/src/a.rs b/src/a.rs\n--- a/src/a.rs\n+++ b/src/a.rs\n@@ -1 +1 @@\n-a\n+b\n",
            "a Rust file after a file that is not Rust",
        ),
        ("@@ -1 +1 @@\n-a\n+b\n", "a hunk whose file is not named"),
    ] {
        ensure_mutants_not_run(&Described {
            diff: Text::of(diff),
            why: Text::of(why),
        })?;
    }
    Ok(())
}

/// With exit 0 and no output directory, `diff` gives the one approvable
/// reason; with any other exit status it fails and writes no result.
fn ensure_mutants_not_run(case: &Described) -> Result<(), Fail> {
    let diff = &case.diff;
    let why = case.why.as_str();
    let scratch = Scratch::repository()?;
    let ended = run_without_output(&scratch, diff, Count::new(0))?;
    ensure_eq!(ended, ExitStatus::Clean, "{why} with exit 0 succeeds");
    let parsed = parse(
        &io::read_bytes(&scratch.path().join("o.txt"))?,
        Job::Mutants,
        &commits(),
    )?;
    ensure_eq!(
        parsed.reasons,
        vec![Reason::general(
            Code::MutantsNotRun,
            &Text::of(
                "cargo-mutants selected no mutant for a diff that changes Rust lines; no mutation result exists"
            )
        )],
        "{why} gives the one approvable reason"
    );
    ensure_eq!(
        parsed.counts.get(&Name::of("reported")),
        Count::new(1),
        "{why} reports one reason"
    );
    ensure_eq!(
        Code::MutantsNotRun.is_path_based(),
        Answer::No,
        "a policy owner approves it, like the other reasons that name no path"
    );
    for exit in [1, 2, 3, 4, 5, 255] {
        let other = Scratch::repository()?;
        ensure_eq!(
            run_without_output(&other, diff, Count::new(exit))?,
            ExitStatus::Failed,
            "{why} with exit {exit} fails and gives no reason"
        );
        ensure!(
            io::exists_no_follow(&other.path().join("o.txt")) == Answer::No,
            "{why} with exit {exit} writes no result"
        );
    }
    Ok(())
}

#[test]
fn a_missing_mutants_directory_never_hides_a_survivor() {
    assert_eq!(
        check_a_missing_mutants_directory_never_hides_a_survivor(),
        Ok(()),
        "a_missing_mutants_directory_never_hides_a_survivor"
    );
}

fn check_a_missing_mutants_directory_never_hides_a_survivor() -> Result<(), Fail> {
    // Head code that makes cargo mutants select nothing (a mutants.toml that
    // excludes src, a hand-written empty output) gets exit 0 and no directory
    // from a diff that does add lines: that must not be a clean result.
    let scratch = Scratch::repository()?;
    ensure_eq!(
        run_without_output(&scratch, &adding_diff(), Count::new(0))?,
        ExitStatus::Clean,
        "added lines and no output succeed, but not as a clean result"
    );
    let parsed = parse(
        &io::read_bytes(&scratch.path().join("o.txt"))?,
        Job::Mutants,
        &commits(),
    )?;
    ensure_eq!(
        parsed
            .reasons
            .iter()
            .map(|reason| reason.code)
            .collect::<Vec<_>>(),
        vec![Code::MutantsNotRun],
        "the missing output of a diff of Rust lines needs a human"
    );
    for exit in [1, 2, 3, 4, 5, 255] {
        for diff in [Text::of(""), removing_diff(), adding_diff()] {
            ensure_eq!(
                run_without_output(&scratch, &diff, Count::new(exit))?,
                ExitStatus::Failed,
                "exit {exit} with no output directory fails"
            );
        }
    }
    for (diff, why) in [
        ("@@ -1,3 +1,3 @@\n a\n", "a hunk cut short"),
        ("@@ nonsense @@\n", "a hunk header that is not one"),
        ("@@ -1,1 +1,1 @@\n a\n b\n", "a hunk longer than its header"),
        ("garbage\n", "text that is not a diff"),
    ] {
        ensure_eq!(
            run_without_output(&scratch, &Text::of(diff), Count::new(0))?,
            ExitStatus::Failed,
            "{why} is not believed"
        );
    }
    let bare = Scratch::repository()?;
    ensure!(
        mutants(&MutantsRun {
            commits: commits(),
            diff: bare.path().join("nothing.diff"),
            exit: Count::new(0),
            out: bare.path().join("o.txt"),
            out_dir: bare.path().join("mut"),
        })
        .is_err(),
        "a missing diff file is an error"
    );
    Ok(())
}

#[test]
fn what_a_diff_touches_is_read_from_its_hunks() {
    assert_eq!(
        check_what_a_diff_touches_is_read_from_its_hunks(),
        Ok(()),
        "what_a_diff_touches_is_read_from_its_hunks"
    );
}

fn check_what_a_diff_touches_is_read_from_its_hunks() -> Result<(), Fail> {
    let adding = adding_diff();
    let removing = removing_diff();
    ensure_touched!([
        ("", Touched::Nothing, "empty"),
        (adding.as_str(), Touched::RustLines, "one line added"),
        (removing.as_str(), Touched::RustLines, "one line removed"),
        (
            "--- a/x.rs\n+++ b/x.rs\n@@ -1,2 +1,1 @@\n-a\n b\n",
            Touched::RustLines,
            "the +++ header is not a line, the removed line is",
        ),
        (
            "--- a/x.rs\n+++ b/x.rs\n@@ -1,1 +1,1 @@\n a\n",
            Touched::Nothing,
            "a hunk of context only changes no line",
        ),
        (
            "--- a/x.rs\n+++ b/x.rs\n@@ -1 +1,2 @@\n a\n+++ b\n",
            Touched::RustLines,
            "an added line whose text starts with ++ is an added line",
        ),
        (
            "--- a/x.txt\n+++ b/x.txt\n@@ -1 +1,2 @@\n a\n+++ b\n",
            Touched::Nothing,
            "the same in a file that is not Rust",
        ),
        (
            "@@ -1,2 +1,1 @@\n-a\n b\n@@ -9 +8,2 @@\n c\n+d\n",
            Touched::RustLines,
            "a change in a later hunk",
        ),
    ]);
    ensure_touched!([
        (
            "--- a/x.rs\n+++ /dev/null\n@@ -1 +0,0 @@\n-a\n",
            Touched::RustLines,
            "a deleted file removes lines",
        ),
        (
            "--- a/x.rs\t\n+++ b/x.rs\t\n@@ -1 +1 @@\n-a\n+b\n",
            Touched::RustLines,
            "a path that ends with a tab",
        ),
        (
            "--- a/x.rss\n+++ b/x.rss\n@@ -1 +1 @@\n-a\n+b\n",
            Touched::Nothing,
            "a path that only starts like .rs",
        ),
    ]);
    Ok(())
}

#[test]
fn a_diff_that_is_not_understood_is_unknown() {
    assert_eq!(
        check_a_diff_that_is_not_understood_is_unknown(),
        Ok(()),
        "a_diff_that_is_not_understood_is_unknown"
    );
}

fn check_a_diff_that_is_not_understood_is_unknown() -> Result<(), Fail> {
    ensure_touched!([
        (
            "@@ -0,0 +1,0 @@\n",
            Touched::Unknown,
            "a hunk of no lines is not trusted",
        ),
        (
            "Binary files a/x and b/x differ\n",
            Touched::Unknown,
            "a file shown as binary may be a hidden change",
        ),
        ("not a diff\n", Touched::Unknown, "text that is not a diff"),
        (
            "@@ -1,1 +1,1 @@\n a\n+b\n",
            Touched::Unknown,
            "a line past the end of its hunk is not ignored",
        ),
        (
            "--- a/x.rs\n+++ b/x.rs\n@@ -1 +1 @@\n-a\n+b\ngarbage\n",
            Touched::Unknown,
            "a diff of Rust lines that is garbled later is not understood",
        ),
    ]);
    Ok(())
}

#[test]
fn a_no_newline_marker_is_read_only_after_a_line_of_a_hunk() {
    assert_eq!(
        check_a_no_newline_marker_is_read_only_after_a_line_of_a_hunk(),
        Ok(()),
        "a_no_newline_marker_is_read_only_after_a_line_of_a_hunk"
    );
    assert_eq!(
        check_a_no_newline_marker_in_the_wrong_place_is_not_understood(),
        Ok(()),
        "a_no_newline_marker_in_the_wrong_place_is_not_understood"
    );
}

fn check_a_no_newline_marker_is_read_only_after_a_line_of_a_hunk() -> Result<(), Fail> {
    let head = "--- a/x.rs\n+++ b/x.rs\n";
    let marker = "\\ No newline at end of file\n";
    ensure_touched!([
        (
            format!("{head}@@ -1 +1 @@\n-a\n{marker}+b\n{marker}"),
            Touched::RustLines,
            "markers inside a hunk and after its last line",
        ),
        (
            format!("{head}@@ -1 +1 @@\n-a\n+b\n{marker}"),
            Touched::RustLines,
            "a marker after the last line of the last hunk",
        ),
        (
            format!("{head}@@ -1 +1 @@\n-a\n+b\n{marker}@@ -5 +5 @@\n-c\n+d\n"),
            Touched::RustLines,
            "a marker after a hunk that is followed by another hunk",
        ),
        (
            format!(
                "{head}@@ -1 +1 @@\n-a\n+b\n{marker}diff --git a/y.rs b/y.rs\n--- a/y.rs\n+++ b/y.rs\n@@ -1 +1 @@\n a\n{marker}"
            ),
            Touched::RustLines,
            "a marker after a hunk that is followed by another file",
        ),
        (
            format!("{head}@@ -1 +1 @@\n a\n{marker}"),
            Touched::Nothing,
            "a marker after context only is no change",
        ),
        (
            format!("{head}@@ -1,2 +1,2 @@\n a\n{marker} b\n{marker}"),
            Touched::Nothing,
            "a marker after each side of a context pair",
        ),
    ]);
    Ok(())
}

fn check_a_no_newline_marker_in_the_wrong_place_is_not_understood() -> Result<(), Fail> {
    let head = "--- a/x.rs\n+++ b/x.rs\n";
    let marker = "\\ No newline at end of file\n";
    ensure_touched!([
        (
            format!("{head}@@ -1 +1 @@\n{marker}-a\n+b\n"),
            Touched::Unknown,
            "a marker right after the hunk header",
        ),
        (
            format!("{head}@@ -1 +1 @@\n-a\n{marker}{marker}+b\n"),
            Touched::Unknown,
            "two markers in a row inside a hunk",
        ),
        (
            format!("{head}@@ -1 +1 @@\n-a\n+b\n{marker}{marker}"),
            Touched::Unknown,
            "two markers in a row after a hunk",
        ),
        (
            format!("{head}{marker}"),
            Touched::Unknown,
            "a marker after the file headers",
        ),
        (marker.to_owned(), Touched::Unknown, "a marker at the start"),
        (
            format!("{head}@@ -1 +1 @@\n-a\n+b\n+++ b/z\n{marker}"),
            Touched::Unknown,
            "a marker after a header that follows a hunk",
        ),
        (
            format!("{head}@@ -1 +1 @@\n-a\n+b\ndiff --git a/y b/y\n{marker}"),
            Touched::Unknown,
            "a marker after a header, not a hunk line",
        ),
        (
            format!("{head}@@ -2 +2 @@\n-a\n{marker}"),
            Touched::Unknown,
            "a hunk cut short after a marker",
        ),
    ]);
    Ok(())
}

#[test]
fn checks_count_failures() {
    assert_eq!(
        check_checks_count_failures(),
        Ok(()),
        "checks_count_failures"
    );
}

/// The status file text of every expected check, each with exit 0 unless
/// `failing` gives it another.
fn status_text(failing: &[(Name, Count)]) -> Text {
    Text::from_string(
        EXPECTED_CHECKS
            .iter()
            .map(|name| {
                let exit = match failing.iter().find(|(known, _)| known.as_str() == *name) {
                    Some((_, code)) => code.get(),
                    None => 0,
                };
                format!("{name} {exit}\n")
            })
            .collect::<Vec<String>>()
            .concat(),
    )
}

fn run_checks(scratch: &Scratch, text: &Text) -> Result<(ExitStatus, Bytes), Fail> {
    scratch.write(&RelPath::of("status.txt"), text)?;
    let out = scratch.path().join("o.txt");
    let report = checks(&ChecksRun {
        commits: commits(),
        out: out.clone(),
        status_file: scratch.path().join("status.txt"),
    })?;
    Ok((report.status, io::read_bytes(&out)?))
}

fn check_checks_count_failures() -> Result<(), Fail> {
    let scratch = Scratch::repository()?;
    let (all_pass, artifact) = run_checks(&scratch, &status_text(&[]))?;
    ensure_eq!(all_pass, ExitStatus::Clean, "every expected check passed");
    ensure!(
        parse(&artifact, Job::HeadChecks, &commits())?
            .counts
            .get(&Name::of("checks"))
            == Count::new(EXPECTED_CHECKS.len()),
        "the artifact counts the checks"
    );
    ensure_eq!(
        run_checks(
            &scratch,
            &status_text(&[(Name::of("clippy"), Count::new(101))])
        )?
        .0,
        ExitStatus::Failed,
        "one failure"
    );
    ensure_eq!(
        run_checks(&scratch, &Text::of("fmt zero\n"))?.0,
        ExitStatus::Failed,
        "garbage counts as failure"
    );
    ensure_eq!(
        run_checks(&scratch, &Text::of(""))?.0,
        ExitStatus::Failed,
        "no checks at all fails"
    );
    Ok(())
}

#[test]
fn every_expected_check_must_appear_exactly_once() {
    assert_eq!(
        check_every_expected_check_must_appear_exactly_once(),
        Ok(()),
        "every_expected_check_must_appear_exactly_once"
    );
}

/// The status text with `extra` after it.
fn status_then(extra: &Text) -> Text {
    Text::from_string(format!("{}{}", status_text(&[]).as_str(), extra.as_str()))
}

fn check_every_expected_check_must_appear_exactly_once() -> Result<(), Fail> {
    let scratch = Scratch::repository()?;
    ensure_eq!(
        run_checks(&scratch, &Text::of("fmt 0\nclippy 0\n"))?.0,
        ExitStatus::Failed,
        "a status file with only some of the checks fails, however green"
    );
    for dropped in EXPECTED_CHECKS {
        let without = Text::from_string(
            status_text(&[])
                .as_str()
                .lines()
                .filter(|line| !line.starts_with(&format!("{dropped} ")))
                .map(|line| format!("{line}\n"))
                .collect::<Vec<String>>()
                .concat(),
        );
        ensure_eq!(
            run_checks(&scratch, &without)?.0,
            ExitStatus::Failed,
            "{dropped} missing fails"
        );
    }
    for (extra, why) in [
        ("fmt 0\n", "a check twice"),
        ("extra 0\n", "an unknown check"),
        ("\nnot a line\n", "a garbled extra line"),
    ] {
        ensure_eq!(
            run_checks(&scratch, &status_then(&Text::of(extra)))?.0,
            ExitStatus::Failed,
            "{why} fails"
        );
    }
    ensure!(
        run_checks_missing_file().is_err(),
        "an unreadable status file is an error, not a clean result"
    );
    Ok(())
}

fn run_checks_missing_file() -> Result<(), Fail> {
    let scratch = Scratch::repository()?;
    let report = checks(&ChecksRun {
        commits: commits(),
        out: scratch.path().join("o.txt"),
        status_file: scratch.path().join("nothing.txt"),
    })?;
    ensure_eq!(report.status, ExitStatus::Clean, "unreachable");
    Ok(())
}
