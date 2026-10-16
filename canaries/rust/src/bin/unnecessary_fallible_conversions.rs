//! Canary.
#[rustfmt::skip]
fn main() {
    let small: u8 = 1;
    let wide: u64 = small.try_into().unwrap(); //~ clippy::unnecessary_fallible_conversions
    assert!(wide > 0, "positive");
}
