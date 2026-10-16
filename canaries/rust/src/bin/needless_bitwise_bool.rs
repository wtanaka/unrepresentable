//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    let lhs: u8 = 1;
    let rhs: u8 = 2;
    let res = (lhs > rhs) & (rhs > lhs); //~ clippy::needless_bitwise_bool
    sink(res);
}
