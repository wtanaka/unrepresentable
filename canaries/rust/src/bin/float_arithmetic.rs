//! Canary.
#[rustfmt::skip]
fn main() {
    let value: f32 = 1.0;
    let sum = value + value; //~ clippy::float_arithmetic
    assert!(sum > value, "bigger");
}
