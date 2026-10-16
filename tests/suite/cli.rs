//! Command-line behaviour that is not about any one rule: usage errors and `init`.

use crate::support::{Answer, Exit, Fail, Flag, Message, Name, RelPath, Sandbox};

macro_rules! msg {
    ($text:expr_2021) => {
        Message::of($text)
    };
}

fn workspace(name: &Name) -> Result<Sandbox, Fail> {
    let package = Name::of("fx");
    let sandbox = Sandbox::create(name)?;
    sandbox.seed(&package, &Sandbox::default_config(&package))?;
    sandbox.write(&RelPath::of("src/main.rs"), &msg!("fn main() {}\n"))?;
    sandbox.init_repo()?;
    Ok(sandbox)
}

#[test]
fn an_unknown_flag_is_a_usage_error() {
    assert_eq!(
        check_an_unknown_flag_is_a_usage_error(),
        Ok(()),
        "an_unknown_flag_is_a_usage_error"
    );
}

fn check_an_unknown_flag_is_a_usage_error() -> Result<(), Fail> {
    let sandbox = workspace(&Name::of("cli-unknown-flag"))?;
    let outcome = sandbox.run(&[Flag::Raw(msg!("--no-such-flag"))])?;
    ensure_eq!(outcome.exit, Exit::Error, "an unknown flag exits 2");
    ensure!(
        outcome.stderr().as_str().contains("Usage:"),
        "the usage line is printed: {:?}",
        outcome.stderr()
    );
    Ok(())
}

#[test]
fn a_flag_missing_its_value_is_a_usage_error() {
    assert_eq!(
        check_a_flag_missing_its_value_is_a_usage_error(),
        Ok(()),
        "a_flag_missing_its_value_is_a_usage_error"
    );
}

fn check_a_flag_missing_its_value_is_a_usage_error() -> Result<(), Fail> {
    let sandbox = workspace(&Name::of("cli-missing-value"))?;
    let outcome = sandbox.run_with_root_first(&[Flag::Raw(msg!("--against"))])?;
    ensure_eq!(outcome.exit, Exit::Error, "--against without a ref exits 2");
    Ok(())
}

#[test]
fn init_writes_a_config_and_both_ledgers() {
    assert_eq!(
        check_init_writes_a_config_and_both_ledgers(),
        Ok(()),
        "init_writes_a_config_and_both_ledgers"
    );
}

fn check_init_writes_a_config_and_both_ledgers() -> Result<(), Fail> {
    let package = Name::of("fx");
    let sandbox = Sandbox::create(&Name::of("cli-init"))?;
    sandbox.seed(&package, &Sandbox::default_config(&package))?;
    sandbox.write(&RelPath::of("src/main.rs"), &msg!("fn main() {}\n"))?;
    sandbox.remove(&RelPath::of(".unrepresentable/config"))?;
    sandbox.remove(&RelPath::of(".unrepresentable/ledger.md"))?;
    sandbox.remove(&RelPath::of(".unrepresentable/canary-exemptions.md"))?;
    let outcome = sandbox.run(&[Flag::Init])?;
    ensure_eq!(
        outcome.exit,
        Exit::Clean,
        "init exits 0: {:?}",
        outcome.stderr()
    );
    let config = sandbox.read(&RelPath::of(".unrepresentable/config"))?;
    ensure!(
        config.as_str().contains("[member.fx]"),
        "every member gets a role: {config:?}"
    );
    ensure!(
        config.as_str().contains("role = \"pure\""),
        "members start pure: {config:?}"
    );
    ensure!(
        config
            .as_str()
            .contains("code = [\"src/**\", \"tests/**\"]"),
        "every member's src and tests are code, since `code` is required: {config:?}"
    );
    ensure!(
        sandbox
            .read(&RelPath::of(".unrepresentable/ledger.md"))?
            .as_str()
            .contains("## Expect ledger"),
        "the expect ledger is written"
    );
    ensure!(
        sandbox
            .read(&RelPath::of(".unrepresentable/canary-exemptions.md"))?
            .as_str()
            .contains("## Canary exemptions"),
        "the canary exemptions are written"
    );
    Ok(())
}

#[test]
fn init_records_existing_findings_as_the_baseline() {
    assert_eq!(
        check_init_records_existing_findings_as_the_baseline(),
        Ok(()),
        "init_records_existing_findings_as_the_baseline"
    );
}

