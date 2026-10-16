//! Canary.
#[rustfmt::skip]
fn main() {
    let value: u8 = 1;
    assert!(value == value, "equal"); //~ clippy::eq_op
}
