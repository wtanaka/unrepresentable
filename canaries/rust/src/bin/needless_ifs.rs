//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    let val: u8 = 1;
    if val == 1 {} //~ clippy::needless_ifs
    sink(val);
}
