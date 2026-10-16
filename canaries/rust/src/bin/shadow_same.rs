//! Canary.
#[rustfmt::skip]
fn main() {
    let val: u8 = 1;
    let val = val; //~ clippy::shadow_same clippy::redundant_locals
    sink(val);
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}
