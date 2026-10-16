//! Every config key, in turn: loosening it is escalated (exit 3), tightening
//! it is free (exit 0). Restrictions loosen by being removed, exceptions by
//! being added.
use crate::support::{Answer, Exit, Fail, Flag, Message, Name, Outcome, RelPath, Sandbox};

macro_rules! msg {
    ($text:expr_2021) => {
        Message::of($text)
    };
}

macro_rules! top {
    ($line:expr_2021) => {
        Extra::Top(msg!($line))
    };
}

macro_rules! table {
    ($text:expr_2021) => {
        Extra::Table(msg!($text))
    };
}

/// One difference between the tight and the loose configuration.
enum Extra {
    /// A root-level line that must be absent.
    Drop(Message),
    /// Replaces the member table.
    Member(Message),
    /// Whole tables, appended after the member table.
    Table(Message),
    /// A root-level `key = value` line.
    Top(Message),
}

#[derive(Clone)]
struct Config {
    member: Message,
    tables: Vec<Message>,
    top: Vec<Message>,
}

impl Config {
    fn base() -> Self {
        Self {
            member: msg!("[member.fx]\nrole = \"pure\"\n"),
            tables: Vec::new(),
            top: vec![
                msg!("baseline = \"forbidden\""),
                msg!("default-range = \"main...HEAD\""),
                msg!("code = [\"src/**\"]"),
                msg!("judge = [\"policy/**\"]"),
                msg!("append-only = [\"policy/fixtures/**\"]"),
            ],
        }
    }

    fn render(&self) -> Message {
        let mut text = String::new();
        for line in &self.top {
            text.push_str(line.as_str());
            text.push('\n');
        }
        text.push('\n');
        text.push_str(self.member.as_str());
        for table in &self.tables {
            text.push('\n');
            text.push_str(table.as_str());
        }
        Message::from_string(text)
    }

    fn with(mut self, extra: &Extra) -> Self {
        match extra {
            Extra::Top(line) => self.top.push(line.clone()),
            Extra::Table(text) => self.tables.push(text.clone()),
            Extra::Drop(line) => self.top.retain(|kept| kept != line),
            Extra::Member(text) => self.member = text.clone(),
        }
        self
    }
}

/// The two configurations of one experiment.
struct Tight(Config);

struct Loose(Config);

/// Outcomes of going from tight to loose and from loose to tight.
struct Pair {
    loosened: Outcome,
    tightened: Outcome,
}

/// A committed configuration and the one that replaces it on the work branch.
struct Step {
    after: Config,
    before: Config,
}

fn change(name: &Name, step: &Step) -> Result<Outcome, Fail> {
    let package = Name::of("fx");
    let sandbox = Sandbox::create(name)?;
    sandbox.seed(&package, &step.before.render())?;
    // A crate for `forbidden-dependency` rules to name: a rule naming a crate
    // that is not in the lock file is an error.
    sandbox.append(
        &RelPath::of("Cargo.lock"),
        &msg!("\n[[package]]\nname = \"num-bigint\"\nversion = \"0.4.0\"\n"),
    )?;
    sandbox.write(&RelPath::of("src/main.rs"), &msg!("fn main() {}\n"))?;
    sandbox.write(&RelPath::of("policy/rules.txt"), &msg!("rule\n"))?;
    sandbox.init_repo()?;
    sandbox.git(&msg!("checkout -q -b work"))?;
    sandbox.write(
        &RelPath::of(".unrepresentable/config"),
        &step.after.render(),
    )?;
    sandbox.commit_all(&Name::of("config"))?;
    sandbox.run(&[Flag::Against(Name::of("main"))])
}

fn experiment(name: &Name, tight: &Tight, loose: &Loose) -> Result<Pair, Fail> {
    let loosening = Step {
        after: loose.0.clone(),
        before: tight.0.clone(),
    };
    let tightening = Step {
        after: tight.0.clone(),
        before: loose.0.clone(),
    };
    Ok(Pair {
        loosened: change(
            &Name::from_string(format!("cfg-{}-loosen", name.as_str())),
            &loosening,
        )?,
        tightened: change(
            &Name::from_string(format!("cfg-{}-tighten", name.as_str())),
            &tightening,
        )?,
    })
}

