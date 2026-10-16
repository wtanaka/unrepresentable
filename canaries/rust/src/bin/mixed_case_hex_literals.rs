//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    let num: u32 = 0x1aF; //~ clippy::mixed_case_hex_literals
    sink(num);
}
