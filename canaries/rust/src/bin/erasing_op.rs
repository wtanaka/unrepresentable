//! Canary.
#[rustfmt::skip]
fn main() {
    let value: u8 = 3;
    let zero = value * 0; //~ clippy::erasing_op
    assert!(zero == 0, "zero");
}