/// A restriction: the tight configuration has it, the loose one lacks it.
fn restriction(name: &Name, extras: &[Extra]) -> Result<Pair, Fail> {
    let mut tight = Config::base();
    for extra in extras {
        tight = tight.with(extra);
    }
    experiment(name, &Tight(tight), &Loose(Config::base()))
}

/// An exception: the loose configuration has it, the tight one lacks it.
fn exception(name: &Name, extras: &[Extra]) -> Result<Pair, Fail> {
    let mut loose = Config::base();
    for extra in extras {
        loose = loose.with(extra);
    }
    experiment(name, &Tight(Config::base()), &Loose(loose))
}

#[test]
fn baseline_mode_forbidden_to_shrink_only_is_loosening() {
    assert_eq!(
        check_baseline_mode_forbidden_to_shrink_only_is_loosening(),
        Ok(()),
        "baseline_mode_forbidden_to_shrink_only_is_loosening"
    );
}

fn check_baseline_mode_forbidden_to_shrink_only_is_loosening() -> Result<(), Fail> {
    let loose = Config::base()
        .with(&Extra::Drop(msg!("baseline = \"forbidden\"")))
        .with(&top!("baseline = \"shrink-only\""));
    let pair = experiment(
        &Name::of("baseline-mode"),
        &Tight(Config::base()),
        &Loose(loose),
    )?;
    ensure_eq!(
        pair.loosened.exit,
        Exit::Approval,
        "relaxing the baseline mode needs a human"
    );
    ensure!(
        pair.loosened
            .has_approval(&msg!("CONFIG_LOOSENED baseline forbidden -> shrink-only"))
            == Answer::Yes,
        "the mode change is named: {:?}",
        pair.loosened.approvals()
    );
    ensure_eq!(
        pair.tightened.exit,
        Exit::Clean,
        "forbidding the baseline is free"
    );
    Ok(())
}

#[test]
fn default_range_changes_are_escalated_both_ways() {
    assert_eq!(
        check_default_range_changes_are_escalated_both_ways(),
        Ok(()),
        "default_range_changes_are_escalated_both_ways"
    );
}

fn check_default_range_changes_are_escalated_both_ways() -> Result<(), Fail> {
    let other = Config::base()
        .with(&Extra::Drop(msg!("default-range = \"main...HEAD\"")))
        .with(&top!("default-range = \"HEAD\""));
    let pair = experiment(
        &Name::of("default-range"),
        &Tight(Config::base()),
        &Loose(other),
    )?;
    ensure!(
        pair.loosened
            .has_approval(&msg!("CONFIG_CHANGED default-range"))
            == Answer::Yes,
        "a new range is escalated: {:?}",
        pair.loosened.approvals()
    );
    ensure!(
        pair.tightened
            .has_approval(&msg!("CONFIG_CHANGED default-range"))
            == Answer::Yes,
        "so is changing it back: {:?}",
        pair.tightened.approvals()
    );
    Ok(())
}

#[test]
fn canary_dirs_are_exceptions() {
    assert_eq!(
        check_canary_dirs_are_exceptions(),
        Ok(()),
        "canary_dirs_are_exceptions"
    );
}

