//! Canary.
#[rustfmt::skip]
fn main() {
    let val: u8 = (|| 1)(); //~ clippy::redundant_closure_call
    sink(val);
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}
