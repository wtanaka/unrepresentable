//! Canary.
#[rustfmt::skip]
fn main() {
    let first: f64 = 1.0;
    let second: f64 = 2.0;
    let third: f64 = 3.0;
    let result = first * second + third; //~ clippy::suboptimal_flops clippy::float_arithmetic
    assert!(result > first, "bigger");
}