fn check_canary_dirs_are_exceptions() -> Result<(), Fail> {
    let pair = exception(
        &Name::of("canary-dirs"),
        &[top!("canary-dirs = [\"canaries/\"]")],
    )?;
    ensure_eq!(
        pair.loosened.exit,
        Exit::Approval,
        "a canary dir exempts files"
    );
    ensure!(
        pair.loosened
            .has_approval(&msg!("EXCEPTION_ADDED canary-dir:canaries/"))
            == Answer::Yes,
        "the directory is named: {:?}",
        pair.loosened.approvals()
    );
    ensure_eq!(
        pair.tightened.exit,
        Exit::Approval,
        "dropping it stops its canaries from running, so it needs a human too"
    );
    ensure!(
        pair.tightened
            .has_approval(&msg!("CONFIG_LOOSENED removed canary-dir:canaries/"))
            == Answer::Yes,
        "the removal is named: {:?}",
        pair.tightened.approvals()
    );
    Ok(())
}

#[test]
fn clippy_canaries_are_a_restriction() {
    assert_eq!(
        check_clippy_canaries_are_a_restriction(),
        Ok(()),
        "clippy_canaries_are_a_restriction"
    );
}

fn check_clippy_canaries_are_a_restriction() -> Result<(), Fail> {
    let pair = restriction(
        &Name::of("clippy-canaries"),
        &[top!(
            "clippy-canaries = { package = \"fx-canaries\", dir = \"canaries/src/bin\" }"
        )],
    )?;
    ensure_eq!(
        pair.loosened.exit,
        Exit::Approval,
        "dropping the canary crate is loosening"
    );
    ensure!(
        pair.loosened.has_approval(&msg!(
            "CONFIG_LOOSENED removed clippy-canaries:fx-canaries|canaries/src/bin"
        )) == Answer::Yes,
        "the canary crate is named: {:?}",
        pair.loosened.approvals()
    );
    ensure_eq!(pair.tightened.exit, Exit::Clean, "adding it is free");
    Ok(())
}

#[test]
fn arith_files_are_exceptions() {
    assert_eq!(
        check_arith_files_are_exceptions(),
        Ok(()),
        "arith_files_are_exceptions"
    );
}

fn check_arith_files_are_exceptions() -> Result<(), Fail> {
    let pair = exception(
        &Name::of("arith-files"),
        &[top!("arith-files = [\"src/arith.rs\"]")],
    )?;
    ensure_eq!(
        pair.loosened.exit,
        Exit::Approval,
        "an arithmetic file is an exception"
    );
    ensure!(
        pair.loosened
            .has_approval(&msg!("EXCEPTION_ADDED arith-file:src/arith.rs"))
            == Answer::Yes,
        "the file is named: {:?}",
        pair.loosened.approvals()
    );
    ensure_eq!(pair.tightened.exit, Exit::Clean, "dropping it is free");
    Ok(())
}

#[test]
fn configured_test_attributes_are_exceptions() {
    assert_eq!(
        check_test_attributes_are_exceptions(),
        Ok(()),
        "configured_test_attributes_are_exceptions"
    );
}

fn check_test_attributes_are_exceptions() -> Result<(), Fail> {
    let pair = exception(
        &Name::of("test-attributes"),
        &[top!("test-attributes = [\"my::test\"]")],
    )?;
    ensure_eq!(
        pair.loosened.exit,
        Exit::Approval,
        "a test attribute is an exception"
    );
    ensure!(
        pair.loosened
            .has_approval(&msg!("EXCEPTION_ADDED test-attribute:my::test"))
            == Answer::Yes,
        "the attribute is named: {:?}",
        pair.loosened.approvals()
    );
    ensure_eq!(pair.tightened.exit, Exit::Clean, "dropping it is free");
    Ok(())
}

#[test]
fn silent_fallback_names_are_a_restriction() {
    assert_eq!(
        check_silent_fallback_names_are_a_restriction(),
        Ok(()),
        "silent_fallback_names_are_a_restriction"
    );
}

fn check_silent_fallback_names_are_a_restriction() -> Result<(), Fail> {
    let pair = restriction(
        &Name::of("silent-fallback-names"),
        &[top!("silent-fallback-names = [\"lenient\"]")],
    )?;
    ensure_eq!(
        pair.loosened.exit,
        Exit::Approval,
        "dropping a banned name is loosening"
    );
    ensure!(
        pair.loosened
            .has_approval(&msg!("CONFIG_LOOSENED removed silent-fallback:lenient"))
            == Answer::Yes,
        "the name is reported: {:?}",
        pair.loosened.approvals()
    );
    ensure_eq!(
        pair.tightened.exit,
        Exit::Clean,
        "banning another name is free"
    );
    Ok(())
}

