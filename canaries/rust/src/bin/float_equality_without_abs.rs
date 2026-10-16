//! Canary.
#[rustfmt::skip]
fn main() {
    let left: f32 = 1.0;
    let right: f32 = 1.0;
    assert!((left - right) < f32::EPSILON, "close"); //~ clippy::float_equality_without_abs clippy::float_arithmetic
}
