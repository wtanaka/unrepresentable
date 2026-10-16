//! Canary.
#[rustfmt::skip]
fn main() {
    let value: u8 = 3;
    let same = value + 0; //~ clippy::identity_op
    assert!(same == value, "same");
}
