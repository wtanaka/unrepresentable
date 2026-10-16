//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    let lhs: i32 = -1;
    let rem: i32 = lhs % 3; //~ clippy::modulo_arithmetic clippy::integer_division_remainder_used
    sink(rem);
}
