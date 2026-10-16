//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    let num: u8 = 1_32; //~ clippy::mistyped_literal_suffixes
    sink(num);
}
