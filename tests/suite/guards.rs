//! Guards over manifests, lint config and dependency boundaries.
use crate::support::{
    check_case, Answer, Edit, Exit, Fail, Flag, Message, Name, Outcome, RelPath, Sandbox,
};

#[cfg(unix)]
use crate::support::Link;

/// The config of a project whose `canaries/` directory holds rule canaries.
const CANARY_CONFIG: &str = "baseline = \"forbidden\"
default-range = \"HEAD\"
code = [\"src/**\"]
canary-dirs = [\"tests/canaries/\"]

[member.fx]
role = \"pure\"

[[macro-comment]]
macro = \"proptest\"
prefix = \"// pins: \"
within-lines = 3
";

/// A replacement in the root manifest.
struct ManifestEdit {
    /// The text to find.
    from: Message,
    /// The text that replaces `from`.
    to: Message,
}

#[test]
fn manifest_lints_positive() {
    assert_eq!(
        check_case(&Name::of("manifest-lints/positive")),
        Ok(()),
        "manifest-lints/positive"
    );
}

#[test]
fn manifest_lints_negative() {
    assert_eq!(
        check_case(&Name::of("manifest-lints/negative")),
        Ok(()),
        "manifest-lints/negative"
    );
}

#[test]
fn stray_lint_config_member_clippy_toml() {
    assert_eq!(
        check_case(&Name::of("stray-lint-config/member-clippy-toml")),
        Ok(()),
        "stray-lint-config/member-clippy-toml"
    );
}

#[test]
fn stray_lint_config_cargo_rustflags() {
    assert_eq!(
        check_case(&Name::of("stray-lint-config/cargo-rustflags")),
        Ok(()),
        "stray-lint-config/cargo-rustflags"
    );
}

#[test]
fn stray_lint_config_cargo_env_and_wrapper() {
    assert_eq!(
        check_case(&Name::of("stray-lint-config/cargo-env-and-wrapper")),
        Ok(()),
        "stray-lint-config/cargo-env-and-wrapper"
    );
}

#[test]
fn stray_lint_config_recorded_exception() {
    assert_eq!(
        check_case(&Name::of("stray-lint-config/recorded-exception")),
        Ok(()),
        "stray-lint-config/recorded-exception"
    );
}

#[test]
fn stray_lint_config_harmless_cargo_config() {
    assert_eq!(
        check_case(&Name::of("stray-lint-config/harmless-cargo-config")),
        Ok(()),
        "stray-lint-config/harmless-cargo-config"
    );
}

#[test]
fn stray_lint_config_member_under_node_modules() {
    assert_eq!(
        check_case(&Name::of("stray-lint-config/member-under-node-modules")),
        Ok(()),
        "stray-lint-config/member-under-node-modules"
    );
}

#[test]
fn stray_lint_config_ancestor_clippy_toml() {
    assert_eq!(
        check_case(&Name::of("stray-lint-config/ancestor-clippy-toml")),
        Ok(()),
        "stray-lint-config/ancestor-clippy-toml"
    );
}

#[test]
fn stray_lint_config_ancestor_cargo_config() {
    assert_eq!(
        check_case(&Name::of("stray-lint-config/ancestor-cargo-config")),
        Ok(()),
        "stray-lint-config/ancestor-cargo-config"
    );
}

#[test]
fn stray_lint_config_pnpm_package() {
    assert_eq!(
        check_case(&Name::of("stray-lint-config/pnpm-package")),
        Ok(()),
        "stray-lint-config/pnpm-package"
    );
}

#[test]
fn stray_lint_config_member_cargo_config() {
    assert_eq!(
        check_case(&Name::of("stray-lint-config/member-cargo-config")),
        Ok(()),
        "stray-lint-config/member-cargo-config"
    );
}

#[test]
fn stray_lint_config_gitignored_member() {
    assert_eq!(
        check_case(&Name::of("stray-lint-config/gitignored-member")),
        Ok(()),
        "stray-lint-config/gitignored-member"
    );
}

#[test]
fn stray_lint_config_path_dependency_member() {
    assert_eq!(
        check_case(&Name::of("stray-lint-config/path-dependency-member")),
        Ok(()),
        "stray-lint-config/path-dependency-member"
    );
}

#[cfg(unix)]
#[test]
fn stray_lint_config_pnpm_links_are_not_reported() {
    assert_eq!(
        check_pnpm_links_are_not_reported(),
        Ok(()),
        "stray_lint_config_pnpm_links_are_not_reported"
    );
}

#[cfg(unix)]
#[test]
fn stray_lint_config_linked_member_dir_is_reported() {
    assert_eq!(
        check_linked_member_dir_is_reported(),
        Ok(()),
        "stray_lint_config_linked_member_dir_is_reported"
    );
}

#[cfg(unix)]
#[test]
fn stray_lint_config_linked_ancestor_dir_is_reported() {
    assert_eq!(
        check_linked_ancestor_dir_is_reported(),
        Ok(()),
        "stray_lint_config_linked_ancestor_dir_is_reported"
    );
}

#[cfg(unix)]
#[test]
fn stray_lint_config_linked_member_cargo_dir_is_reported() {
    assert_eq!(
        check_linked_member_cargo_dir_is_reported(),
        Ok(()),
        "stray_lint_config_linked_member_cargo_dir_is_reported"
    );
}

/// A config with a `pure` role for each member in `names`.
fn members_config(names: &[Name]) -> Message {
    let blocks: Vec<String> = names
        .iter()
        .map(|name| format!("\n[member.{}]\nrole = \"pure\"\n", name.as_str()))
        .collect();
    let roles = blocks.concat();
    Message::from_string(format!(
        "baseline = \"forbidden\"\ndefault-range = \"HEAD\"\ncode = [\"src/**\"]\n{roles}"
    ))
}

/// The manifest of a member package `name`, with `extra` appended.
#[cfg(unix)]
fn member_manifest(name: &Name, extra: &Message) -> Message {
    Message::from_string(format!(
        "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lints]\nworkspace = true\n{}",
        name.as_str(),
        extra.as_str()
    ))
}

/// A pnpm-style directory link inside `node_modules`, and the same kind of link
/// elsewhere, are both outside every member and ancestor directory: neither is
/// reported.
#[cfg(unix)]
fn check_pnpm_links_are_not_reported() -> Result<(), Fail> {
    let package = Name::of("fx");
    let sandbox = Sandbox::create(&Name::of("case-pnpm-links"))?;
    sandbox.seed(&package, &Sandbox::default_config(&package))?;
    sandbox.overlay(&RelPath::of("stray-lint-config/pnpm-package"))?;
    sandbox.link(&Link {
        path: RelPath::of("node_modules/alias"),
        to: RelPath::of("pnpm-pkg"),
    })?;
    sandbox.link(&Link {
        path: RelPath::of("vendor/alias"),
        to: RelPath::of("../node_modules/pnpm-pkg"),
    })?;
    let outcome = run_committed(&sandbox)?;
    for path in ["node_modules/alias", "vendor/alias"] {
        ensure!(
            outcome.has_rule_in(&RelPath::of(path), &Name::of("stray-lint-config")) == Answer::No,
            "{} is outside every member and ancestor: {:?} {:?}",
            path,
            outcome.stderr(),
            outcome.findings()
        );
    }
    Ok(())
}

/// A member that is a symbolic link to a directory, reached as a path
/// dependency, is reported.
#[cfg(unix)]
fn check_linked_member_dir_is_reported() -> Result<(), Fail> {
    let sandbox = Sandbox::create(&Name::of("case-linked-member"))?;
    sandbox.seed_members(
        &[Name::of("a")],
        &members_config(&[Name::of("a"), Name::of("lib")]),
    )?;
    // The path dependency must be in the lock file: cargo runs with `--locked`.
    sandbox.write(
        &RelPath::of("Cargo.lock"),
        &Message::of(
            "# This file is automatically @generated by Cargo.\n# It is not intended for manual editing.\nversion = 4\n\n[[package]]\nname = \"a\"\nversion = \"0.1.0\"\ndependencies = [\n \"lib\",\n]\n\n[[package]]\nname = \"lib\"\nversion = \"0.1.0\"\n",
        ),
    )?;
    sandbox.write(
        &RelPath::of("vendor/lib/Cargo.toml"),
        &member_manifest(&Name::of("lib"), &Message::of("")),
    )?;
    sandbox.write(
        &RelPath::of("vendor/lib/src/main.rs"),
        &Message::of("fn main() {}\n"),
    )?;
    sandbox.link(&Link {
        path: RelPath::of("crates/lib"),
        to: RelPath::of("../vendor/lib"),
    })?;
    sandbox.write(
        &RelPath::of("crates/a/Cargo.toml"),
        &member_manifest(
            &Name::of("a"),
            &Message::of("\n[dependencies]\nlib = { path = \"../lib\" }\n"),
        ),
    )?;
    let outcome = run_committed(&sandbox)?;
    ensure!(
        outcome.has_rule_in(&RelPath::of("crates/lib"), &Name::of("stray-lint-config"))
            == Answer::Yes,
        "a linked member directory is reported: {:?} {:?}",
        outcome.stderr(),
        outcome.findings()
    );
    Ok(())
}

/// An ancestor of a member that is a symbolic link to a directory is reported.
#[cfg(unix)]
fn check_linked_ancestor_dir_is_reported() -> Result<(), Fail> {
    let sandbox = Sandbox::create(&Name::of("case-linked-ancestor"))?;
    sandbox.seed_members(&[Name::of("a")], &members_config(&[Name::of("a")]))?;
    sandbox.link(&Link {
        path: RelPath::of("cr"),
        to: RelPath::of("crates"),
    })?;
    sandbox.edit(&Edit {
        file: RelPath::of("Cargo.toml"),
        from: Message::of("members = [\"crates/*\"]"),
        to: Message::of("members = [\"cr/a\"]"),
    })?;
    let outcome = run_committed(&sandbox)?;
    ensure!(
        outcome.has_rule_in(&RelPath::of("cr"), &Name::of("stray-lint-config")) == Answer::Yes,
        "a linked ancestor directory is reported: {:?} {:?}",
        outcome.stderr(),
        outcome.findings()
    );
    Ok(())
}

/// The `.cargo` directory of a member that is a symbolic link is reported.
#[cfg(unix)]
fn check_linked_member_cargo_dir_is_reported() -> Result<(), Fail> {
    let sandbox = Sandbox::create(&Name::of("case-linked-member-cargo"))?;
    sandbox.seed_members(&[Name::of("a")], &members_config(&[Name::of("a")]))?;
    sandbox.write(
        &RelPath::of("realcargo/config.toml"),
        &Message::of("[build]\nrustflags = [\"-C\", \"debuginfo=0\"]\n"),
    )?;
    sandbox.link(&Link {
        path: RelPath::of("crates/a/.cargo"),
        to: RelPath::of("../../realcargo"),
    })?;
    let outcome = run_committed(&sandbox)?;
    ensure!(
        outcome.has_rule_in(
            &RelPath::of("crates/a/.cargo"),
            &Name::of("stray-lint-config")
        ) == Answer::Yes,
        "a linked `.cargo` directory of a member is reported: {:?} {:?}",
        outcome.stderr(),
        outcome.findings()
    );
    Ok(())
}

#[test]
fn stray_lint_config_member_target_dir_is_reported() {
    assert_eq!(
        check_stray_lint_config_member_target_dir_is_reported(),
        Ok(()),
        "stray_lint_config_member_target_dir_is_reported"
    );
}

/// A member whose directory is named `target` is walked like any other member,
/// so a `clippy.toml` in it is reported. The seeded `.gitignore` ignores
/// `target/` and is left as it is: the scope walk reads the working directory,
/// so the ignored file is still reported.
fn check_stray_lint_config_member_target_dir_is_reported() -> Result<(), Fail> {
    let sandbox = Sandbox::create(&Name::of("case-member-target-dir"))?;
    sandbox.seed_members(
        &[Name::of("a")],
        &members_config(&[Name::of("a"), Name::of("tools-target")]),
    )?;
    sandbox.edit(&Edit {
        file: RelPath::of("Cargo.toml"),
        from: Message::of("members = [\"crates/*\"]"),
        to: Message::of("members = [\"crates/*\", \"tools/target\"]"),
    })?;
    sandbox.write(
        &RelPath::of("Cargo.lock"),
        &Message::of(
            "# This file is automatically @generated by Cargo.\n# It is not intended for manual editing.\nversion = 4\n\n[[package]]\nname = \"a\"\nversion = \"0.1.0\"\n\n[[package]]\nname = \"tools-target\"\nversion = \"0.1.0\"\n",
        ),
    )?;
    sandbox.write(
        &RelPath::of("tools/target/Cargo.toml"),
        &Message::of(
            "[package]\nname = \"tools-target\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lints]\nworkspace = true\n",
        ),
    )?;
    sandbox.write(
        &RelPath::of("tools/target/src/main.rs"),
        &Message::of("fn main() {}\n"),
    )?;
    sandbox.write(
        &RelPath::of("tools/target/clippy.toml"),
        &Message::of("cognitive-complexity-threshold = 100\n"),
    )?;
    let outcome = run_committed(&sandbox)?;
    ensure!(
        outcome.has_rule_in(
            &RelPath::of("tools/target/clippy.toml"),
            &Name::of("stray-lint-config")
        ) == Answer::Yes,
        "a clippy.toml in a member directory named `target` is reported: {:?} {:?}",
        outcome.stderr(),
        outcome.findings()
    );
    Ok(())
}

#[test]
fn forbidden_dependency_positive() {
    assert_eq!(
        check_case(&Name::of("forbidden-dependency/positive")),
        Ok(()),
        "forbidden-dependency/positive"
    );
}

#[test]
fn forbidden_dependency_negative() {
    assert_eq!(
        check_case(&Name::of("forbidden-dependency/negative")),
        Ok(()),
        "forbidden-dependency/negative"
    );
}

#[test]
fn forbidden_reference_positive() {
    assert_eq!(
        check_case(&Name::of("forbidden-reference/positive")),
        Ok(()),
        "forbidden-reference/positive"
    );
}

#[test]
fn forbidden_reference_positions() {
    assert_eq!(
        check_case(&Name::of("forbidden-reference/positions")),
        Ok(()),
        "forbidden-reference/positions"
    );
}

#[test]
fn forbidden_reference_negative() {
    assert_eq!(
        check_case(&Name::of("forbidden-reference/negative")),
        Ok(()),
        "forbidden-reference/negative"
    );
}

#[test]
fn baseline_stale_entry() {
    assert_eq!(
        check_case(&Name::of("baseline/stale-entry")),
        Ok(()),
        "baseline/stale-entry"
    );
}

#[test]
fn baseline_forbidden_entry() {
    assert_eq!(
        check_case(&Name::of("baseline/forbidden-entry")),
        Ok(()),
        "baseline/forbidden-entry"
    );
}

#[test]
fn baseline_shrink_only_covered() {
    assert_eq!(
        check_case(&Name::of("baseline/shrink-only-covered")),
        Ok(()),
        "baseline/shrink-only-covered"
    );
}

fn prepared(name: &Name) -> Result<Sandbox, Fail> {
    let package = Name::of("fx");
    let sandbox = Sandbox::create(name)?;
    sandbox.seed(&package, &Sandbox::default_config(&package))?;
    sandbox.write(&RelPath::of("src/main.rs"), &Message::of("fn main() {}\n"))?;
    Ok(sandbox)
}

fn run_committed(sandbox: &Sandbox) -> Result<Outcome, Fail> {
    sandbox.init_repo()?;
    sandbox.run(&[Flag::Range(Message::of("HEAD"))])
}

fn after_edit(name: &Name, edit: &Edit) -> Result<Outcome, Fail> {
    let sandbox = prepared(name)?;
    sandbox.edit(edit)?;
    run_committed(&sandbox)
}

#[test]
fn clean_workspace_has_no_findings() {
    assert_eq!(
        check_clean_workspace_has_no_findings(),
        Ok(()),
        "clean_workspace_has_no_findings"
    );
}

fn check_clean_workspace_has_no_findings() -> Result<(), Fail> {
    let sandbox = prepared(&Name::of("guards-clean"))?;
    let outcome = run_committed(&sandbox)?;
    ensure_eq!(
        outcome.findings(),
        Vec::<Message>::new(),
        "a clean workspace has no findings"
    );
    ensure_eq!(outcome.exit, Exit::Clean, "a clean workspace exits 0");
    ensure_eq!(
        outcome.approvals(),
        Vec::<Message>::new(),
        "nothing changed against HEAD"
    );
    Ok(())
}

#[test]
fn lint_coverage_reports_a_removed_lint() {
    assert_eq!(
        check_lint_coverage_reports_a_removed_lint(),
        Ok(()),
        "lint_coverage_reports_a_removed_lint"
    );
}

fn check_lint_coverage_reports_a_removed_lint() -> Result<(), Fail> {
    let outcome = after_edit(
        &Name::of("guards-lint-removed"),
        &Edit {
            file: RelPath::of("Cargo.toml"),
            from: Message::of("dead_code = \"deny\"\n"),
            to: Message::of(""),
        },
    )?;
    ensure!(
        outcome.has_rule_in(&RelPath::of("Cargo.toml"), &Name::of("lint-coverage")) == Answer::Yes,
        "a lint without a deny line or an omission row is uncovered: {:?}",
        outcome.findings()
    );
    ensure_eq!(outcome.exit, Exit::Findings, "uncovered lint exits 1");
    Ok(())
}

#[test]
fn lint_coverage_reports_a_lowered_lint() {
    assert_eq!(
        check_lint_coverage_reports_a_lowered_lint(),
        Ok(()),
        "lint_coverage_reports_a_lowered_lint"
    );
}

fn check_lint_coverage_reports_a_lowered_lint() -> Result<(), Fail> {
    let outcome = after_edit(
        &Name::of("guards-lint-lowered"),
        &Edit {
            file: RelPath::of("Cargo.toml"),
            from: Message::of("dead_code = \"deny\""),
            to: Message::of("dead_code = \"warn\""),
        },
    )?;
    ensure!(
        outcome.has_rule_in(&RelPath::of("Cargo.toml"), &Name::of("lint-coverage")) == Answer::Yes,
        "a lint set below deny is not set: {:?}",
        outcome.findings()
    );
    Ok(())
}

#[test]
fn lint_coverage_reports_a_group_without_negative_priority() {
    assert_eq!(
        check_lint_coverage_reports_a_group_without_negative_priority(),
        Ok(()),
        "lint_coverage_reports_a_group_without_negative_priority"
    );
}

fn check_lint_coverage_reports_a_group_without_negative_priority() -> Result<(), Fail> {
    let outcome = after_edit(
        &Name::of("guards-lint-priority"),
        &Edit {
            file: RelPath::of("Cargo.toml"),
            from: Message::of("pedantic = { level = \"deny\", priority = -1 }"),
            to: Message::of("pedantic = { level = \"deny\", priority = 0 }"),
        },
    )?;
    ensure!(
        outcome.has_rule_in(&RelPath::of("Cargo.toml"), &Name::of("lint-coverage")) == Answer::Yes,
        "a group needs a negative priority: {:?}",
        outcome.findings()
    );
    Ok(())
}

#[test]
fn lint_coverage_reports_the_restriction_group() {
    assert_eq!(
        check_lint_coverage_reports_the_restriction_group(),
        Ok(()),
        "lint_coverage_reports_the_restriction_group"
    );
}

fn check_lint_coverage_reports_the_restriction_group() -> Result<(), Fail> {
    let outcome = after_edit(
        &Name::of("guards-lint-restriction"),
        &Edit {
            file: RelPath::of("Cargo.toml"),
            from: Message::of("[workspace.lints.clippy]\n"),
            to: Message::of("[workspace.lints.clippy]\nrestriction = \"deny\"\n"),
        },
    )?;
    ensure!(
        outcome.has_rule_in(&RelPath::of("Cargo.toml"), &Name::of("lint-coverage")) == Answer::Yes,
        "the restriction group must never be set wholesale: {:?}",
        outcome.findings()
    );
    Ok(())
}

#[test]
fn lint_coverage_reports_an_unknown_lint() {
    assert_eq!(
        check_lint_coverage_reports_an_unknown_lint(),
        Ok(()),
        "lint_coverage_reports_an_unknown_lint"
    );
}

fn check_lint_coverage_reports_an_unknown_lint() -> Result<(), Fail> {
    let outcome = after_edit(
        &Name::of("guards-lint-unknown"),
        &Edit {
            file: RelPath::of("Cargo.toml"),
            from: Message::of("[workspace.lints.rust]\n"),
            to: Message::of("[workspace.lints.rust]\nnot_a_real_lint = \"deny\"\n"),
        },
    )?;
    ensure!(
        outcome.has_rule_in(&RelPath::of("Cargo.toml"), &Name::of("lint-coverage")) == Answer::Yes,
        "an unknown lint name is reported: {:?}",
        outcome.findings()
    );
    Ok(())
}

#[test]
fn lint_coverage_reports_a_loosened_clippy_knob() {
    assert_eq!(
        check_lint_coverage_reports_a_loosened_clippy_knob(),
        Ok(()),
        "lint_coverage_reports_a_loosened_clippy_knob"
    );
}

fn check_lint_coverage_reports_a_loosened_clippy_knob() -> Result<(), Fail> {
    let outcome = after_edit(
        &Name::of("guards-clippy-knob"),
        &Edit {
            file: RelPath::of("clippy.toml"),
            from: Message::of("max-fn-params-bools = 0"),
            to: Message::of("max-fn-params-bools = 3"),
        },
    )?;
    ensure!(
        outcome.has_rule_in(&RelPath::of("clippy.toml"), &Name::of("lint-coverage")) == Answer::Yes,
        "a clippy.toml knob off its strictest setting is reported: {:?}",
        outcome.findings()
    );
    Ok(())
}

#[test]
fn lint_coverage_reports_an_omission_row_for_a_set_lint() {
    assert_eq!(
        check_lint_coverage_reports_an_omission_row_for_a_set_lint(),
        Ok(()),
        "lint_coverage_reports_an_omission_row_for_a_set_lint"
    );
}

fn check_lint_coverage_reports_an_omission_row_for_a_set_lint() -> Result<(), Fail> {
    let sandbox = prepared(&Name::of("guards-omission-set"))?;
    sandbox.append(
        &RelPath::of(".unrepresentable/ledger.md"),
        &Message::of("| `dead_code` | - | already set, so the row is wrong |\n"),
    )?;
    let outcome = run_committed(&sandbox)?;
    ensure!(
        outcome.has_rule_in(
            &RelPath::of(".unrepresentable/ledger.md"),
            &Name::of("lint-coverage")
        ) == Answer::Yes,
        "an omission row for a lint that is set is reported: {:?}",
        outcome.findings()
    );
    Ok(())
}

#[test]
fn lint_coverage_reports_a_missing_toolchain_pin() {
    assert_eq!(
        check_lint_coverage_reports_a_missing_toolchain_pin(),
        Ok(()),
        "lint_coverage_reports_a_missing_toolchain_pin"
    );
}

fn check_lint_coverage_reports_a_missing_toolchain_pin() -> Result<(), Fail> {
    let sandbox = prepared(&Name::of("guards-toolchain-missing"))?;
    sandbox.remove(&RelPath::of("rust-toolchain.toml"))?;
    let outcome = run_committed(&sandbox)?;
    ensure!(
        outcome.has_rule_in(
            &RelPath::of("rust-toolchain.toml"),
            &Name::of("lint-coverage")
        ) == Answer::Yes,
        "a missing toolchain pin is reported: {:?}",
        outcome.findings()
    );
    Ok(())
}

#[test]
fn lint_coverage_reports_an_unpinned_toolchain() {
    assert_eq!(
        check_lint_coverage_reports_an_unpinned_toolchain(),
        Ok(()),
        "lint_coverage_reports_an_unpinned_toolchain"
    );
}

fn check_lint_coverage_reports_an_unpinned_toolchain() -> Result<(), Fail> {
    let outcome = after_edit(
        &Name::of("guards-toolchain-floating"),
        &Edit {
            file: RelPath::of("rust-toolchain.toml"),
            from: Message::of("channel = \"1.98.1\""),
            to: Message::of("channel = \"stable\""),
        },
    )?;
    ensure!(
        outcome.has_rule_in(
            &RelPath::of("rust-toolchain.toml"),
            &Name::of("lint-coverage")
        ) == Answer::Yes,
        "a floating channel is reported: {:?}",
        outcome.findings()
    );
    Ok(())
}

#[test]
fn manifest_lints_requires_forbidden_unsafe_code() {
    assert_eq!(
        check_manifest_lints_requires_forbidden_unsafe_code(),
        Ok(()),
        "manifest_lints_requires_forbidden_unsafe_code"
    );
}

fn check_manifest_lints_requires_forbidden_unsafe_code() -> Result<(), Fail> {
    let outcome = after_edit(
        &Name::of("guards-unsafe-code"),
        &Edit {
            file: RelPath::of("Cargo.toml"),
            from: Message::of("unsafe_code = \"forbid\"\n"),
            to: Message::of(""),
        },
    )?;
    ensure!(
        outcome.has_rule_in(&RelPath::of("Cargo.toml"), &Name::of("manifest-lints")) == Answer::Yes,
        "the root manifest must forbid unsafe_code: {:?}",
        outcome.findings()
    );
    Ok(())
}

#[test]
fn a_missing_ledger_is_a_usage_error() {
    assert_eq!(
        check_a_missing_ledger_is_a_usage_error(),
        Ok(()),
        "a_missing_ledger_is_a_usage_error"
    );
}

fn check_a_missing_ledger_is_a_usage_error() -> Result<(), Fail> {
    let sandbox = prepared(&Name::of("guards-no-ledger"))?;
    sandbox.remove(&RelPath::of(".unrepresentable/ledger.md"))?;
    let outcome = run_committed(&sandbox)?;
    ensure_eq!(outcome.exit, Exit::Error, "a missing ledger exits 2");
    ensure!(
        outcome
            .stderr()
            .as_str()
            .contains(".unrepresentable/ledger.md"),
        "the error names the ledger: {:?}",
        outcome.stderr()
    );
    Ok(())
}

#[test]
fn an_unknown_config_key_is_a_parse_error() {
    assert_eq!(
        check_an_unknown_config_key_is_a_parse_error(),
        Ok(()),
        "an_unknown_config_key_is_a_parse_error"
    );
}

fn check_an_unknown_config_key_is_a_parse_error() -> Result<(), Fail> {
    let sandbox = prepared(&Name::of("guards-unknown-key"))?;
    sandbox.append(
        &RelPath::of(".unrepresentable/config"),
        &Message::of("\n[nonsense]\nkey = 1\n"),
    )?;
    let outcome = run_committed(&sandbox)?;
    ensure_eq!(
        outcome.exit,
        Exit::Error,
        "an unknown config section exits 2"
    );
    Ok(())
}

#[test]
fn a_member_without_a_role_is_a_usage_error() {
    assert_eq!(
        check_a_member_without_a_role_is_a_usage_error(),
        Ok(()),
        "a_member_without_a_role_is_a_usage_error"
    );
}

fn check_a_member_without_a_role_is_a_usage_error() -> Result<(), Fail> {
    let sandbox = prepared(&Name::of("guards-no-role"))?;
    sandbox.write(
        &RelPath::of(".unrepresentable/config"),
        &Message::of("baseline = \"forbidden\"\ndefault-range = \"HEAD\"\ncode = [\"src/**\"]\n"),
    )?;
    let outcome = run_committed(&sandbox)?;
    ensure_eq!(outcome.exit, Exit::Error, "a member with no role exits 2");
    ensure!(
        outcome.stderr().as_str().contains("has no role"),
        "the error explains the missing role: {:?}",
        outcome.stderr()
    );
    Ok(())
}

#[test]
fn lint_coverage_negative() {
    assert_eq!(
        check_lint_coverage_negative(),
        Ok(()),
        "lint_coverage_negative"
    );
}

fn check_lint_coverage_negative() -> Result<(), Fail> {
    let sandbox = prepared(&Name::of("guards-lint-coverage-negative"))?;
    let outcome = run_committed(&sandbox)?;
    ensure!(
        outcome.has_rule_in(&RelPath::of("Cargo.toml"), &Name::of("lint-coverage")) == Answer::No,
        "a fully covered lint table has no lint-coverage finding: {:?}",
        outcome.findings()
    );
    ensure!(
        outcome.has_rule_in(
            &RelPath::of(".unrepresentable/ledger.md"),
            &Name::of("lint-coverage")
        ) == Answer::No,
        "the ledger's omission rows are all valid: {:?}",
        outcome.findings()
    );
    Ok(())
}

#[test]
fn lint_coverage_rejects_an_unstable_marker_on_a_stable_lint() {
    assert_eq!(
        check_lint_coverage_rejects_an_unstable_marker_on_a_stable_lint(),
        Ok(()),
        "lint_coverage_rejects_an_unstable_marker_on_a_stable_lint"
    );
}

fn check_lint_coverage_rejects_an_unstable_marker_on_a_stable_lint() -> Result<(), Fail> {
    let sandbox = prepared(&Name::of("guards-unstable-marker"))?;
    sandbox.edit(&Edit {
        file: RelPath::of("Cargo.toml"),
        from: Message::of("dead_code = \"deny\"\n"),
        to: Message::of(""),
    })?;
    sandbox.append(
        &RelPath::of(".unrepresentable/ledger.md"),
        &Message::of("| `dead_code` | unstable | claims to exist only on nightly |\n"),
    )?;
    let outcome = run_committed(&sandbox)?;
    ensure!(
        outcome.has_rule_in(
            &RelPath::of(".unrepresentable/ledger.md"),
            &Name::of("lint-coverage")
        ) == Answer::Yes,
        "a stable lint marked unstable is reported: {:?}",
        outcome.findings()
    );
    ensure!(
        outcome
            .stdout()
            .as_str()
            .contains("`dead_code` is marked `unstable`"),
        "the finding names the lint: {}",
        outcome.stdout().as_str()
    );
    Ok(())
}

#[test]
fn preference_omission_positive() {
    assert_eq!(
        check_preference_omission_positive(),
        Ok(()),
        "preference_omission_positive"
    );
}

fn check_preference_omission_positive() -> Result<(), Fail> {
    let sandbox = prepared(&Name::of("guards-preference-positive"))?;
    let outcome = run_committed(&sandbox)?;
    let note = "preference-omission";
    ensure!(
        outcome
            .stdout()
            .as_str()
            .contains(&format!("{note}: `clippy::missing_trait_methods`")),
        "a `-` row prints a preference note: {}",
        outcome.stdout().as_str()
    );
    ensure_eq!(
        outcome.exit,
        Exit::Clean,
        "a preference note never changes the exit code"
    );
    Ok(())
}

#[test]
fn preference_omission_negative() {
    assert_eq!(
        check_preference_omission_negative(),
        Ok(()),
        "preference_omission_negative"
    );
}

fn check_preference_omission_negative() -> Result<(), Fail> {
    let sandbox = prepared(&Name::of("guards-preference-negative"))?;
    let outcome = run_committed(&sandbox)?;
    let shown = outcome.stdout().as_str();
    let note = "preference-omission";
    ensure!(
        !shown.contains(&format!("{note}: `must_not_suspend`")),
        "an `unstable` row is not a preference: {shown}"
    );
    ensure!(
        !shown.contains(&format!("{note}: `clippy::single_call_fn`")),
        "a contradiction row is not a preference: {shown}"
    );
    Ok(())
}

#[test]
fn canary_positive() {
    assert_eq!(check_canary_positive(), Ok(()), "canary_positive");
}

fn check_canary_positive() -> Result<(), Fail> {
    let package = Name::of("fx");
    let sandbox = Sandbox::create(&Name::of("guards-canary-positive"))?;
    sandbox.seed(&package, &Message::of(CANARY_CONFIG))?;
    sandbox.write(&RelPath::of("src/main.rs"), &Message::of("fn main() {}\n"))?;
    sandbox.write(
        &RelPath::of("tests/canaries/bad.rs"),
        &Message::of("// canary role=pure\nfn main() {}\n"),
    )?;
    let outcome = sandbox.run(&[Flag::Canaries])?;
    ensure!(
        outcome.has_rule_in(&RelPath::of("tests/canaries/bad.rs"), &Name::of("canary"))
            == Answer::Yes,
        "a canary without marks is reported: {:?}",
        outcome.findings()
    );
    ensure_eq!(outcome.exit, Exit::Findings, "a broken canary exits 1");
    Ok(())
}

#[test]
fn canary_negative() {
    assert_eq!(check_canary_negative(), Ok(()), "canary_negative");
}

fn check_canary_negative() -> Result<(), Fail> {
    let package = Name::of("fx");
    let sandbox = Sandbox::create(&Name::of("guards-canary-negative"))?;
    sandbox.seed(&package, &Message::of(CANARY_CONFIG))?;
    sandbox.write(&RelPath::of("src/main.rs"), &Message::of("fn main() {}\n"))?;
    sandbox.copy_repo_files(&RelPath::of("tests/canaries"))?;
    let outcome = sandbox.run(&[Flag::Canaries])?;
    ensure_eq!(
        outcome.findings(),
        Vec::<Message>::new(),
        "this repository's canaries all fire where they are marked; stderr: {:?}",
        outcome.stderr()
    );
    ensure_eq!(outcome.exit, Exit::Clean, "correct canaries exit 0");
    Ok(())
}

/// Runs the tool after one edit of the root manifest and returns its output text.
fn stdout_after_manifest_edit(name: &Name, edit: &ManifestEdit) -> Result<Message, Fail> {
    let outcome = after_edit(
        name,
        &Edit {
            file: RelPath::of("Cargo.toml"),
            from: edit.from.clone(),
            to: edit.to.clone(),
        },
    )?;
    Ok(outcome.stdout().clone())
}

#[test]
fn lint_coverage_reports_a_group_allowed_at_a_higher_priority() {
    assert_eq!(
        check_lint_coverage_reports_a_group_allowed_at_a_higher_priority(),
        Ok(()),
        "lint_coverage_reports_a_group_allowed_at_a_higher_priority"
    );
}

fn check_lint_coverage_reports_a_group_allowed_at_a_higher_priority() -> Result<(), Fail> {
    let out = stdout_after_manifest_edit(
        &Name::of("guards-lint-group-allow-high"),
        &ManifestEdit {
            from: Message::of("[workspace.lints.rust]\n"),
            to: Message::of(
                "[workspace.lints.rust]\nnonstandard_style = { level = \"allow\", priority = 5 }\n",
            ),
        },
    )?;
    ensure!(
        out.as_str()
            .contains("lint `non-snake-case` is not set to deny or forbid"),
        "a group allowed above a named deny leaves the lint not denied: {}",
        out.as_str()
    );
    Ok(())
}

#[test]
fn lint_coverage_accepts_a_lint_named_over_an_allowed_group() {
    assert_eq!(
        check_lint_coverage_accepts_a_lint_named_over_an_allowed_group(),
        Ok(()),
        "lint_coverage_accepts_a_lint_named_over_an_allowed_group"
    );
}

fn check_lint_coverage_accepts_a_lint_named_over_an_allowed_group() -> Result<(), Fail> {
    let out = stdout_after_manifest_edit(
        &Name::of("guards-lint-group-allow-equal"),
        &ManifestEdit {
            from: Message::of("[workspace.lints.rust]\n"),
            to: Message::of(
                "[workspace.lints.rust]\nnonstandard_style = { level = \"allow\", priority = -1 }\n",
            ),
        },
    )?;
    ensure!(
        !out.as_str().contains("is not set to deny or forbid"),
        "a lint named at a higher priority than an allowed group stays denied: {}",
        out.as_str()
    );
    Ok(())
}

#[test]
fn lint_coverage_reports_a_group_allowed_over_a_lower_named_deny() {
    assert_eq!(
        check_lint_coverage_reports_a_group_allowed_over_a_lower_named_deny(),
        Ok(()),
        "lint_coverage_reports_a_group_allowed_over_a_lower_named_deny"
    );
}

fn check_lint_coverage_reports_a_group_allowed_over_a_lower_named_deny() -> Result<(), Fail> {
    let out = stdout_after_manifest_edit(
        &Name::of("guards-lint-group-over-named"),
        &ManifestEdit {
            from: Message::of("non_snake_case = \"deny\""),
            to: Message::of("non_snake_case = { level = \"deny\", priority = -2 }\nnonstandard_style = { level = \"allow\", priority = -1 }"),
        },
    )?;
    ensure!(
        out.as_str()
            .contains("lint `non-snake-case` is not set to deny or forbid"),
        "an allowed group above a lower named deny wins: {}",
        out.as_str()
    );
    Ok(())
}

#[test]
fn lint_coverage_keeps_the_tool_prefix_of_other_lint_tables() {
    assert_eq!(
        check_lint_coverage_keeps_the_tool_prefix_of_other_lint_tables(),
        Ok(()),
        "lint_coverage_keeps_the_tool_prefix_of_other_lint_tables"
    );
}

fn check_lint_coverage_keeps_the_tool_prefix_of_other_lint_tables() -> Result<(), Fail> {
    let sandbox = prepared(&Name::of("guards-lint-rustdoc-prefix"))?;
    sandbox.edit(&Edit {
        file: RelPath::of("Cargo.toml"),
        from: Message::of("unused_must_use = \"deny\"\n"),
        to: Message::of(""),
    })?;
    sandbox.edit(&Edit {
        file: RelPath::of("Cargo.toml"),
        from: Message::of("[workspace.lints.clippy]\n"),
        to: Message::of(
            "[workspace.lints.rustdoc]\nunused_must_use = \"deny\"\n\n[workspace.lints.clippy]\n",
        ),
    })?;
    let out = run_committed(&sandbox)?;
    ensure!(
        out.stdout()
            .as_str()
            .contains("lint `unused-must-use` is not set to deny or forbid"),
        "a rustdoc table entry does not cover the rustc lint of the same name: {}",
        out.stdout().as_str()
    );
    Ok(())
}
