//! Canary.
#[rustfmt::skip]
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

#[rustfmt::skip]
/// Canary item.
fn length(val: String) -> usize { //~ clippy::needless_pass_by_value
    val.len()
}
#[rustfmt::skip]
fn main() {
    sink(length(String::new()));
}
