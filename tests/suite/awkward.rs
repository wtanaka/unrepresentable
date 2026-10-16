//! Awkward but legal Rust: macros, raw strings, nested `cfg_attr`, inner attributes.

use crate::support::{check_case, Fail, Flag, Message, Name, RelPath, Sandbox};

#[test]
fn awkward_proptest_body() {
    assert_eq!(
        check_case(&Name::of("awkward/proptest-body")),
        Ok(()),
        "awkward/proptest-body"
    );
}

#[test]
fn awkward_macro_rules_bodies() {
    assert_eq!(
        check_case(&Name::of("awkward/macro-rules-bodies")),
        Ok(()),
        "awkward/macro-rules-bodies"
    );
}

#[test]
fn awkward_local_macro_in_fn() {
    assert_eq!(
        check_case(&Name::of("awkward/local-macro-in-fn")),
        Ok(()),
        "awkward/local-macro-in-fn"
    );
}

#[test]
fn awkward_aliased_and_glob_imports() {
    assert_eq!(
        check_case(&Name::of("awkward/aliased-and-glob-imports")),
        Ok(()),
        "awkward/aliased-and-glob-imports"
    );
}

#[test]
fn awkward_multi_line_signature() {
    assert_eq!(
        check_case(&Name::of("awkward/multi-line-signature")),
        Ok(()),
        "awkward/multi-line-signature"
    );
}

#[test]
fn awkward_strings_and_comments() {
    assert_eq!(
        check_case(&Name::of("awkward/strings-and-comments")),
        Ok(()),
        "awkward/strings-and-comments"
    );
}

#[test]
fn awkward_cfg_attr_nested_twice() {
    assert_eq!(
        check_case(&Name::of("awkward/cfg-attr-nested-twice")),
        Ok(()),
        "awkward/cfg-attr-nested-twice"
    );
}

#[test]
fn awkward_inner_expect_attribute() {
    assert_eq!(
        check_case(&Name::of("awkward/inner-expect-attribute")),
        Ok(()),
        "awkward/inner-expect-attribute"
    );
}

#[test]
fn awkward_trait_and_impl_signatures() {
    assert_eq!(
        check_case(&Name::of("awkward/trait-and-impl-signatures")),
        Ok(()),
        "awkward/trait-and-impl-signatures"
    );
}

#[test]
fn item_labels_include_the_trait_generic_arguments() {
    assert_eq!(
        check_item_labels_include_the_trait_generic_arguments(),
        Ok(()),
        "item_labels_include_the_trait_generic_arguments"
    );
}

fn check_item_labels_include_the_trait_generic_arguments() -> Result<(), Fail> {
    let package = Name::of("fx");
    let sandbox = Sandbox::create(&Name::of("awkward-item-labels"))?;
    sandbox.seed(&package, &Sandbox::default_config(&package))?;
    sandbox.overlay(&RelPath::of("item-labels/generic-impls"))?;
    sandbox.init_repo()?;
    let outcome = sandbox.run(&[Flag::Range(Message::of("HEAD"))])?;
    let shown = outcome.stdout().as_str();
    ensure!(
        shown.contains("impl From<A> for Y / fn from"),
        "the From<A> impl has its own label: {shown}"
    );
    ensure!(
        shown.contains("impl From<B> for Y / fn from"),
        "the From<B> impl has its own label: {shown}"
    );
    Ok(())
}
