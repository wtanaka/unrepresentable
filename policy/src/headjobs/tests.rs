use super::{checks, mutants, ChecksRun, MutantsRun, EXPECTED_CHECKS};
use crate::artifacts::parse;
use crate::bytes::Bytes;
use crate::exit::{ExitStatus, Report};
use crate::io;
use crate::reason::{Code, Commits, Job};
use crate::testkit::{Fail, Scratch};
use crate::types::{Count, Name, RelPath, Text};

fn commits() -> Commits {
    Commits {
        base: sha!('b'),
        head: sha!('a'),
    }
}

fn status(report: &Report) -> ExitStatus {
    report.status
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
    ensure!(
        run_mutants(&absent, Count::new(0)).is_err(),
        "a missing output directory is an error"
    );
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
