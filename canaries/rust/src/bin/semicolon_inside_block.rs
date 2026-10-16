//! Canary.
#[rustfmt::skip]
/// Canary item.
fn nothing() {
    drop(String::new());
}

#[rustfmt::skip]
fn main() {
    { nothing() }; //~ clippy::semicolon_inside_block clippy::unnecessary_operation
}
