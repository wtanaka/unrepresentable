//! Canary.
#[rustfmt::skip]
fn main() {
    drop("a".repeat(1)); //~ clippy::repeat_once
}
