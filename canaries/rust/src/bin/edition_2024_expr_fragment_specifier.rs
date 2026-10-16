//! Canary.
#[rustfmt::skip]
/// Canary item.
macro_rules! mac {
    ($val:expr) => {}; //~ edition_2024_expr_fragment_specifier
}

fn main() {
    mac!(true);
}
