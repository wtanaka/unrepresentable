//! Canary.
#[rustfmt::skip]
fn main() {
    let val: u8 = 1;
    let val = val.wrapping_add(1); //~ clippy::shadow_reuse
    sink(val);
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}
