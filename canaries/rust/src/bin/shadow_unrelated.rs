//! Canary.
#[rustfmt::skip]
fn main() {
    let val: u8 = 1;
    sink(val);
    let val: u8 = 2; //~ clippy::shadow_unrelated
    sink(val);
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}
