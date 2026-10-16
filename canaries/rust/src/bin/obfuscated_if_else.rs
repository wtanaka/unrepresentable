//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    let cond = true;
    let one: u8 = 1;
    let two: u8 = 2;
    sink(cond.then_some(one).unwrap_or(two)); //~ clippy::obfuscated_if_else
}