fn check_init_records_existing_findings_as_the_baseline() -> Result<(), Fail> {
    let package = Name::of("fx");
    let sandbox = Sandbox::create(&Name::of("cli-init-baseline"))?;
    sandbox.seed(&package, &Sandbox::default_config(&package))?;
    sandbox.write(
        &RelPath::of("src/main.rs"),
        &msg!("fn held() {\n    panic!(\"held\");\n}\nfn main() {}\n"),
    )?;
    sandbox.remove(&RelPath::of(".unrepresentable/config"))?;
    let outcome = sandbox.run(&[Flag::Init])?;
    ensure_eq!(
        outcome.exit,
        Exit::Clean,
        "init exits 0: {:?}",
        outcome.stderr()
    );
    let config = sandbox.read(&RelPath::of(".unrepresentable/config"))?;
    ensure!(
        config.as_str().contains("[[baseline-entry]]"),
        "the panic is recorded: {config:?}"
    );
    ensure!(
        config.as_str().contains("rule = \"abort-panic\""),
        "with its rule: {config:?}"
    );
    ensure!(
        config.as_str().contains("item = \"fn held\""),
        "and its item: {config:?}"
    );
    Ok(())
}

#[test]
fn init_refuses_to_overwrite_a_config() {
    assert_eq!(
        check_init_refuses_to_overwrite_a_config(),
        Ok(()),
        "init_refuses_to_overwrite_a_config"
    );
}

fn check_init_refuses_to_overwrite_a_config() -> Result<(), Fail> {
    let sandbox = workspace(&Name::of("cli-init-refuses"))?;
    let before = sandbox.read(&RelPath::of(".unrepresentable/config"))?;
    let outcome = sandbox.run(&[Flag::Init])?;
    ensure_eq!(
        outcome.exit,
        Exit::Error,
        "init over an existing config exits 2"
    );
    ensure_eq!(
        sandbox.read(&RelPath::of(".unrepresentable/config"))?,
        before,
        "the existing config is untouched"
    );
    Ok(())
}

/// Fails unless every logged cargo call has the subcommand first, then
/// `--release --locked`, and returns the calls.
fn logged_calls(sandbox: &Sandbox) -> Result<Vec<Message>, Fail> {
    let log = sandbox.read(&RelPath::of("cargo-calls.log"))?;
    let calls: Vec<Message> = log.as_str().lines().map(Message::of).collect();
    ensure!(!calls.is_empty(), "the run called cargo at least once");
    for call in &calls {
        let words: Vec<&str> = call.as_str().split_whitespace().collect();
        ensure_eq!(
            words.get(1..3),
            Some(&["--release", "--locked"][..]),
            "`cargo {call:?}` has --release --locked right after the subcommand"
        );
    }
    Ok(calls)
}

#[test]
fn a_normal_run_calls_cargo_with_release_and_locked() {
    assert_eq!(
        check_a_normal_run_calls_cargo_with_release_and_locked(),
        Ok(()),
        "a_normal_run_calls_cargo_with_release_and_locked"
    );
}

fn check_a_normal_run_calls_cargo_with_release_and_locked() -> Result<(), Fail> {
    let sandbox = workspace(&Name::of("cli-cargo-flags-run"))?;
    let logger = sandbox.install_cargo_logger()?;
    let outcome = sandbox.run_with_cargo(&[], &logger)?;
    ensure!(
        outcome.exit != Exit::Error,
        "the run decides: {:?}",
        outcome.stderr()
    );
    let calls = logged_calls(&sandbox)?;
    ensure!(
        calls.iter().any(|call| call
            .as_str()
            .starts_with("clippy --release --locked --version")),
        "`clippy --version` is called: {calls:?}"
    );
    ensure!(
        calls.iter().any(|call| call
            .as_str()
            .starts_with("clippy --release --locked -p fx --bin fx -- -W help")),
        "the lint tables are requested from one target: {calls:?}"
    );
    ensure_eq!(
        sandbox.has_path(&RelPath::of("target/debug")),
        Answer::No,
        "no debug profile directory is created"
    );
    Ok(())
}

#[test]
fn the_clippy_canaries_run_with_release_and_locked() {
    assert_eq!(
        check_the_clippy_canaries_run_with_release_and_locked(),
        Ok(()),
        "the_clippy_canaries_run_with_release_and_locked"
    );
}