#[test]
fn fn_name_suffixes_are_a_restriction() {
    assert_eq!(
        check_fn_name_suffixes_are_a_restriction(),
        Ok(()),
        "fn_name_suffixes_are_a_restriction"
    );
}

fn check_fn_name_suffixes_are_a_restriction() -> Result<(), Fail> {
    let pair = restriction(
        &Name::of("fn-name-suffixes"),
        &[top!("fn-name-suffixes = [\"_hack\"]")],
    )?;
    ensure_eq!(
        pair.loosened.exit,
        Exit::Approval,
        "dropping a banned suffix is loosening"
    );
    ensure!(
        pair.loosened
            .has_approval(&msg!("CONFIG_LOOSENED removed fn-name-suffix:_hack"))
            == Answer::Yes,
        "the suffix is reported: {:?}",
        pair.loosened.approvals()
    );
    ensure_eq!(pair.tightened.exit, Exit::Clean, "banning one is free");
    Ok(())
}

#[test]
fn assertion_macros_are_exceptions() {
    assert_eq!(
        check_assertion_macros_are_exceptions(),
        Ok(()),
        "assertion_macros_are_exceptions"
    );
}

fn check_assertion_macros_are_exceptions() -> Result<(), Fail> {
    let pair = exception(
        &Name::of("assertion-macros"),
        &[top!("assertion-macros = [\"check\"]")],
    )?;
    ensure_eq!(
        pair.loosened.exit,
        Exit::Approval,
        "a custom assertion macro is an exception"
    );
    ensure!(
        pair.loosened
            .has_approval(&msg!("EXCEPTION_ADDED assertion-macro:check"))
            == Answer::Yes,
        "the macro is named: {:?}",
        pair.loosened.approvals()
    );
    ensure_eq!(pair.tightened.exit, Exit::Clean, "dropping it is free");
    Ok(())
}

#[test]
fn judge_globs_are_a_restriction() {
    assert_eq!(
        check_judge_globs_are_a_restriction(),
        Ok(()),
        "judge_globs_are_a_restriction"
    );
}

fn check_judge_globs_are_a_restriction() -> Result<(), Fail> {
    let loose = Config::base().with(&Extra::Drop(msg!("judge = [\"policy/**\"]")));
    let pair = experiment(&Name::of("judge"), &Tight(Config::base()), &Loose(loose))?;
    ensure_eq!(
        pair.loosened.exit,
        Exit::Approval,
        "narrowing the judge set is loosening"
    );
    ensure!(
        pair.loosened
            .has_approval(&msg!("CONFIG_LOOSENED removed judge:policy/**"))
            == Answer::Yes,
        "the glob is reported: {:?}",
        pair.loosened.approvals()
    );
    ensure_eq!(pair.tightened.exit, Exit::Clean, "widening it is free");
    Ok(())
}

#[test]
fn code_globs_are_a_restriction() {
    assert_eq!(
        check_code_globs_are_a_restriction(),
        Ok(()),
        "code_globs_are_a_restriction"
    );
}

fn check_code_globs_are_a_restriction() -> Result<(), Fail> {
    let tight = Config::base()
        .with(&Extra::Drop(msg!("code = [\"src/**\"]")))
        .with(&top!("code = [\"src/**\", \"lib/**\"]"));
    let pair = experiment(&Name::of("code"), &Tight(tight), &Loose(Config::base()))?;
    ensure_eq!(
        pair.loosened.exit,
        Exit::Approval,
        "narrowing the code set is loosening"
    );
    ensure!(
        pair.loosened
            .has_approval(&msg!("CONFIG_LOOSENED removed code:lib/**"))
            == Answer::Yes,
        "the glob is reported: {:?}",
        pair.loosened.approvals()
    );
    ensure_eq!(pair.tightened.exit, Exit::Clean, "widening it is free");
    Ok(())
}

