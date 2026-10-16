//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    let lhs: u32 = 5;
    let rem: u32 = lhs % 1; //~ clippy::modulo_one clippy::integer_division_remainder_used
    sink(rem);
}
