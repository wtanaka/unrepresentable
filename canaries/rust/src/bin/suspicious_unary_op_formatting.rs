//! Canary.
#[rustfmt::skip]
fn main() {
    let value: i8 = 1;
    assert!(value >- 30, "big"); //~ clippy::suspicious_unary_op_formatting
}
