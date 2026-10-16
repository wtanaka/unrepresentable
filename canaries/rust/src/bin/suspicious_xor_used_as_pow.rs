//! Canary.
#[rustfmt::skip]
fn main() {
    let value: u32 = 2 ^ 8; //~ clippy::suspicious_xor_used_as_pow clippy::suspicious_xor_used_as_pow
    assert!(value > 0, "positive");
}
