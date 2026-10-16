//! Canary.
#[rustfmt::skip]
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

#[rustfmt::skip]
/// Canary item.
fn rec(lhs: u8, rhs: u8) -> u8 { //~ clippy::only_used_in_recursion
    if lhs == 0 {
        return 0;
    }
    rec(lhs, rhs)
}
#[rustfmt::skip]
fn main() {
    sink(rec(1, 2));
}
