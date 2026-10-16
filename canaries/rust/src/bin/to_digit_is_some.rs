//! Canary.
#[rustfmt::skip]
fn main() {
    let value = '1';
    assert!(value.to_digit(10).is_some(), "digit"); //~ clippy::to_digit_is_some
}
