//! Canary.
#[rustfmt::skip]
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

#[rustfmt::skip]
/// Canary item.
fn fail() -> Result<(), u8> { //~ clippy::panic_in_result_fn
    panic!(); //~ clippy::panic
}
#[rustfmt::skip]
fn main() {
    sink(fail());
}
