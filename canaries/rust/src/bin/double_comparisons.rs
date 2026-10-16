//! Canary.
#[rustfmt::skip]
fn main() {
    let value: u8 = 3;
    let both = value < 5 || value == 5; //~ clippy::double_comparisons
    assert!(both, "both");
}
