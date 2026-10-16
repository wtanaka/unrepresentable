//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    let val: i32 = 1;
    let res = val * -1; //~ clippy::neg_multiply clippy::default_numeric_fallback
    sink(res);
}