#[test]
fn append_only_globs_are_exceptions() {
    assert_eq!(
        check_append_only_globs_are_exceptions(),
        Ok(()),
        "append_only_globs_are_exceptions"
    );
}

fn check_append_only_globs_are_exceptions() -> Result<(), Fail> {
    let tight = Config::base().with(&Extra::Drop(msg!("append-only = [\"policy/fixtures/**\"]")));
    let pair = experiment(
        &Name::of("append-only"),
        &Tight(tight),
        &Loose(Config::base()),
    )?;
    ensure_eq!(
        pair.loosened.exit,
        Exit::Approval,
        "an append-only glob is an exception"
    );
    ensure!(
        pair.loosened
            .has_approval(&msg!("EXCEPTION_ADDED append-only:policy/fixtures/**"))
            == Answer::Yes,
        "the glob is named: {:?}",
        pair.loosened.approvals()
    );
    ensure_eq!(pair.tightened.exit, Exit::Clean, "dropping it is free");
    Ok(())
}

#[test]
fn cargo_exceptions_are_exceptions() {
    assert_eq!(
        check_cargo_exceptions_are_exceptions(),
        Ok(()),
        "cargo_exceptions_are_exceptions"
    );
}

fn check_cargo_exceptions_are_exceptions() -> Result<(), Fail> {
    let pair = exception(
        &Name::of("cargo-exceptions"),
        &[top!(
            "cargo-exceptions = [\".cargo/config.toml:build.rustflags\"]"
        )],
    )?;
    ensure_eq!(
        pair.loosened.exit,
        Exit::Approval,
        "a cargo exception is an exception"
    );
    ensure!(
        pair.loosened.has_approval(&msg!(
            "EXCEPTION_ADDED cargo-exception:.cargo/config.toml:build.rustflags"
        )) == Answer::Yes,
        "the entry is named: {:?}",
        pair.loosened.approvals()
    );
    ensure_eq!(pair.tightened.exit, Exit::Clean, "dropping it is free");
    Ok(())
}

#[test]
fn a_pure_role_becoming_shell_is_loosening() {
    assert_eq!(
        check_a_pure_role_becoming_shell_is_loosening(),
        Ok(()),
        "a_pure_role_becoming_shell_is_loosening"
    );
}

fn check_a_pure_role_becoming_shell_is_loosening() -> Result<(), Fail> {
    let shell = Extra::Member(msg!("[member.fx]\nrole = \"shell\"\n"));
    let pair = exception(&Name::of("role"), &[shell])?;
    ensure_eq!(
        pair.loosened.exit,
        Exit::Approval,
        "shell may touch the world"
    );
    ensure!(
        pair.loosened
            .has_approval(&msg!("CONFIG_LOOSENED removed role-pure:fx"))
            == Answer::Yes,
        "the lost purity is reported: {:?}",
        pair.loosened.approvals()
    );
    ensure!(
        pair.loosened
            .has_approval(&msg!("EXCEPTION_ADDED role-shell:fx"))
            == Answer::Yes,
        "the shell role is reported: {:?}",
        pair.loosened.approvals()
    );
    ensure_eq!(pair.tightened.exit, Exit::Clean, "becoming pure is free");
    Ok(())
}

#[test]
fn ambient_allowances_are_exceptions() {
    assert_eq!(
        check_ambient_allowances_are_exceptions(),
        Ok(()),
        "ambient_allowances_are_exceptions"
    );
}

