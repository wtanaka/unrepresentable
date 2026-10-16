//! Canary.
#[rustfmt::skip]
fn main() {
    let value: f32 = 1.123_456_789; //~ clippy::excessive_precision
    assert!(value > 1.0, "big");
}
