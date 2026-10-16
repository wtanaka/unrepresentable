//! `--against`/`--range` over a real git repository: which changes need a
//! human (exit 3), which are findings (exit 1), which are free (exit 0), and
//! which are errors (exit 2).
use crate::support::{Answer, Edit, Exit, Fail, Flag, Message, Name, Outcome, RelPath, Sandbox};

macro_rules! put {
    ($sandbox:expr_2021, $name:literal, $text:literal) => {
        $sandbox.write(&RelPath::of($name), &Message::of($text))
    };
}

macro_rules! rel {
    ($text:literal) => {
        RelPath::of($text)
    };
}

macro_rules! msg {
    ($text:expr_2021) => {
        Message::of($text)
    };
}

/// Default configuration of every project here: code under `src/`, judge
/// files under `policy/`, new fixtures under `policy/fixtures/` are free.
const BASE_CONFIG: &str = "baseline = \"forbidden\"
default-range = \"main...HEAD\"
code = [\"src/**\"]
judge = [\"policy/**\"]
append-only = [\"policy/fixtures/**\"]

[member.fx]
role = \"pure\"
";

/// A committed project on `main`, with the working branch `work` checked out.
fn project(name: &Name) -> Result<Sandbox, Fail> {
    let package = Name::of("fx");
    let sandbox = Sandbox::create(name)?;
    sandbox.seed(&package, &Message::of(BASE_CONFIG))?;
    put!(sandbox, "src/main.rs", "fn main() {}\n")?;
    put!(sandbox, "policy/rules.txt", "rule one\n")?;
    put!(sandbox, "policy/fixtures/a.txt", "fixture a\n")?;
    put!(sandbox, "policy/r\u{e9}sum\u{e9}.txt", "caf\u{e9}\n")?;
    put!(sandbox, ".github/workflows/ci.yml", "name: ci\n")?;
    put!(sandbox, "ci/unrepresentable.sh", "#!/bin/sh\n")?;
    put!(sandbox, "CODEOWNERS", "* @owner\n")?;
    put!(sandbox, ".cargo/config.toml", "[net]\nretry = 1\n")?;
    put!(sandbox, "notes/readme.txt", "notes\n")?;
    sandbox.init_repo()?;
    sandbox.git(&Message::of("checkout -q -b work"))?;
    Ok(sandbox)
}

fn run(sandbox: &Sandbox) -> Result<Outcome, Fail> {
    sandbox.run(&[Flag::Against(Name::of("main"))])
}

/// Commits the pending edits on `work` and runs against `main`.
fn committed(sandbox: &Sandbox) -> Result<Outcome, Fail> {
    sandbox.commit_all(&Name::of("change"))?;
    run(sandbox)
}

#[test]
fn an_unchanged_branch_is_clean() {
    assert_eq!(
        check_an_unchanged_branch_is_clean(),
        Ok(()),
        "an_unchanged_branch_is_clean"
    );
}

fn check_an_unchanged_branch_is_clean() -> Result<(), Fail> {
    let sandbox = project(&Name::of("against-unchanged"))?;
    let outcome = run(&sandbox)?;
    ensure_eq!(outcome.exit, Exit::Clean, "nothing changed");
    ensure_eq!(outcome.approvals(), Vec::<Message>::new(), "no approvals");
    Ok(())
}

#[test]
fn code_only_changes_are_clean() {
    assert_eq!(
        check_code_only_changes_are_clean(),
        Ok(()),
        "code_only_changes_are_clean"
    );
}

fn check_code_only_changes_are_clean() -> Result<(), Fail> {
    let sandbox = project(&Name::of("against-code-only"))?;
    put!(sandbox, "src/main.rs", "fn main() {}\nfn extra() {}\n")?;
    let outcome = committed(&sandbox)?;
    ensure_eq!(outcome.exit, Exit::Clean, "code alone needs nobody");
    ensure_eq!(outcome.approvals(), Vec::<Message>::new(), "no approvals");
    Ok(())
}

#[test]
fn a_workflow_change_alone_needs_approval() {
    assert_eq!(
        check_a_workflow_change_alone_needs_approval(),
        Ok(()),
        "a_workflow_change_alone_needs_approval"
    );
}

fn check_a_workflow_change_alone_needs_approval() -> Result<(), Fail> {
    let sandbox = project(&Name::of("against-workflow"))?;
    put!(sandbox, ".github/workflows/ci.yml", "name: ci\non: push\n")?;
    let outcome = committed(&sandbox)?;
    ensure_eq!(outcome.exit, Exit::Approval, "workflow edits go to a human");
    ensure!(
        outcome.has_approval(&msg!("WORKFLOW_CHANGED .github/workflows/ci.yml")) == Answer::Yes,
        "the workflow is named: {:?}",
        outcome.approvals()
    );
    Ok(())
}

#[test]
fn protected_files_need_approval() {
    assert_eq!(
        check_protected_files_need_approval(),
        Ok(()),
        "protected_files_need_approval"
    );
}

fn check_protected_files_need_approval() -> Result<(), Fail> {
    let sandbox = project(&Name::of("against-protected"))?;
    put!(
        sandbox,
        "ci/unrepresentable.sh",
        "#!/bin/sh\necho changed\n"
    )?;
    put!(sandbox, "CODEOWNERS", "* @someone-else\n")?;
    let outcome = committed(&sandbox)?;
    ensure_eq!(
        outcome.exit,
        Exit::Approval,
        "protected files go to a human"
    );
    ensure!(
        outcome.has_approval(&msg!("PROTECTED_CHANGED ci/unrepresentable.sh")) == Answer::Yes,
        "the script is protected: {:?}",
        outcome.approvals()
    );
    ensure!(
        outcome.has_approval(&msg!("PROTECTED_CHANGED CODEOWNERS")) == Answer::Yes,
        "CODEOWNERS is protected: {:?}",
        outcome.approvals()
    );
    Ok(())
}

#[test]
fn editing_an_existing_fixture_needs_approval() {
    assert_eq!(
        check_editing_an_existing_fixture_needs_approval(),
        Ok(()),
        "editing_an_existing_fixture_needs_approval"
    );
}

fn check_editing_an_existing_fixture_needs_approval() -> Result<(), Fail> {
    let sandbox = project(&Name::of("against-fixture-edit"))?;
    put!(sandbox, "policy/fixtures/a.txt", "fixture a, edited\n")?;
    let outcome = committed(&sandbox)?;
    ensure_eq!(outcome.exit, Exit::Approval, "fixtures are append-only");
    ensure!(
        outcome.has_approval(&msg!("JUDGE_MODIFIED policy/fixtures/a.txt")) == Answer::Yes,
        "the edited fixture is named: {:?}",
        outcome.approvals()
    );
    Ok(())
}

#[test]
fn deleting_a_fixture_needs_approval() {
    assert_eq!(
        check_deleting_a_fixture_needs_approval(),
        Ok(()),
        "deleting_a_fixture_needs_approval"
    );
}

fn check_deleting_a_fixture_needs_approval() -> Result<(), Fail> {
    let sandbox = project(&Name::of("against-fixture-delete"))?;
    sandbox.remove(&rel!("policy/fixtures/a.txt"))?;
    let outcome = committed(&sandbox)?;
    ensure_eq!(
        outcome.exit,
        Exit::Approval,
        "removing a fixture is not append-only"
    );
    ensure!(
        outcome.has_approval(&msg!("JUDGE_CHANGED policy/fixtures/a.txt (deleted)")) == Answer::Yes,
        "the deleted fixture is named: {:?}",
        outcome.approvals()
    );
    Ok(())
}

#[test]
fn a_new_fixture_beside_a_code_change_is_free() {
    assert_eq!(
        check_a_new_fixture_beside_a_code_change_is_free(),
        Ok(()),
        "a_new_fixture_beside_a_code_change_is_free"
    );
}

fn check_a_new_fixture_beside_a_code_change_is_free() -> Result<(), Fail> {
    let sandbox = project(&Name::of("against-new-fixture"))?;
    put!(sandbox, "policy/fixtures/b.txt", "fixture b\n")?;
    put!(sandbox, "src/main.rs", "fn main() {}\nfn extra() {}\n")?;
    let outcome = committed(&sandbox)?;
    ensure_eq!(
        outcome.exit,
        Exit::Clean,
        "appended fixtures pair with code"
    );
    ensure_eq!(outcome.approvals(), Vec::<Message>::new(), "no approvals");
    ensure_eq!(
        outcome.findings(),
        Vec::<Message>::new(),
        "no pairing finding"
    );
    Ok(())
}

#[test]
fn a_new_judge_file_that_is_not_append_only_needs_approval() {
    assert_eq!(
        check_a_new_judge_file_that_is_not_append_only_needs_approval(),
        Ok(()),
        "a_new_judge_file_that_is_not_append_only_needs_approval"
    );
}

fn check_a_new_judge_file_that_is_not_append_only_needs_approval() -> Result<(), Fail> {
    let sandbox = project(&Name::of("against-new-judge"))?;
    put!(sandbox, "policy/new-rule.txt", "rule two\n")?;
    let outcome = committed(&sandbox)?;
    ensure_eq!(outcome.exit, Exit::Approval, "new judge files need a human");
    ensure!(
        outcome.has_approval(&msg!("JUDGE_CHANGED policy/new-rule.txt (added)")) == Answer::Yes,
        "the new file is named: {:?}",
        outcome.approvals()
    );
    Ok(())
}

#[test]
fn judge_and_code_in_one_change_is_a_finding() {
    assert_eq!(
        check_judge_and_code_in_one_change_is_a_finding(),
        Ok(()),
        "judge_and_code_in_one_change_is_a_finding"
    );
}

fn check_judge_and_code_in_one_change_is_a_finding() -> Result<(), Fail> {
    let sandbox = project(&Name::of("against-pairing"))?;
    put!(sandbox, "policy/rules.txt", "rule one, changed\n")?;
    put!(sandbox, "src/main.rs", "fn main() {}\nfn extra() {}\n")?;
    let outcome = committed(&sandbox)?;
    ensure_eq!(outcome.exit, Exit::Findings, "mixed changes are findings");
    ensure!(
        outcome.has_rule_in(
            &rel!(".unrepresentable/config"),
            &Name::of("judge-code-pairing")
        ) == Answer::Yes,
        "the pairing finding is reported: {:?}",
        outcome.findings()
    );
    Ok(())
}

#[test]
fn renaming_a_judge_file_out_of_the_glob_needs_approval() {
    assert_eq!(
        check_renaming_a_judge_file_out_of_the_glob_needs_approval(),
        Ok(()),
        "renaming_a_judge_file_out_of_the_glob_needs_approval"
    );
}

fn check_renaming_a_judge_file_out_of_the_glob_needs_approval() -> Result<(), Fail> {
    let sandbox = project(&Name::of("against-judge-rename"))?;
    sandbox.git(&Message::of("mv policy/rules.txt notes/rules.txt"))?;
    let outcome = committed(&sandbox)?;
    ensure_eq!(
        outcome.exit,
        Exit::Approval,
        "moving a judge file out is a change to it"
    );
    ensure!(
        outcome.has_approval(&msg!("JUDGE_CHANGED policy/rules.txt (deleted)")) == Answer::Yes,
        "the old path is seen: {:?}",
        outcome.approvals()
    );
    Ok(())
}

#[test]
fn judge_paths_with_non_ascii_names_are_seen() {
    assert_eq!(
        check_judge_paths_with_non_ascii_names_are_seen(),
        Ok(()),
        "judge_paths_with_non_ascii_names_are_seen"
    );
}

fn check_judge_paths_with_non_ascii_names_are_seen() -> Result<(), Fail> {
    let sandbox = project(&Name::of("against-non-ascii"))?;
    put!(
        sandbox,
        "policy/r\u{e9}sum\u{e9}.txt",
        "caf\u{e9}, changed\n"
    )?;
    put!(sandbox, "policy/na\u{ef}ve.txt", "new\n")?;
    let outcome = committed(&sandbox)?;
    ensure_eq!(
        outcome.exit,
        Exit::Approval,
        "non-ASCII judge paths need approval"
    );
    ensure!(
        outcome.has_approval(&msg!("JUDGE_MODIFIED policy/r\u{e9}sum\u{e9}.txt")) == Answer::Yes,
        "the modified file is named unquoted: {:?}",
        outcome.approvals()
    );
    ensure!(
        outcome.has_approval(&msg!("JUDGE_CHANGED policy/na\u{ef}ve.txt (added)")) == Answer::Yes,
        "the added file is named unquoted: {:?}",
        outcome.approvals()
    );
    Ok(())
}

#[test]
fn an_untracked_code_file_plus_a_staged_judge_file_are_one_change() {
    assert_eq!(
        check_an_untracked_code_file_plus_a_staged_judge_file_are_one_change(),
        Ok(()),
        "an_untracked_code_file_plus_a_staged_judge_file_are_one_change"
    );
}

fn check_an_untracked_code_file_plus_a_staged_judge_file_are_one_change() -> Result<(), Fail> {
    let sandbox = project(&Name::of("against-untracked-staged"))?;
    put!(sandbox, "src/data.txt", "untracked code-side file\n")?;
    put!(sandbox, "policy/rules.txt", "rule one, staged\n")?;
    sandbox.git(&Message::of("add policy/rules.txt"))?;
    let outcome = run(&sandbox)?;
    ensure_eq!(
        outcome.exit,
        Exit::Findings,
        "the union of changes mixes judge and code"
    );
    ensure!(
        outcome.has_rule_in(
            &rel!(".unrepresentable/config"),
            &Name::of("judge-code-pairing")
        ) == Answer::Yes,
        "the pairing finding is reported: {:?}",
        outcome.findings()
    );
    Ok(())
}

#[test]
fn an_untracked_code_file_alone_is_clean() {
    assert_eq!(
        check_an_untracked_code_file_alone_is_clean(),
        Ok(()),
        "an_untracked_code_file_alone_is_clean"
    );
}

fn check_an_untracked_code_file_alone_is_clean() -> Result<(), Fail> {
    let sandbox = project(&Name::of("against-untracked-only"))?;
    put!(sandbox, "src/data.txt", "untracked code-side file\n")?;
    let outcome = run(&sandbox)?;
    ensure_eq!(
        outcome.exit,
        Exit::Clean,
        "code alone is free, tracked or not"
    );
    Ok(())
}

#[test]
fn an_untracked_judge_file_is_seen() {
    assert_eq!(
        check_an_untracked_judge_file_is_seen(),
        Ok(()),
        "an_untracked_judge_file_is_seen"
    );
}

fn check_an_untracked_judge_file_is_seen() -> Result<(), Fail> {
    let sandbox = project(&Name::of("against-untracked-judge"))?;
    put!(sandbox, "policy/untracked.txt", "never added to git\n")?;
    let outcome = run(&sandbox)?;
    ensure_eq!(outcome.exit, Exit::Approval, "untracked judge files count");
    ensure!(
        outcome.has_approval(&msg!("JUDGE_CHANGED policy/untracked.txt (added)")) == Answer::Yes,
        "the untracked file is named: {:?}",
        outcome.approvals()
    );
    Ok(())
}

#[test]
fn a_detached_head_with_working_tree_changes_is_seen() {
    assert_eq!(
        check_a_detached_head_with_working_tree_changes_is_seen(),
        Ok(()),
        "a_detached_head_with_working_tree_changes_is_seen"
    );
}

fn check_a_detached_head_with_working_tree_changes_is_seen() -> Result<(), Fail> {
    let sandbox = project(&Name::of("against-detached"))?;
    sandbox.git(&Message::of("checkout -q --detach"))?;
    put!(
        sandbox,
        "policy/rules.txt",
        "rule one, edited in the working tree\n"
    )?;
    let outcome = run(&sandbox)?;
    ensure_eq!(
        outcome.exit,
        Exit::Approval,
        "uncommitted judge edits are seen"
    );
    ensure!(
        outcome.has_approval(&msg!("JUDGE_MODIFIED policy/rules.txt")) == Answer::Yes,
        "the edited file is named: {:?}",
        outcome.approvals()
    );
    Ok(())
}

#[test]
fn a_jj_colocated_checkout_is_handled() {
    assert_eq!(
        check_a_jj_colocated_checkout_is_handled(),
        Ok(()),
        "a_jj_colocated_checkout_is_handled"
    );
}

fn check_a_jj_colocated_checkout_is_handled() -> Result<(), Fail> {
    let sandbox = project(&Name::of("against-jj-colocated"))?;
    sandbox.git(&Message::of("checkout -q --detach"))?;
    put!(sandbox, ".jj/.gitignore", "/*\n")?;
    put!(sandbox, ".jj/repo/store/clippy.toml", "stray = true\n")?;
    put!(
        sandbox,
        ".jj/working_copy/orphan.rs",
        "fn orphan() { panic!() }\n"
    )?;
    put!(
        sandbox,
        "policy/rules.txt",
        "rule one, edited in the jj working copy\n"
    )?;
    let outcome = run(&sandbox)?;
    ensure_eq!(
        outcome.exit,
        Exit::Approval,
        "the jj working copy edit is seen"
    );
    ensure!(
        outcome.has_approval(&msg!("JUDGE_MODIFIED policy/rules.txt")) == Answer::Yes,
        "the edited file is named: {:?}",
        outcome.approvals()
    );
    ensure_eq!(
        outcome.findings(),
        Vec::<Message>::new(),
        "nothing inside .jj is scanned or reported"
    );
    Ok(())
}

#[test]
fn a_missing_ref_is_an_error() {
    assert_eq!(
        check_a_missing_ref_is_an_error(),
        Ok(()),
        "a_missing_ref_is_an_error"
    );
}

fn check_a_missing_ref_is_an_error() -> Result<(), Fail> {
    let sandbox = project(&Name::of("against-missing-ref"))?;
    let outcome = sandbox.run(&[Flag::Against(Name::of("no-such-ref"))])?;
    ensure_eq!(outcome.exit, Exit::Error, "an unknown ref exits 2");
    ensure!(
        outcome.stderr().as_str().contains("git"),
        "the error names git: {:?}",
        outcome.stderr()
    );
    Ok(())
}

#[test]
fn a_missing_range_ref_is_an_error() {
    assert_eq!(
        check_a_missing_range_ref_is_an_error(),
        Ok(()),
        "a_missing_range_ref_is_an_error"
    );
}

fn check_a_missing_range_ref_is_an_error() -> Result<(), Fail> {
    let sandbox = project(&Name::of("against-missing-range"))?;
    let outcome = sandbox.run(&[Flag::Range(Message::of("no-such-ref..HEAD"))])?;
    ensure_eq!(outcome.exit, Exit::Error, "an unknown range start exits 2");
    Ok(())
}

#[test]
fn a_shallow_clone_is_an_error() {
    assert_eq!(
        check_a_shallow_clone_is_an_error(),
        Ok(()),
        "a_shallow_clone_is_an_error"
    );
}

fn check_a_shallow_clone_is_an_error() -> Result<(), Fail> {
    let origin = project(&Name::of("against-shallow-origin"))?;
    put!(origin, "src/main.rs", "fn main() {}\nfn extra() {}\n")?;
    origin.commit_all(&Name::of("second"))?;
    let clone = origin.shallow_clone(&Name::of("against-shallow-clone"))?;
    let outcome = clone.run(&[Flag::Range(Message::of("HEAD"))])?;
    ensure_eq!(
        outcome.exit,
        Exit::Error,
        "a shallow clone cannot prove the merge base"
    );
    ensure!(
        outcome.stderr().as_str().contains("shallow"),
        "the error says why: {:?}",
        outcome.stderr()
    );
    Ok(())
}

#[test]
fn the_configured_default_range_is_used() {
    assert_eq!(
        check_the_configured_default_range_is_used(),
        Ok(()),
        "the_configured_default_range_is_used"
    );
}

fn check_the_configured_default_range_is_used() -> Result<(), Fail> {
    let sandbox = project(&Name::of("against-default-range"))?;
    put!(sandbox, ".github/workflows/ci.yml", "name: ci\non: push\n")?;
    sandbox.commit_all(&Name::of("change"))?;
    let outcome = sandbox.run(&[])?;
    ensure_eq!(
        outcome.exit,
        Exit::Approval,
        "main...HEAD from the config sees the change"
    );
    ensure!(
        outcome.stdout().as_str().starts_with("range: main...HEAD"),
        "the range is printed: {:?}",
        outcome.stdout()
    );
    Ok(())
}

#[test]
fn no_range_anywhere_is_a_usage_error() {
    assert_eq!(
        check_no_range_anywhere_is_a_usage_error(),
        Ok(()),
        "no_range_anywhere_is_a_usage_error"
    );
}

fn check_no_range_anywhere_is_a_usage_error() -> Result<(), Fail> {
    let sandbox = project(&Name::of("against-no-range"))?;
    sandbox.edit(&Edit {
        file: RelPath::of(".unrepresentable/config"),
        from: Message::of("default-range = \"main...HEAD\"\n"),
        to: Message::of(""),
    })?;
    sandbox.commit_all(&Name::of("change"))?;
    let outcome = sandbox.run(&[])?;
    ensure_eq!(outcome.exit, Exit::Error, "no range exits 2");
    Ok(())
}

#[test]
fn toolchain_dependency_and_ignore_changes_need_approval() {
    assert_eq!(
        check_toolchain_dependency_and_ignore_changes_need_approval(),
        Ok(()),
        "toolchain_dependency_and_ignore_changes_need_approval"
    );
}

fn check_toolchain_dependency_and_ignore_changes_need_approval() -> Result<(), Fail> {
    let sandbox = project(&Name::of("against-misc-files"))?;
    sandbox.append(&rel!("rust-toolchain.toml"), &Message::of("# touched\n"))?;
    sandbox.append(&rel!("Cargo.lock"), &Message::of("# touched\n"))?;
    sandbox.append(&rel!(".gitignore"), &Message::of("*.log\n"))?;
    put!(sandbox, ".cargo/config.toml", "[net]\nretry = 2\n")?;
    let outcome = committed(&sandbox)?;
    ensure_eq!(outcome.exit, Exit::Approval, "these files are judge files");
    for expected in [
        "TOOLCHAIN_CHANGED rust-toolchain.toml",
        "DEPENDENCY_CHANGED Cargo.lock",
        "GITIGNORE_CHANGED .gitignore",
        "CARGO_CONFIG_CHANGED .cargo/config.toml",
    ] {
        ensure!(
            outcome.has_approval(&msg!(expected)) == Answer::Yes,
            "{expected} is reported: {:?}",
            outcome.approvals()
        );
    }
    Ok(())
}

#[test]
fn deny_toml_is_a_judge_path_at_any_depth() {
    assert_eq!(
        check_deny_toml_is_a_judge_path_at_any_depth(),
        Ok(()),
        "deny_toml_is_a_judge_path_at_any_depth"
    );
}

fn check_deny_toml_is_a_judge_path_at_any_depth() -> Result<(), Fail> {
    let sandbox = project(&Name::of("against-deny-toml"))?;
    put!(
        sandbox,
        "deny.toml",
        "[bans]\nmultiple-versions = \"deny\"\n"
    )?;
    put!(
        sandbox,
        "crates/a/.deny.toml",
        "[licenses]\nallow = [\"MIT\"]\n"
    )?;
    let added = committed(&sandbox)?;
    ensure_eq!(added.exit, Exit::Approval, "new deny files need a human");
    for expected in [
        "JUDGE_CHANGED deny.toml (added)",
        "JUDGE_CHANGED crates/a/.deny.toml (added)",
    ] {
        ensure!(
            added.has_approval(&msg!(expected)) == Answer::Yes,
            "{expected} is reported: {:?}",
            added.approvals()
        );
    }
    sandbox.git(&Message::of("branch -f main HEAD"))?;
    put!(
        sandbox,
        "deny.toml",
        "[bans]\nmultiple-versions = \"warn\"\n"
    )?;
    put!(
        sandbox,
        "crates/a/.deny.toml",
        "[licenses]\nallow = [\"MIT\", \"GPL-3.0\"]\n"
    )?;
    let modified = committed(&sandbox)?;
    ensure_eq!(
        modified.exit,
        Exit::Approval,
        "loosening deny files needs a human"
    );
    for expected in [
        "DENY_CONFIG_CHANGED deny.toml",
        "DENY_CONFIG_CHANGED crates/a/.deny.toml",
    ] {
        ensure!(
            modified.has_approval(&msg!(expected)) == Answer::Yes,
            "{expected} is reported: {:?}",
            modified.approvals()
        );
    }
    Ok(())
}

#[test]
fn deny_toml_beside_a_code_change_is_a_finding() {
    assert_eq!(
        check_deny_toml_beside_a_code_change_is_a_finding(),
        Ok(()),
        "deny_toml_beside_a_code_change_is_a_finding"
    );
}

fn check_deny_toml_beside_a_code_change_is_a_finding() -> Result<(), Fail> {
    let sandbox = project(&Name::of("against-deny-pairing"))?;
    put!(
        sandbox,
        "deny.toml",
        "[bans]\nmultiple-versions = \"deny\"\n"
    )?;
    put!(sandbox, "src/main.rs", "fn main() {}\nfn extra() {}\n")?;
    let outcome = committed(&sandbox)?;
    ensure_eq!(outcome.exit, Exit::Findings, "mixed changes are findings");
    ensure!(
        outcome.has_rule_in(
            &rel!(".unrepresentable/config"),
            &Name::of("judge-code-pairing")
        ) == Answer::Yes,
        "deny.toml with code is a pairing finding: {:?}",
        outcome.findings()
    );
    Ok(())
}

#[test]
fn root_manifest_changes_outside_the_lint_tables_need_approval() {
    assert_eq!(
        check_root_manifest_changes_outside_the_lint_tables_need_approval(),
        Ok(()),
        "root_manifest_changes_outside_the_lint_tables_need_approval"
    );
}

fn check_root_manifest_changes_outside_the_lint_tables_need_approval() -> Result<(), Fail> {
    let sandbox = project(&Name::of("against-manifest"))?;
    sandbox.edit(&Edit {
        file: RelPath::of("Cargo.toml"),
        from: Message::of("edition = \"2021\"\n"),
        to: Message::of("edition = \"2021\"\ndescription = \"changed\"\n"),
    })?;
    let outcome = committed(&sandbox)?;
    ensure_eq!(outcome.exit, Exit::Approval, "manifest edits need a human");
    ensure!(
        outcome.has_approval(&msg!("ROOT_MANIFEST_CHANGED Cargo.toml")) == Answer::Yes,
        "the manifest change is reported: {:?}",
        outcome.approvals()
    );
    Ok(())
}

#[test]
fn removing_and_lowering_lints_is_escalated_and_reported() {
    assert_eq!(
        check_removing_and_lowering_lints_is_escalated_and_reported(),
        Ok(()),
        "removing_and_lowering_lints_is_escalated_and_reported"
    );
}

fn check_removing_and_lowering_lints_is_escalated_and_reported() -> Result<(), Fail> {
    let sandbox = project(&Name::of("against-lint-tables"))?;
    sandbox.edit(&Edit {
        file: RelPath::of("Cargo.toml"),
        from: Message::of("dead_code = \"deny\"\n"),
        to: Message::of(""),
    })?;
    sandbox.edit(&Edit {
        file: RelPath::of("Cargo.toml"),
        from: Message::of("unused_mut = \"deny\""),
        to: Message::of("unused_mut = \"warn\""),
    })?;
    sandbox.edit(&Edit {
        file: RelPath::of("Cargo.toml"),
        from: Message::of("cargo = { level = \"deny\", priority = -1 }"),
        to: Message::of("cargo = { level = \"deny\", priority = -2 }"),
    })?;
    let outcome = committed(&sandbox)?;
    ensure_eq!(
        outcome.exit,
        Exit::Findings,
        "uncovered lints are also findings"
    );
    ensure!(
        outcome.has_approval(&msg!("LINT_REMOVED dead_code")) == Answer::Yes,
        "the removal is reported: {:?}",
        outcome.approvals()
    );
    ensure!(
        outcome.has_approval(&msg!("LINT_LOWERED unused_mut")) == Answer::Yes,
        "the lowering is reported: {:?}",
        outcome.approvals()
    );
    ensure!(
        outcome.has_approval(&msg!("LINT_PRIORITY_CHANGED clippy::cargo")) == Answer::Yes,
        "the priority change is reported: {:?}",
        outcome.approvals()
    );
    Ok(())
}

#[test]
fn loosening_clippy_toml_is_escalated() {
    assert_eq!(
        check_loosening_clippy_toml_is_escalated(),
        Ok(()),
        "loosening_clippy_toml_is_escalated"
    );
}

fn check_loosening_clippy_toml_is_escalated() -> Result<(), Fail> {
    let sandbox = project(&Name::of("against-clippy-toml"))?;
    sandbox.edit(&Edit {
        file: RelPath::of("clippy.toml"),
        from: Message::of("max-fn-params-bools = 0"),
        to: Message::of("max-fn-params-bools = 3"),
    })?;
    let outcome = committed(&sandbox)?;
    ensure!(
        outcome.has_approval(&msg!("CLIPPY_KNOB_LOOSENED max-fn-params-bools")) == Answer::Yes,
        "the knob is named: {:?}",
        outcome.approvals()
    );
    Ok(())
}

#[test]
fn removing_a_disallowed_entry_from_clippy_toml_is_escalated() {
    assert_eq!(
        check_removing_a_disallowed_entry_from_clippy_toml_is_escalated(),
        Ok(()),
        "removing_a_disallowed_entry_from_clippy_toml_is_escalated"
    );
}

fn check_removing_a_disallowed_entry_from_clippy_toml_is_escalated() -> Result<(), Fail> {
    let package = Name::of("fx");
    let sandbox = Sandbox::create(&Name::of("against-disallowed"))?;
    sandbox.seed(&package, &Message::of(BASE_CONFIG))?;
    put!(sandbox, "src/main.rs", "fn main() {}\n")?;
    sandbox.append(
        &rel!("clippy.toml"),
        &Message::of("disallowed-methods = [\"std::mem::forget\"]\n"),
    )?;
    sandbox.init_repo()?;
    sandbox.git(&Message::of("checkout -q -b work"))?;
    sandbox.edit(&Edit {
        file: RelPath::of("clippy.toml"),
        from: Message::of("disallowed-methods = [\"std::mem::forget\"]\n"),
        to: Message::of(""),
    })?;
    let outcome = committed(&sandbox)?;
    ensure!(
        outcome.has_approval(&msg!(
            "DISALLOWED_REMOVED disallowed-methods std::mem::forget"
        )) == Answer::Yes,
        "the removed entry is named: {:?}",
        outcome.approvals()
    );
    Ok(())
}

#[test]
fn tightening_clippy_toml_is_free() {
    assert_eq!(
        check_tightening_clippy_toml_is_free(),
        Ok(()),
        "tightening_clippy_toml_is_free"
    );
}

fn check_tightening_clippy_toml_is_free() -> Result<(), Fail> {
    let package = Name::of("fx");
    let sandbox = Sandbox::create(&Name::of("against-clippy-tighten"))?;
    sandbox.seed(&package, &Message::of(BASE_CONFIG))?;
    put!(sandbox, "src/main.rs", "fn main() {}\n")?;
    sandbox.append(
        &rel!("clippy.toml"),
        &Message::of("disallowed-methods = [\"std::mem::forget\"]\n"),
    )?;
    sandbox.init_repo()?;
    sandbox.git(&Message::of("checkout -q -b work"))?;
    sandbox.edit(&Edit {
        file: RelPath::of("clippy.toml"),
        from: Message::of("disallowed-methods = [\"std::mem::forget\"]"),
        to: Message::of("disallowed-methods = [\"std::mem::forget\", \"std::mem::swap\"]"),
    })?;
    let outcome = committed(&sandbox)?;
    ensure_eq!(
        outcome.exit,
        Exit::Clean,
        "adding a disallowed entry only tightens"
    );
    Ok(())
}

/// A project whose code carries an `#[expect]` that the ledger does not yet cover.
fn project_with_expect(name: &Name) -> Result<Sandbox, Fail> {
    let package = Name::of("fx");
    let sandbox = Sandbox::create(name)?;
    sandbox.seed(&package, &Message::of(BASE_CONFIG))?;
    put!(sandbox, "src/main.rs", "#[expect(dead_code, reason = \"kept for the ledger test\")]\nfn unused() {}\nfn main() {}\n")?;
    sandbox.init_repo()?;
    sandbox.git(&Message::of("checkout -q -b work"))?;
    Ok(sandbox)
}

#[test]
fn adding_an_expect_ledger_row_needs_approval() {
    assert_eq!(
        check_adding_an_expect_ledger_row_needs_approval(),
        Ok(()),
        "adding_an_expect_ledger_row_needs_approval"
    );
}

fn check_adding_an_expect_ledger_row_needs_approval() -> Result<(), Fail> {
    let sandbox = project_with_expect(&Name::of("against-ledger-row"))?;
    sandbox.edit(&Edit {
        file: RelPath::of(".unrepresentable/ledger.md"),
        from: Message::of("| --- | --- | --- | --- |\n"),
        to: Message::of(
            "| --- | --- | --- | --- |\n| src/main.rs | fn unused | dead_code | kept for the ledger test |\n",
        ),
    })?;
    let outcome = committed(&sandbox)?;
    ensure_eq!(
        outcome.exit,
        Exit::Approval,
        "a new ledger row needs a human"
    );
    ensure!(
        outcome.has_approval(&msg!("LEDGER_ROW_ADDED src/main.rs fn unused dead_code"))
            == Answer::Yes,
        "the row is named: {:?}",
        outcome.approvals()
    );
    ensure_eq!(
        outcome.findings(),
        Vec::<Message>::new(),
        "the row covers the expect"
    );
    Ok(())
}

#[test]
fn adding_an_omission_row_needs_approval() {
    assert_eq!(
        check_adding_an_omission_row_needs_approval(),
        Ok(()),
        "adding_an_omission_row_needs_approval"
    );
}

fn check_adding_an_omission_row_needs_approval() -> Result<(), Fail> {
    let sandbox = project(&Name::of("against-omission-row"))?;
    sandbox.edit(&Edit {
        file: RelPath::of("Cargo.toml"),
        from: Message::of("dead_code = \"deny\"\n"),
        to: Message::of(""),
    })?;
    sandbox.append(
        &rel!(".unrepresentable/ledger.md"),
        &Message::of("| `dead_code` | - | omitted for the test |\n"),
    )?;
    let outcome = committed(&sandbox)?;
    ensure!(
        outcome.has_approval(&msg!("OMISSION_ADDED dead_code")) == Answer::Yes,
        "the omission is named: {:?}",
        outcome.approvals()
    );
    Ok(())
}

#[test]
fn adding_a_canary_exemption_needs_approval() {
    assert_eq!(
        check_adding_a_canary_exemption_needs_approval(),
        Ok(()),
        "adding_a_canary_exemption_needs_approval"
    );
}

fn check_adding_a_canary_exemption_needs_approval() -> Result<(), Fail> {
    let sandbox = project(&Name::of("against-canary-exemption"))?;
    sandbox.edit(&Edit {
        file: RelPath::of(".unrepresentable/canary-exemptions.md"),
        from: Message::of("| --- | --- |\n"),
        to: Message::of("| --- | --- |\n| some_lint | cannot be triggered from a fixture |\n"),
    })?;
    let outcome = committed(&sandbox)?;
    ensure_eq!(
        outcome.exit,
        Exit::Approval,
        "a new exemption needs a human"
    );
    ensure!(
        outcome.has_approval(&msg!("CANARY_EXEMPTION_ADDED some_lint")) == Answer::Yes,
        "the exemption is named: {:?}",
        outcome.approvals()
    );
    Ok(())
}

#[test]
fn a_member_manifest_edit_needs_approval() {
    assert_eq!(
        check_a_member_manifest_edit_needs_approval(),
        Ok(()),
        "a_member_manifest_edit_needs_approval"
    );
}

fn check_a_member_manifest_edit_needs_approval() -> Result<(), Fail> {
    let sandbox = Sandbox::create(&Name::of("against-member-manifest"))?;
    sandbox.seed(&Name::of("fx"), &Message::of(BASE_CONFIG))?;
    put!(sandbox, "src/main.rs", "fn main() {}\n")?;
    put!(
        sandbox,
        "crates/inner/Cargo.toml",
        "[package]\nname = \"inner\"\nversion = \"0.1.0\"\n"
    )?;
    sandbox.init_repo()?;
    sandbox.git(&Message::of("checkout -q -b work"))?;
    sandbox.append(
        &rel!("crates/inner/Cargo.toml"),
        &Message::of("\n[dependencies]\nsome-crate = \"1\"\n"),
    )?;
    let outcome = committed(&sandbox)?;
    ensure_eq!(
        outcome.exit,
        Exit::Approval,
        "a member manifest edit needs a human"
    );
    ensure!(
        outcome.has_approval(&msg!("MEMBER_MANIFEST_CHANGED crates/inner/Cargo.toml"))
            == Answer::Yes,
        "the member manifest is named: {:?}",
        outcome.approvals()
    );
    Ok(())
}

#[test]
fn a_new_member_manifest_needs_approval() {
    assert_eq!(
        check_a_new_member_manifest_needs_approval(),
        Ok(()),
        "a_new_member_manifest_needs_approval"
    );
}

fn check_a_new_member_manifest_needs_approval() -> Result<(), Fail> {
    let sandbox = project(&Name::of("against-new-member-manifest"))?;
    put!(
        sandbox,
        "crates/extra/Cargo.toml",
        "[package]\nname = \"extra\"\nversion = \"0.1.0\"\n"
    )?;
    let outcome = committed(&sandbox)?;
    ensure!(
        outcome.has_approval(&msg!("JUDGE_CHANGED crates/extra/Cargo.toml (added)")) == Answer::Yes,
        "the new member manifest is named: {:?}",
        outcome.approvals()
    );
    Ok(())
}

#[test]
fn rewording_an_omission_reason_needs_approval() {
    assert_eq!(
        check_rewording_an_omission_reason_needs_approval(),
        Ok(()),
        "rewording_an_omission_reason_needs_approval"
    );
}

fn check_rewording_an_omission_reason_needs_approval() -> Result<(), Fail> {
    let sandbox = project(&Name::of("against-omission-reason"))?;
    sandbox.edit(&Edit {
        file: rel!(".unrepresentable/ledger.md"),
        from: Message::of("produces helpers with a single caller."),
        to: Message::of("is fine, nobody will mind."),
    })?;
    let outcome = committed(&sandbox)?;
    ensure_eq!(
        outcome.exit,
        Exit::Approval,
        "a reworded reason needs a human"
    );
    ensure!(
        outcome.has_approval(&msg!(
            "OMISSION_REASON_CHANGED clippy::single_call_fn clippy::too_many_lines"
        )) == Answer::Yes,
        "the reworded row is named: {:?}",
        outcome.approvals()
    );
    Ok(())
}

#[test]
fn rewording_an_expect_reason_needs_approval() {
    assert_eq!(
        check_rewording_an_expect_reason_needs_approval(),
        Ok(()),
        "rewording_an_expect_reason_needs_approval"
    );
}

fn check_rewording_an_expect_reason_needs_approval() -> Result<(), Fail> {
    let sandbox = project_with_expect(&Name::of("against-expect-reason"))?;
    sandbox.git(&Message::of("checkout -q main"))?;
    sandbox.edit(&Edit {
        file: rel!(".unrepresentable/ledger.md"),
        from: Message::of("| --- | --- | --- | --- |\n"),
        to: Message::of(
            "| --- | --- | --- | --- |\n| src/main.rs | fn unused | dead_code | kept for the ledger test |\n",
        ),
    })?;
    sandbox.commit_all(&Name::of("row"))?;
    sandbox.git(&Message::of("checkout -q work"))?;
    sandbox.git(&Message::of("merge -q main"))?;
    sandbox.edit(&Edit {
        file: rel!(".unrepresentable/ledger.md"),
        from: Message::of("kept for the ledger test |"),
        to: Message::of("a different story |"),
    })?;
    let outcome = committed(&sandbox)?;
    ensure_eq!(
        outcome.exit,
        Exit::Approval,
        "a reworded reason needs a human"
    );
    ensure!(
        outcome.has_approval(&msg!(
            "LEDGER_REASON_CHANGED src/main.rs fn unused dead_code"
        )) == Answer::Yes,
        "the reworded row is named: {:?}",
        outcome.approvals()
    );
    Ok(())
}

#[test]
fn rewording_a_canary_exemption_needs_approval() {
    assert_eq!(
        check_rewording_a_canary_exemption_needs_approval(),
        Ok(()),
        "rewording_a_canary_exemption_needs_approval"
    );
}

fn check_rewording_a_canary_exemption_needs_approval() -> Result<(), Fail> {
    let sandbox = project(&Name::of("against-exemption-reason"))?;
    sandbox.edit(&Edit {
        file: rel!(".unrepresentable/canary-exemptions.md"),
        from: Message::of("Needs an aarch64 softfloat target"),
        to: Message::of("Does not need anything special"),
    })?;
    let outcome = committed(&sandbox)?;
    ensure!(
        outcome.has_approval(&msg!(
            "CANARY_EXEMPTION_REASON_CHANGED aarch64_softfloat_neon"
        )) == Answer::Yes,
        "the reworded exemption is named: {:?}",
        outcome.approvals()
    );
    Ok(())
}

#[test]
fn a_default_range_rewritten_to_an_empty_range_is_refused() {
    assert_eq!(
        check_a_default_range_rewritten_to_an_empty_range_is_refused(),
        Ok(()),
        "a_default_range_rewritten_to_an_empty_range_is_refused"
    );
}

fn check_a_default_range_rewritten_to_an_empty_range_is_refused() -> Result<(), Fail> {
    let sandbox = project(&Name::of("against-range-empty"))?;
    sandbox.edit(&Edit {
        file: rel!(".unrepresentable/config"),
        from: msg!("default-range = \"main...HEAD\"\n"),
        to: msg!("default-range = \"HEAD\"\n"),
    })?;
    sandbox.commit_all(&Name::of("change"))?;
    let outcome = sandbox.run(&[])?;
    ensure_eq!(
        outcome.exit,
        Exit::Error,
        "a range the change wrote itself, selecting nothing, is not a pass: {:?}",
        outcome.stdout()
    );
    ensure!(
        outcome.stderr().as_str().contains("selects no changes"),
        "the error says why: {:?}",
        outcome.stderr()
    );
    Ok(())
}

#[test]
fn the_default_range_at_the_base_replaces_one_the_change_rewrote() {
    assert_eq!(
        check_the_default_range_at_the_base_replaces_one_the_change_rewrote(),
        Ok(()),
        "the_default_range_at_the_base_replaces_one_the_change_rewrote"
    );
}

fn check_the_default_range_at_the_base_replaces_one_the_change_rewrote() -> Result<(), Fail> {
    let sandbox = project(&Name::of("against-range-at-base"))?;
    sandbox.edit(&Edit {
        file: rel!(".unrepresentable/config"),
        from: msg!("default-range = \"main...HEAD\"\n"),
        to: msg!("default-range = \"main...main\"\n"),
    })?;
    sandbox.commit_all(&Name::of("change"))?;
    let outcome = sandbox.run(&[])?;
    ensure_eq!(
        outcome.exit,
        Exit::Approval,
        "the base's range sees the config edit: {:?} {:?}",
        outcome.stdout(),
        outcome.stderr()
    );
    ensure!(
        outcome.stdout().as_str().starts_with("range: main...HEAD"),
        "the range of the base is the one used: {:?}",
        outcome.stdout()
    );
    ensure!(
        outcome.has_approval(&msg!("CONFIG_CHANGED default-range")) == Answer::Yes,
        "the edit of the range is named: {:?}",
        outcome.approvals()
    );
    Ok(())
}

#[test]
fn removing_a_code_glob_while_editing_code_still_pairs() {
    assert_eq!(
        check_removing_a_code_glob_while_editing_code_still_pairs(),
        Ok(()),
        "removing_a_code_glob_while_editing_code_still_pairs"
    );
}

fn check_removing_a_code_glob_while_editing_code_still_pairs() -> Result<(), Fail> {
    let sandbox = project(&Name::of("against-code-glob-removed"))?;
    sandbox.edit(&Edit {
        file: rel!(".unrepresentable/config"),
        from: msg!("code = [\"src/**\"]\n"),
        to: msg!("code = [\"lib/**\"]\n"),
    })?;
    put!(sandbox, "src/main.rs", "fn main() {}\nfn extra() {}\n")?;
    let outcome = committed(&sandbox)?;
    ensure_eq!(
        outcome.exit,
        Exit::Findings,
        "the base config still says src is code: {:?}",
        outcome.stdout()
    );
    ensure!(
        outcome.has_rule_in(
            &rel!(".unrepresentable/config"),
            &Name::of("judge-code-pairing")
        ) == Answer::Yes,
        "judge and code paths in one change: {:?}",
        outcome.findings()
    );
    Ok(())
}

#[test]
fn an_append_only_glob_added_by_the_change_does_not_exempt_its_files() {
    assert_eq!(
        check_an_append_only_glob_added_by_the_change_does_not_exempt_its_files(),
        Ok(()),
        "an_append_only_glob_added_by_the_change_does_not_exempt_its_files"
    );
}

fn check_an_append_only_glob_added_by_the_change_does_not_exempt_its_files() -> Result<(), Fail> {
    let sandbox = project(&Name::of("against-append-only-added"))?;
    sandbox.edit(&Edit {
        file: rel!(".unrepresentable/config"),
        from: msg!("append-only = [\"policy/fixtures/**\"]\n"),
        to: msg!("append-only = [\"policy/fixtures/**\", \".github/**\"]\n"),
    })?;
    put!(sandbox, ".github/workflows/new.yml", "name: new\n")?;
    let outcome = committed(&sandbox)?;
    ensure_eq!(outcome.exit, Exit::Approval, "a human decides");
    ensure!(
        outcome.has_approval(&msg!("JUDGE_CHANGED .github/workflows/new.yml (added)"))
            == Answer::Yes,
        "the new workflow is still named: {:?}",
        outcome.approvals()
    );
    Ok(())
}

#[test]
fn codeowners_workflows_rustfmt_and_build_scripts_are_guarded() {
    assert_eq!(
        check_codeowners_workflows_rustfmt_and_build_scripts_are_guarded(),
        Ok(()),
        "codeowners_workflows_rustfmt_and_build_scripts_are_guarded"
    );
}

fn check_codeowners_workflows_rustfmt_and_build_scripts_are_guarded() -> Result<(), Fail> {
    let sandbox = project(&Name::of("against-guarded-paths"))?;
    put!(sandbox, "docs/CODEOWNERS", "* @owner\n")?;
    put!(sandbox, ".github/workflows/policy.yml", "name: policy\n")?;
    put!(sandbox, "rustfmt.toml", "max_width = 80\n")?;
    put!(sandbox, "tools/build.rs", "fn main() {}\n")?;
    let outcome = committed(&sandbox)?;
    // The stray `tools/build.rs` is also an `unreached-file` finding (exit 1),
    // which does not matter here: the approvals are printed either way.
    ensure!(
        outcome.exit != Exit::Error,
        "the run decides: {:?}",
        outcome.stderr()
    );
    for expected in [
        "PROTECTED_CHANGED docs/CODEOWNERS (added)",
        "PROTECTED_CHANGED .github/workflows/policy.yml (added)",
        "JUDGE_CHANGED rustfmt.toml (added)",
        "JUDGE_CHANGED tools/build.rs (added)",
    ] {
        ensure!(
            outcome.has_approval(&msg!(expected)) == Answer::Yes,
            "{expected} is named: {:?}",
            outcome.approvals()
        );
    }
    Ok(())
}

#[test]
fn deleting_a_disallowed_block_from_clippy_toml_needs_approval() {
    assert_eq!(
        check_deleting_a_disallowed_block_from_clippy_toml_needs_approval(),
        Ok(()),
        "deleting_a_disallowed_block_from_clippy_toml_needs_approval"
    );
}

fn check_deleting_a_disallowed_block_from_clippy_toml_needs_approval() -> Result<(), Fail> {
    let block = "\n[[disallowed-methods]]\npath = \"std::mem::drop\"\nreason = \"explicit drops hide ownership\"\n";
    let package = Name::of("fx");
    let sandbox = Sandbox::create(&Name::of("against-clippy-block"))?;
    sandbox.seed(&package, &Message::of(BASE_CONFIG))?;
    put!(sandbox, "src/main.rs", "fn main() {}\n")?;
    sandbox.append(&rel!("clippy.toml"), &msg!(block))?;
    sandbox.init_repo()?;
    sandbox.git(&Message::of("checkout -q -b work"))?;
    sandbox.edit(&Edit {
        file: rel!("clippy.toml"),
        from: msg!(block),
        to: msg!(""),
    })?;
    let outcome = committed(&sandbox)?;
    ensure_eq!(
        outcome.exit,
        Exit::Approval,
        "an array-of-tables entry is a ban like an inline one: {:?} {:?}",
        outcome.stdout(),
        outcome.stderr()
    );
    ensure!(
        outcome.has_approval(&msg!(
            "DISALLOWED_REMOVED disallowed-methods std::mem::drop"
        )) == Answer::Yes,
        "the removed method is named: {:?}",
        outcome.approvals()
    );
    Ok(())
}