fn check_ambient_allowances_are_exceptions() -> Result<(), Fail> {
    let shell = Extra::Member(msg!("[member.fx]\nrole = \"shell\"\n"));
    let allowed = Extra::Member(msg!(
        "[member.fx]\nrole = \"shell\"\nambient = [{ path = \"std::env\", files = [\"src/**\"] }]\n"
    ));
    let tight = Config::base().with(&shell);
    let loose = Config::base().with(&allowed);
    let pair = experiment(&Name::of("ambient"), &Tight(tight), &Loose(loose))?;
    ensure_eq!(
        pair.loosened.exit,
        Exit::Approval,
        "an ambient allowance is an exception"
    );
    ensure!(
        pair.loosened
            .has_approval(&msg!("EXCEPTION_ADDED ambient:fx:std::env:src/**"))
            == Answer::Yes,
        "the allowance is named: {:?}",
        pair.loosened.approvals()
    );
    ensure_eq!(pair.tightened.exit, Exit::Clean, "dropping it is free");
    Ok(())
}

#[test]
fn newtype_allowances_are_exceptions() {
    assert_eq!(
        check_newtype_allowances_are_exceptions(),
        Ok(()),
        "newtype_allowances_are_exceptions"
    );
}

fn check_newtype_allowances_are_exceptions() -> Result<(), Fail> {
    let allowed = Extra::Member(msg!(
        "[member.fx]\nrole = \"pure\"\nnewtypes = [{ type = \"Id\", methods = [\"new\"] }]\n"
    ));
    let pair = exception(&Name::of("newtypes"), &[allowed])?;
    ensure_eq!(
        pair.loosened.exit,
        Exit::Approval,
        "a newtype allowance is an exception"
    );
    ensure!(
        pair.loosened
            .has_approval(&msg!("EXCEPTION_ADDED newtype:fx:Id:new"))
            == Answer::Yes,
        "the allowance is named: {:?}",
        pair.loosened.approvals()
    );
    ensure_eq!(pair.tightened.exit, Exit::Clean, "dropping it is free");
    Ok(())
}

#[test]
fn ban_sets_are_a_restriction() {
    assert_eq!(
        check_ban_sets_are_a_restriction(),
        Ok(()),
        "ban_sets_are_a_restriction"
    );
}

fn check_ban_sets_are_a_restriction() -> Result<(), Fail> {
    let ban = table!(
        "[[ban]]\nid = \"no-writes\"\npaths = [\"std::fs::write\"]\nmacros = [\"log::*\"]\n"
    );
    let pair = restriction(&Name::of("ban"), &[ban])?;
    ensure_eq!(
        pair.loosened.exit,
        Exit::Approval,
        "dropping a ban set is loosening"
    );
    ensure!(
        pair.loosened
            .has_approval(&msg!("CONFIG_LOOSENED removed ban:no-writes"))
            == Answer::Yes,
        "the set is reported: {:?}",
        pair.loosened.approvals()
    );
    ensure!(
        pair.loosened.has_approval(&msg!(
            "CONFIG_LOOSENED removed ban-path:no-writes:std::fs::write"
        )) == Answer::Yes,
        "its path is reported: {:?}",
        pair.loosened.approvals()
    );
    ensure!(
        pair.loosened
            .has_approval(&msg!("CONFIG_LOOSENED removed ban-macro:no-writes:log::*"))
            == Answer::Yes,
        "its macro is reported: {:?}",
        pair.loosened.approvals()
    );
    ensure_eq!(pair.tightened.exit, Exit::Clean, "adding a ban is free");
    Ok(())
}

#[test]
fn removing_one_path_from_a_ban_is_loosening() {
    assert_eq!(
        check_removing_one_path_from_a_ban_is_loosening(),
        Ok(()),
        "removing_one_path_from_a_ban_is_loosening"
    );
}

