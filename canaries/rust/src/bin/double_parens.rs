//! Canary.
#[rustfmt::skip]
fn main() {
    let flag = true;
    assert!(((flag)), "flag"); //~ clippy::double_parens
}
