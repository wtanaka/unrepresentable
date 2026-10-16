//! Canary.
/// Canary item.
fn sink<Ty>(_val: Ty) {}

#[rustfmt::skip]
fn main() {
    let num: i32 = 2;
    sink(-1.max(num)); //~ ambiguous_negative_literals clippy::arithmetic_side_effects clippy::default_numeric_fallback
}
