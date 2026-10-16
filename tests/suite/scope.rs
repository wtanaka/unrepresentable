//! Scope resolution: which files are scanned, and as production or test code.

use crate::support::{check_case, Fail, Flag, Message, Name, RelPath, Sandbox};

#[test]
fn unscannable_include_non_literal() {
    assert_eq!(
        check_case(&Name::of("unscannable-include/non-literal")),
        Ok(()),
        "unscannable-include/non-literal"
    );
}

#[test]
fn unscannable_include_literal() {
    assert_eq!(
        check_case(&Name::of("unscannable-include/literal")),
        Ok(()),
        "unscannable-include/literal"
    );
}

#[test]
fn unscannable_include_literal_with_finding() {
    assert_eq!(
        check_case(&Name::of("unscannable-include/literal-with-finding")),
        Ok(()),
        "unscannable-include/literal-with-finding"
    );
}

#[test]
fn unreached_file_positive() {
    assert_eq!(
        check_case(&Name::of("unreached-file/positive")),
        Ok(()),
        "unreached-file/positive"
    );
}

#[test]
fn unreached_file_negative() {
    assert_eq!(
        check_case(&Name::of("unreached-file/negative")),
        Ok(()),
        "unreached-file/negative"
    );
}

#[test]
fn unreached_file_in_a_hidden_directory() {
    assert_eq!(
        check_case(&Name::of("unreached-file/hidden-dir")),
        Ok(()),
        "unreached-file/hidden-dir"
    );
}

#[test]
fn unreached_file_skips_vcs_and_target_directories() {
    assert_eq!(
        check_unreached_file_skips_vcs_and_target_directories(),
        Ok(()),
        "unreached_file_skips_vcs_and_target_directories"
    );
}

fn check_unreached_file_skips_vcs_and_target_directories() -> Result<(), Fail> {
    let package = Name::of("fx");
    let sandbox = Sandbox::create(&Name::of("scope-skipped-dirs"))?;
    sandbox.seed(&package, &Sandbox::default_config(&package))?;
    sandbox.write(&RelPath::of("src/main.rs"), &Message::of("fn main() {}\n"))?;
    sandbox.write(
        &RelPath::of(".jj/stray.rs"),
        &Message::of("fn stray() {}\n"),
    )?;
    sandbox.write(
        &RelPath::of("target/stray.rs"),
        &Message::of("fn stray() {}\n"),
    )?;
    sandbox.init_repo()?;
    let outcome = sandbox.run(&[Flag::Range(Message::of("HEAD"))])?;
    ensure_eq!(
        outcome.findings(),
        Vec::<Message>::new(),
        "files under .jj and target are not unreached"
    );
    Ok(())
}

#[test]
fn scope_cfg_test_mod_at_top() {
    assert_eq!(
        check_case(&Name::of("scope/cfg-test-mod-at-top")),
        Ok(()),
        "scope/cfg-test-mod-at-top"
    );
}

#[test]
fn scope_cfg_all_test_feature_unwrap() {
    assert_eq!(
        check_case(&Name::of("scope/cfg-all-test-feature-unwrap")),
        Ok(()),
        "scope/cfg-all-test-feature-unwrap"
    );
}

#[test]
fn scope_cfg_test_path_attr() {
    assert_eq!(
        check_case(&Name::of("scope/cfg-test-path-attr")),
        Ok(()),
        "scope/cfg-test-path-attr"
    );
}

#[test]
fn scope_extra_bin_target() {
    assert_eq!(
        check_case(&Name::of("scope/extra-bin-target")),
        Ok(()),
        "scope/extra-bin-target"
    );
}

#[test]
fn scope_examples_and_benches() {
    assert_eq!(
        check_case(&Name::of("scope/examples-and-benches")),
        Ok(()),
        "scope/examples-and-benches"
    );
}

#[test]
fn scope_integration_test_target() {
    assert_eq!(
        check_case(&Name::of("scope/integration-test-target")),
        Ok(()),
        "scope/integration-test-target"
    );
}

#[test]
fn scope_inline_cfg_test_mod() {
    assert_eq!(
        check_case(&Name::of("scope/inline-cfg-test-mod")),
        Ok(()),
        "scope/inline-cfg-test-mod"
    );
}

#[test]
fn scope_unreached_rs_file() {
    assert_eq!(
        check_case(&Name::of("scope/unreached-rs-file")),
        Ok(()),
        "scope/unreached-rs-file"
    );
}

#[test]
fn scope_include_non_literal() {
    assert_eq!(
        check_case(&Name::of("scope/include-non-literal")),
        Ok(()),
        "scope/include-non-literal"
    );
}

#[test]
fn scope_nested_inline_modules_with_path() {
    assert_eq!(
        check_case(&Name::of("scope/nested-inline-modules-with-path")),
        Ok(()),
        "scope/nested-inline-modules-with-path"
    );
}

#[test]
fn scope_mod_rs_layouts() {
    assert_eq!(
        check_case(&Name::of("scope/mod-rs-layouts")),
        Ok(()),
        "scope/mod-rs-layouts"
    );
}

#[test]
fn scope_cfg_any_and_not_test_are_production() {
    assert_eq!(
        check_case(&Name::of("scope/cfg-any-and-not-test-are-production")),
        Ok(()),
        "scope/cfg-any-and-not-test-are-production"
    );
}

#[test]
fn scope_nested_cfg_test_all_inline() {
    assert_eq!(
        check_case(&Name::of("scope/nested-cfg-test-all-inline")),
        Ok(()),
        "scope/nested-cfg-test-all-inline"
    );
}
