//! The baseline and `--ratchet`: entries may only shrink or re-key within one
//! (rule, crate) group, never grow and never move between crates.

use crate::support::{Answer, Exit, Fail, Flag, Holds as _, Message, Name, RelPath, Sandbox};

macro_rules! put {
    ($sandbox:expr_2021, $name:literal, $text:expr_2021) => {
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

const CONFIG: &str = "baseline = \"shrink-only\"
default-range = \"main...HEAD\"
code = [\"src/**\", \"crates/**\"]
judge = [\"policy/**\"]

[member.fx]
role = \"pure\"
";

const MEMBER_CONFIG: &str = "baseline = \"shrink-only\"
default-range = \"main...HEAD\"
code = [\"src/**\", \"crates/**\"]
judge = [\"policy/**\"]

[member.a]
role = \"pure\"

[member.b]
role = \"pure\"
";

const ENTRY: &str = "
[[baseline-entry]]
rule = \"abort-panic\"
file = \"src/main.rs\"
item = \"fn held\"
index = 0
";

const MEMBER_ENTRY: &str = "
[[baseline-entry]]
rule = \"abort-panic\"
file = \"crates/a/src/main.rs\"
item = \"fn held\"
index = 0
";

const HELD: &str = "fn held() {\n    panic!(\"held\");\n}\nfn main() {}\n";

/// A workspace with `config` and a judge file, not yet committed.
fn project(name: &Name, config: &Message) -> Result<Sandbox, Fail> {
    let package = Name::of("fx");
    let sandbox = Sandbox::create(name)?;
    sandbox.seed(&package, config)?;
    put!(sandbox, "policy/rules.txt", "rule\n")?;
    Ok(sandbox)
}

/// Commits everything on `main`, then checks out the working branch `work`.
fn start(sandbox: &Sandbox) -> Result<(), Fail> {
    sandbox.init_repo()?;
    sandbox.git(&msg!("checkout -q -b work"))?;
    Ok(())
}

fn with_entry() -> Message {
    Message::from_string(format!("{CONFIG}{ENTRY}"))
}

fn against_main() -> Flag {
    Flag::Against(Name::of("main"))
}

fn cfg_has_entries(sandbox: &Sandbox) -> Result<Answer, Fail> {
    Ok(sandbox
        .read(&rel!(".unrepresentable/config"))?
        .as_str()
        .contains("[[baseline-entry]]")
        .answer())
}

fn cfg_text(sandbox: &Sandbox) -> Result<Message, Fail> {
    sandbox.read(&rel!(".unrepresentable/config"))
}

#[test]
fn a_baseline_entry_covers_its_finding() {
    assert_eq!(
        check_a_baseline_entry_covers_its_finding(),
        Ok(()),
        "a_baseline_entry_covers_its_finding"
    );
}

fn check_a_baseline_entry_covers_its_finding() -> Result<(), Fail> {
    let sandbox = project(&Name::of("ratchet-covered"), &with_entry())?;
    put!(sandbox, "src/main.rs", HELD)?;
    start(&sandbox)?;
    let outcome = sandbox.run(&[against_main()])?;
    ensure_eq!(
        outcome.findings(),
        Vec::<Message>::new(),
        "the entry covers the panic"
    );
    ensure_eq!(outcome.exit, Exit::Clean, "covered findings exit 0");
    Ok(())
}

#[test]
fn adding_an_entry_needs_approval() {
    assert_eq!(
        check_adding_an_entry_needs_approval(),
        Ok(()),
        "adding_an_entry_needs_approval"
    );
}

fn check_adding_an_entry_needs_approval() -> Result<(), Fail> {
    let sandbox = project(&Name::of("ratchet-add"), &msg!(CONFIG))?;
    put!(sandbox, "src/main.rs", HELD)?;
    start(&sandbox)?;
    sandbox.append(&rel!(".unrepresentable/config"), &msg!(ENTRY))?;
    sandbox.commit_all(&Name::of("baseline"))?;
    let outcome = sandbox.run(&[against_main()])?;
    ensure_eq!(
        outcome.findings(),
        Vec::<Message>::new(),
        "the new entry covers the panic"
    );
    ensure_eq!(
        outcome.exit,
        Exit::Approval,
        "growing the baseline needs a human"
    );
    ensure!(
        outcome.has_approval(&msg!("BASELINE_GREW abort-panic in fx (0 -> 1)")) == Answer::Yes,
        "the growth is named: {:?}",
        outcome.approvals()
    );
    Ok(())
}

#[test]
fn removing_a_finding_makes_its_entry_stale() {
    assert_eq!(
        check_removing_a_finding_makes_its_entry_stale(),
        Ok(()),
        "removing_a_finding_makes_its_entry_stale"
    );
}

fn check_removing_a_finding_makes_its_entry_stale() -> Result<(), Fail> {
    let sandbox = project(&Name::of("ratchet-stale"), &with_entry())?;
    put!(sandbox, "src/main.rs", HELD)?;
    start(&sandbox)?;
    put!(sandbox, "src/main.rs", "fn main() {}\n")?;
    let outcome = {
        sandbox.commit_all(&Name::of("fix"))?;
        sandbox.run(&[against_main()])?
    };
    ensure_eq!(outcome.exit, Exit::Findings, "a stale entry is a finding");
    ensure!(
        outcome.has_rule_in(&rel!(".unrepresentable/config"), &Name::of("baseline")) == Answer::Yes,
        "the stale entry is reported: {:?}",
        outcome.findings()
    );
    Ok(())
}

#[test]
fn ratchet_drops_a_stale_entry_and_the_cfg_change_ships_alone() {
    assert_eq!(
        check_ratchet_drops_a_stale_entry_and_the_cfg_change_ships_alone(),
        Ok(()),
        "ratchet_drops_a_stale_entry_and_the_cfg_change_ships_alone"
    );
}

fn check_ratchet_drops_a_stale_entry_and_the_cfg_change_ships_alone() -> Result<(), Fail> {
    let sandbox = project(&Name::of("ratchet-drop"), &with_entry())?;
    put!(sandbox, "src/main.rs", HELD)?;
    start(&sandbox)?;
    put!(sandbox, "src/main.rs", "fn main() {}\n")?;
    sandbox.commit_all(&Name::of("fix"))?;
    let outcome = sandbox.run(&[Flag::Ratchet, against_main()])?;
    ensure!(
        cfg_has_entries(&sandbox)? == Answer::No,
        "the stale entry is gone from the cfg"
    );
    ensure_eq!(
        outcome.findings(),
        vec![msg!(".unrepresentable/config:1:1: judge-code-pairing")],
        "only the pairing finding remains: the cfg rewrite must be its own change"
    );
    sandbox.commit_all(&Name::of("ratchet"))?;
    let alone = sandbox.run(&[Flag::Range(msg!("HEAD~1...HEAD"))])?;
    ensure_eq!(
        alone.exit,
        Exit::Clean,
        "a shrinking baseline change needs nobody"
    );
    ensure_eq!(
        alone.approvals(),
        Vec::<Message>::new(),
        "shrinking is free"
    );
    Ok(())
}

#[test]
fn ratchet_rekeys_a_renamed_fn() {
    assert_eq!(
        check_ratchet_rekeys_a_renamed_fn(),
        Ok(()),
        "ratchet_rekeys_a_renamed_fn"
    );
}

fn check_ratchet_rekeys_a_renamed_fn() -> Result<(), Fail> {
    let sandbox = project(&Name::of("ratchet-rename"), &with_entry())?;
    put!(sandbox, "src/main.rs", HELD)?;
    start(&sandbox)?;
    put!(
        sandbox,
        "src/main.rs",
        "fn renamed() {\n    panic!(\"held\");\n}\nfn main() {}\n"
    )?;
    sandbox.commit_all(&Name::of("rename"))?;
    let before = sandbox.run(&[against_main()])?;
    ensure!(
        before.has_rule_in(&rel!("src/main.rs"), &Name::of("abort-panic")) == Answer::Yes,
        "the renamed finding is new: {:?}",
        before.findings()
    );
    ensure!(
        before.has_rule_in(&rel!(".unrepresentable/config"), &Name::of("baseline")) == Answer::Yes,
        "the old entry is stale: {:?}",
        before.findings()
    );
    let after = sandbox.run(&[Flag::Ratchet, against_main()])?;
    ensure!(
        cfg_text(&sandbox)?
            .as_str()
            .contains("item = \"fn renamed\""),
        "the entry was re-keyed: {:?}",
        cfg_text(&sandbox)?
    );
    ensure_eq!(
        after.findings(),
        vec![msg!(".unrepresentable/config:1:1: judge-code-pairing")],
        "the rename itself passes; only the shared change is flagged"
    );
    sandbox.commit_all(&Name::of("rekey"))?;
    let alone = sandbox.run(&[Flag::Range(msg!("HEAD~1...HEAD"))])?;
    ensure_eq!(
        alone.exit,
        Exit::Clean,
        "re-keying within a group needs nobody"
    );
    ensure_eq!(
        alone.approvals(),
        Vec::<Message>::new(),
        "the count did not grow"
    );
    Ok(())
}

#[test]
fn ratchet_rekeys_a_finding_moved_to_another_file_of_the_same_crate() {
    assert_eq!(
        check_ratchet_rekeys_a_finding_moved_to_another_file_of_the_same_crate(),
        Ok(()),
        "ratchet_rekeys_a_finding_moved_to_another_file_of_the_same_crate"
    );
}

fn check_ratchet_rekeys_a_finding_moved_to_another_file_of_the_same_crate() -> Result<(), Fail> {
    let sandbox = project(&Name::of("ratchet-same-crate"), &with_entry())?;
    put!(sandbox, "src/main.rs", HELD)?;
    start(&sandbox)?;
    put!(sandbox, "src/main.rs", "mod other;\nfn main() {}\n")?;
    put!(
        sandbox,
        "src/other.rs",
        "fn held() {\n    panic!(\"held\");\n}\n"
    )?;
    sandbox.commit_all(&Name::of("move"))?;
    let after = sandbox.run(&[Flag::Ratchet, against_main()])?;
    ensure!(
        cfg_text(&sandbox)?
            .as_str()
            .contains("file = \"src/other.rs\""),
        "the entry follows the file: {:?}",
        cfg_text(&sandbox)?
    );
    ensure_eq!(
        after.findings(),
        vec![msg!(".unrepresentable/config:1:1: judge-code-pairing")],
        "only the shared change is flagged"
    );
    Ok(())
}

#[test]
fn ratchet_never_moves_a_violation_between_crates() {
    assert_eq!(
        check_ratchet_never_moves_a_violation_between_crates(),
        Ok(()),
        "ratchet_never_moves_a_violation_between_crates"
    );
}

fn check_ratchet_never_moves_a_violation_between_crates() -> Result<(), Fail> {
    let package_a = Name::of("a");
    let package_b = Name::of("b");
    let sandbox = Sandbox::create(&Name::of("ratchet-cross-crate"))?;
    sandbox.seed_members(
        &[package_a, package_b],
        &Message::from_string(format!("{MEMBER_CONFIG}{MEMBER_ENTRY}")),
    )?;
    put!(
        sandbox,
        "crates/a/src/main.rs",
        "fn held() {\n    panic!(\"held\");\n}\nfn main() {}\n"
    )?;
    sandbox.init_repo()?;
    sandbox.git(&msg!("checkout -q -b work"))?;
    put!(sandbox, "crates/a/src/main.rs", "fn main() {}\n")?;
    put!(
        sandbox,
        "crates/b/src/main.rs",
        "fn held() {\n    panic!(\"held\");\n}\nfn main() {}\n"
    )?;
    sandbox.commit_all(&Name::of("move"))?;
    let plain = sandbox.run(&[against_main()])?;
    ensure_eq!(plain.exit, Exit::Findings, "the moved violation is new");
    let ratcheted = sandbox.run(&[Flag::Ratchet, against_main()])?;
    ensure_eq!(ratcheted.exit, Exit::Findings, "ratchet cannot launder it");
    ensure!(
        ratcheted.has_rule_in(&rel!("crates/b/src/main.rs"), &Name::of("abort-panic"))
            == Answer::Yes,
        "the violation in the other crate is still reported: {:?}",
        ratcheted.findings()
    );
    ensure!(
        !cfg_text(&sandbox)?
            .as_str()
            .contains("crates/b/src/main.rs"),
        "no entry was created for the other crate: {:?}",
        cfg_text(&sandbox)?
    );
    Ok(())
}

#[test]
fn ratchet_cannot_add_a_new_violation() {
    assert_eq!(
        check_ratchet_cannot_add_a_new_violation(),
        Ok(()),
        "ratchet_cannot_add_a_new_violation"
    );
}

fn check_ratchet_cannot_add_a_new_violation() -> Result<(), Fail> {
    let sandbox = project(&Name::of("ratchet-new"), &msg!(CONFIG))?;
    put!(sandbox, "src/main.rs", "fn main() {}\n")?;
    start(&sandbox)?;
    put!(
        sandbox,
        "src/main.rs",
        "fn held() {\n    panic!(\"new\");\n}\nfn main() {}\n"
    )?;
    sandbox.commit_all(&Name::of("violate"))?;
    let outcome = sandbox.run(&[Flag::Ratchet, against_main()])?;
    ensure_eq!(
        outcome.exit,
        Exit::Findings,
        "a new violation stays a finding"
    );
    ensure!(
        outcome.has_rule_in(&rel!("src/main.rs"), &Name::of("abort-panic")) == Answer::Yes,
        "the violation is reported: {:?}",
        outcome.findings()
    );
    ensure!(
        cfg_has_entries(&sandbox)? == Answer::No,
        "ratchet wrote no entry"
    );
    Ok(())
}

#[test]
fn ratchet_does_not_trade_one_rule_for_another() {
    assert_eq!(
        check_ratchet_does_not_trade_one_rule_for_another(),
        Ok(()),
        "ratchet_does_not_trade_one_rule_for_another"
    );
}

fn check_ratchet_does_not_trade_one_rule_for_another() -> Result<(), Fail> {
    let sandbox = project(&Name::of("ratchet-other-rule"), &with_entry())?;
    put!(sandbox, "src/main.rs", HELD)?;
    start(&sandbox)?;
    put!(
        sandbox,
        "src/main.rs",
        "struct Id(u32);\nfn held(value: Option<Id>) -> Id {\n    value.unwrap()\n}\nfn main() {}\n"
    )?;
    sandbox.commit_all(&Name::of("swap"))?;
    let outcome = sandbox.run(&[Flag::Ratchet, against_main()])?;
    ensure!(
        outcome.has_rule_in(&rel!("src/main.rs"), &Name::of("abort-unwrap")) == Answer::Yes,
        "the unwrap is not covered by the panic entry: {:?}",
        outcome.findings()
    );
    ensure!(
        cfg_has_entries(&sandbox)? == Answer::No,
        "the stale entry was dropped, nothing was added"
    );
    Ok(())
}

#[test]
fn forbidden_mode_rejects_every_entry() {
    assert_eq!(
        check_forbidden_mode_rejects_every_entry(),
        Ok(()),
        "forbidden_mode_rejects_every_entry"
    );
}

fn check_forbidden_mode_rejects_every_entry() -> Result<(), Fail> {
    let config = Message::from_string(format!(
        "{}{ENTRY}",
        CONFIG.replace("shrink-only", "forbidden")
    ));
    let sandbox = project(&Name::of("ratchet-forbidden"), &config)?;
    put!(sandbox, "src/main.rs", HELD)?;
    start(&sandbox)?;
    let outcome = sandbox.run(&[against_main()])?;
    ensure_eq!(
        outcome.exit,
        Exit::Findings,
        "entries are findings in forbidden mode"
    );
    ensure!(
        outcome.has_rule_in(&rel!(".unrepresentable/config"), &Name::of("baseline")) == Answer::Yes,
        "the entry is reported: {:?}",
        outcome.findings()
    );
    Ok(())
}
