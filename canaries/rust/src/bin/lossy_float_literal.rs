//! Canary.
fn main() {
    let val: f32 = 16_777_217.0; //~ clippy::lossy_float_literal
    assert!(val > 1.0, "big");
}
