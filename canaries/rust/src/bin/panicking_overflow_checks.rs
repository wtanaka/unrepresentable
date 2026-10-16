//! Canary.
#[rustfmt::skip]
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

#[rustfmt::skip]
/// Canary item.
fn check(lhs: u8, rhs: u8) -> bool { //~ clippy::missing_const_for_fn
    lhs + rhs < lhs //~ clippy::panicking_overflow_checks clippy::arithmetic_side_effects
}
#[rustfmt::skip]
fn main() {
    sink(check(1, 2));
}
