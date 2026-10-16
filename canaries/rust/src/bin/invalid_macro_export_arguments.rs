//! Canary.
#[rustfmt::skip]
#[macro_export(hello)] //~ invalid_macro_export_arguments
/// Canary item.
macro_rules! mac {
    () => {};
}

fn main() {}