fn check_the_clippy_canaries_run_with_release_and_locked() -> Result<(), Fail> {
    let package = Name::of("fx");
    let sandbox = Sandbox::create(&Name::of("cli-cargo-flags-canaries"))?;
    sandbox.seed(
        &package,
        &Message::of(
            "baseline = \"forbidden\"\ndefault-range = \"HEAD\"\ncode = [\"src/**\"]\nclippy-canaries = { package = \"fx\", dir = \"src/bin\" }\n\n[member.fx]\nrole = \"pure\"\n",
        ),
    )?;
    sandbox.write(&RelPath::of("src/main.rs"), &msg!("fn main() {}\n"))?;
    sandbox.write(&RelPath::of("src/bin/probe.rs"), &msg!("fn main() {}\n"))?;
    sandbox.init_repo()?;
    let logger = sandbox.install_cargo_logger()?;
    let outcome = sandbox.run_with_cargo(&[Flag::Canaries], &logger)?;
    ensure!(
        outcome.exit != Exit::Error,
        "the run decides: {:?}",
        outcome.stderr()
    );
    let calls = logged_calls(&sandbox)?;
    ensure!(
        calls.iter().any(|call| call.as_str().starts_with(
            "clippy --release --locked -p fx --bin probe --message-format=json -- -D warnings"
        )),
        "the canary binary is checked with the flags in place: {calls:?}"
    );
    ensure_eq!(
        sandbox.has_path(&RelPath::of("target/debug")),
        Answer::No,
        "no debug profile directory is created"
    );
    Ok(())
}

#[test]
fn a_file_in_the_clippy_canary_dir_that_no_canary_loads_is_reported() {
    assert_eq!(
        check_a_file_in_the_clippy_canary_dir_that_no_canary_loads_is_reported(),
        Ok(()),
        "a_file_in_the_clippy_canary_dir_that_no_canary_loads_is_reported"
    );
}

fn check_a_file_in_the_clippy_canary_dir_that_no_canary_loads_is_reported() -> Result<(), Fail> {
    let package = Name::of("fx");
    let sandbox = Sandbox::create(&Name::of("cli-canary-unrun-file"))?;
    sandbox.seed(
        &package,
        &Message::of(
            "baseline = \"forbidden\"\ndefault-range = \"HEAD\"\ncode = [\"src/**\"]\nclippy-canaries = { package = \"fx\", dir = \"src/bin\" }\n\n[member.fx]\nrole = \"pure\"\n",
        ),
    )?;
    sandbox.write(&RelPath::of("src/main.rs"), &msg!("fn main() {}\n"))?;
    sandbox.write(
        &RelPath::of("src/bin/probe.rs"),
        &msg!("#[path = \"probe/helper.rs\"]\nmod helper;\nfn main() {}\n"),
    )?;
    sandbox.write(
        &RelPath::of("src/bin/probe/helper.rs"),
        &msg!("fn used() {}\n"),
    )?;
    sandbox.write(
        &RelPath::of("src/bin/parked/production.rs"),
        &msg!("fn parked() {}\n"),
    )?;
    sandbox.init_repo()?;
    let logger = sandbox.install_cargo_logger()?;
    let outcome = sandbox.run_with_cargo(&[Flag::Canaries], &logger)?;
    ensure!(
        outcome.has_rule_in(
            &RelPath::of("src/bin/parked/production.rs"),
            &Name::of("canary")
        ) == Answer::Yes,
        "a file no canary loads is reported: {:?} {:?}",
        outcome.stderr(),
        outcome.findings()
    );
    ensure!(
        outcome.has_rule_in(&RelPath::of("src/bin/probe/helper.rs"), &Name::of("canary"))
            == Answer::No,
        "a module a canary loads is not: {:?}",
        outcome.findings()
    );
    Ok(())
}

#[test]
fn canaries_and_self_test_refuse_a_range() {
    assert_eq!(
        check_canaries_and_self_test_refuse_a_range(),
        Ok(()),
        "canaries_and_self_test_refuse_a_range"
    );
}

fn check_canaries_and_self_test_refuse_a_range() -> Result<(), Fail> {
    let package = Name::of("fx");
    let sandbox = Sandbox::create(&Name::of("cli-mode-range"))?;
    sandbox.seed(&package, &Sandbox::default_config(&package))?;
    sandbox.write(&RelPath::of("src/main.rs"), &msg!("fn main() {}\n"))?;
    sandbox.init_repo()?;
    for (mode, selector) in [("canaries", Flag::Canaries), ("self-test", Flag::SelfTest)] {
        for flag in [Flag::Against(Name::of("main")), Flag::Range(msg!("HEAD"))] {
            let outcome = sandbox.run(&[selector.clone(), flag])?;
            ensure_eq!(
                outcome.exit,
                Exit::Error,
                "{mode} with a range would look as if it checked a change: {:?}",
                outcome.stdout()
            );
            ensure!(
                outcome.stderr().as_str().contains("unexpected argument"),
                "{mode}: the error says why: {:?}",
                outcome.stderr()
            );
        }
    }
    Ok(())
}
