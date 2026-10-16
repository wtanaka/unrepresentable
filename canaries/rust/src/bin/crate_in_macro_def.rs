//! Canary.
#[macro_export]
/// Canary item.
macro_rules! call_foo {
    () => {
        crate::foo() //~ clippy::crate_in_macro_def
    };
}

/// Canary item.
const fn foo() {}

fn main() {
    foo();
}
