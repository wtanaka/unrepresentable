//! Canary.
#[rustfmt::skip]
/// Canary item.
fn nothing() {
    drop(String::new());
}

#[rustfmt::skip]
fn main() {
    nothing() //~ clippy::semicolon_if_nothing_returned
}
