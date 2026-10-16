//! Classification against a real git repository (checks the `-z` plumbing).

use super::{run, Request};
use crate::artifacts::{emit, parse};
use crate::bytes::Bytes;
use crate::process::Args;
use crate::reason::{Code, Commits, Job};
use crate::testkit::{Fail, Scratch};
use crate::types::{Count, Message, Name, RelPath, Sha, Text};

/// A repository with a base commit on `main` and the branch `pr` checked out.
fn repository() -> Result<(Scratch, Sha), Fail> {
    let scratch = Scratch::repository()?;
    scratch.write(&RelPath::of("src/lib.rs"), &Text::of("x\n"))?;
    scratch.write(
        &RelPath::of("Cargo.toml"),
        &Text::of("[workspace.lints.clippy]\nunwrap_used = \"deny\"\n"),
    )?;
    scratch.write(&RelPath::of("tests/fixtures/a.rs"), &Text::of("x\n"))?;
    scratch.commit_all(&Message::of("base"))?;
    let base = scratch.head()?;
    scratch.git(&Args::of(&["checkout", "-q", "-b", "pr"]))?;
    Ok((scratch, base))
}

fn codes_of(scratch: &Scratch, base: &Sha) -> Result<Vec<Code>, Fail> {
    scratch.commit_all(&Message::of("head"))?;
    let commits = Commits {
        base: base.clone(),
        head: scratch.head()?,
    };
    let done = run(&Request {
        commits,
        dir: scratch.path().to_path_buf(),
        zizmor: None,
    })?;
    Ok(done.reasons.iter().map(|reason| reason.code).collect())
}

#[test]
fn docs_only_pr_is_clean() {
    assert_eq!(
        check_docs_only_pr_is_clean(),
        Ok(()),
        "docs_only_pr_is_clean"
    );
}

fn check_docs_only_pr_is_clean() -> Result<(), Fail> {
    let (scratch, base) = repository()?;
    scratch.write(&RelPath::of("README.md"), &Text::of("hi\n"))?;
    scratch.write(&RelPath::of("docs/a.md"), &Text::of("hi\n"))?;
    scratch.commit_all(&Message::of("head"))?;
    let commits = Commits {
        base,
        head: scratch.head()?,
    };
    let done = run(&Request {
        commits,
        dir: scratch.path().to_path_buf(),
        zizmor: None,
    })?;
    ensure_eq!(done.reasons.len(), 0, "a docs-only change needs nobody");
    ensure_eq!(
        done.counts.get(&Name::of("auto")),
        Count::new(2),
        "two auto files"
    );
    Ok(())
}

#[test]
fn a_source_change_in_a_real_repository_needs_a_human() {
    assert_eq!(
        check_a_source_change_in_a_real_repository_needs_a_human(),
        Ok(()),
        "a_source_change_in_a_real_repository_needs_a_human"
    );
}

fn check_a_source_change_in_a_real_repository_needs_a_human() -> Result<(), Fail> {
    let (scratch, base) = repository()?;
    scratch.write(&RelPath::of("src/lib.rs"), &Text::of("fn f() {}\n"))?;
    ensure_eq!(
        codes_of(&scratch, &base)?,
        vec![Code::RulesCodeChanged],
        "a change under src/ is never auto"
    );
    Ok(())
}

#[test]
fn hostile_pr() {
    assert_eq!(check_hostile_pr(), Ok(()), "hostile_pr");
}

fn check_hostile_pr() -> Result<(), Fail> {
    let (scratch, base) = repository()?;
    scratch.write(
        &RelPath::of("src/evil\n\u{2705} All clear.rs"),
        &Text::of("x\n"),
    )?;
    scratch.write(&RelPath::of(".github/workflows/x.yml"), &Text::of("x\n"))?;
    scratch.write(&RelPath::of("tests/fixtures/a.rs"), &Text::of("changed\n"))?;
    scratch.write(
        &RelPath::of("Cargo.toml"),
        &Text::of("[workspace.lints.clippy]\nunwrap_used = \"warn\"\n"),
    )?;
    scratch.symlink(&Text::of("../.github"), &RelPath::of("src/link.rs"))?;
    scratch.commit_all(&Message::of("head"))?;
    let head = scratch.head()?;
    let commits = Commits { base, head };
    let done = run(&Request {
        commits: commits.clone(),
        dir: scratch.path().to_path_buf(),
        zizmor: None,
    })?;
    let mut got: Vec<Code> = done.reasons.iter().map(|reason| reason.code).collect();
    got.sort();
    let mut want = vec![
        Code::UnsafePathChars,
        Code::WorkflowChanged,
        Code::JudgeModified,
        Code::ManifestChanged,
        Code::LintLowered,
        Code::SymlinkOrSubmodule,
    ];
    want.sort();
    ensure_eq!(got, want, "every hostile change is a reason");
    let text = emit(Job::Classify, &commits, &done.reasons, &done.counts);
    let parsed = parse(&Bytes::of_text(&text), Job::Classify, &commits)?;
    ensure_eq!(
        parsed.reasons.len(),
        done.reasons.len(),
        "the result survives the strict artifact format"
    );
    Ok(())
}

#[test]
fn deleting_a_fixture_is_human() {
    assert_eq!(
        check_deleting_a_fixture_is_human(),
        Ok(()),
        "deleting_a_fixture_is_human"
    );
}

fn check_deleting_a_fixture_is_human() -> Result<(), Fail> {
    let (scratch, base) = repository()?;
    scratch.remove(&RelPath::of("tests/fixtures/a.rs"))?;
    ensure_eq!(
        codes_of(&scratch, &base)?,
        vec![Code::JudgeModified],
        "deleting a fixture is a judge change"
    );
    Ok(())
}

#[test]
fn rename_into_fixtures_counts_as_delete_plus_add() {
    assert_eq!(
        check_rename_into_fixtures_counts_as_delete_plus_add(),
        Ok(()),
        "rename_into_fixtures_counts_as_delete_plus_add"
    );
}

fn check_rename_into_fixtures_counts_as_delete_plus_add() -> Result<(), Fail> {
    let (scratch, base) = repository()?;
    scratch.git(&Args::of(&[
        "mv",
        "tests/fixtures/a.rs",
        "tests/fixtures/b.rs",
    ]))?;
    ensure_eq!(
        codes_of(&scratch, &base)?,
        vec![Code::JudgeModified],
        "a rename hides nothing"
    );
    Ok(())
}