fn check_removing_one_path_from_a_ban_is_loosening() -> Result<(), Fail> {
    let wide = table!(
        "[[ban]]\nid = \"no-writes\"\npaths = [\"std::fs::write\", \"std::fs::remove_file\"]\n"
    );
    let narrow = table!("[[ban]]\nid = \"no-writes\"\npaths = [\"std::fs::write\"]\n");
    let tight = Config::base().with(&wide);
    let loose = Config::base().with(&narrow);
    let pair = experiment(&Name::of("ban-path"), &Tight(tight), &Loose(loose))?;
    ensure_eq!(
        pair.loosened.exit,
        Exit::Approval,
        "a shorter ban list is loosening"
    );
    ensure!(
        pair.loosened.has_approval(&msg!(
            "CONFIG_LOOSENED removed ban-path:no-writes:std::fs::remove_file"
        )) == Answer::Yes,
        "the dropped path is reported: {:?}",
        pair.loosened.approvals()
    );
    ensure_eq!(
        pair.tightened.exit,
        Exit::Clean,
        "a longer ban list is free"
    );
    Ok(())
}

#[test]
fn macro_comment_requirements_are_a_restriction() {
    assert_eq!(
        check_macro_comment_requirements_are_a_restriction(),
        Ok(()),
        "macro_comment_requirements_are_a_restriction"
    );
}

fn check_macro_comment_requirements_are_a_restriction() -> Result<(), Fail> {
    let required = table!(
        "[[macro-comment]]\nmacro = \"proptest\"\nprefix = \"// pins: \"\nwithin-lines = 3\n"
    );
    let pair = restriction(&Name::of("macro-comment"), &[required])?;
    ensure_eq!(
        pair.loosened.exit,
        Exit::Approval,
        "dropping a comment requirement is loosening"
    );
    ensure!(
        pair.loosened.has_approval(&msg!(
            "CONFIG_LOOSENED removed macro-comment:proptest|// pins: |3"
        )) == Answer::Yes,
        "the requirement is reported: {:?}",
        pair.loosened.approvals()
    );
    ensure_eq!(pair.tightened.exit, Exit::Clean, "adding one is free");
    Ok(())
}

#[test]
fn forbidden_dependencies_are_a_restriction() {
    assert_eq!(
        check_forbidden_dependencies_are_a_restriction(),
        Ok(()),
        "forbidden_dependencies_are_a_restriction"
    );
}

fn check_forbidden_dependencies_are_a_restriction() -> Result<(), Fail> {
    let rule =
        table!("[[forbidden-dependency]]\ncrate = \"fx\"\nmust-not-reach = [\"num-bigint\"]\n");
    let pair = restriction(&Name::of("forbidden-dependency"), &[rule])?;
    ensure_eq!(
        pair.loosened.exit,
        Exit::Approval,
        "dropping a boundary is loosening"
    );
    ensure!(
        pair.loosened.has_approval(&msg!(
            "CONFIG_LOOSENED removed forbidden-dependency:fx->num-bigint"
        )) == Answer::Yes,
        "the boundary is reported: {:?}",
        pair.loosened.approvals()
    );
    ensure_eq!(pair.tightened.exit, Exit::Clean, "adding one is free");
    Ok(())
}

#[test]
fn forbidden_references_are_a_restriction() {
    assert_eq!(
        check_forbidden_references_are_a_restriction(),
        Ok(()),
        "forbidden_references_are_a_restriction"
    );
}

fn check_forbidden_references_are_a_restriction() -> Result<(), Fail> {
    let rule = table!(
        "[[forbidden-reference]]\nfiles = [\"src/view/**\"]\nmust-not-name = [\"crate::edit::Form\"]\n"
    );
    let pair = restriction(&Name::of("forbidden-reference"), &[rule])?;
    ensure_eq!(
        pair.loosened.exit,
        Exit::Approval,
        "dropping a reference ban is loosening"
    );
    ensure!(
        pair.loosened.has_approval(&msg!(
            "CONFIG_LOOSENED removed forbidden-reference:src/view/**>>crate::edit::Form"
        )) == Answer::Yes,
        "the ban is reported: {:?}",
        pair.loosened.approvals()
    );
    ensure_eq!(pair.tightened.exit, Exit::Clean, "adding one is free");
    Ok(())
}
