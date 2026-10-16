//! Canary.
fn main() {
    let value: f32 = 0.0 / 0.0; //~ clippy::zero_divided_by_zero clippy::float_arithmetic clippy::eq_op
    let _v = value.to_string();
}
