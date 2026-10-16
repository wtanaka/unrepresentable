//! Canary.
#[rustfmt::skip]
fn main() {
    let mut value: i8 = 1;
    assert!(value > 0, "positive");
    value =- 1; //~ clippy::suspicious_assignment_formatting
    assert_eq!(value, -1, "negative");
}
