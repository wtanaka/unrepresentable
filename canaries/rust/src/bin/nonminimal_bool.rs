//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    let lhs: u8 = 1;
    let rhs: u8 = 2;
    sink(!(lhs == rhs)); //~ clippy::nonminimal_bool
}
