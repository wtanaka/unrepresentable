//! Canary.
#[rustfmt::skip]
fn main() {
    let value: u8 = 1;
    let same = value as u8; //~ clippy::unnecessary_cast clippy::as_conversions trivial_numeric_casts
    assert!(same > 0, "positive");
}
