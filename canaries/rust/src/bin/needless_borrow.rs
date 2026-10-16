//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

/// Canary item.
fn takes(val: &String) {
    sink(val);
}
fn main() {
    let owned = String::new();
    let rf = &owned;
    takes(&rf); //~ clippy::needless_borrow
}
