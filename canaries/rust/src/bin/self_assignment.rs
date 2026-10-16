//! Canary.
#[rustfmt::skip]
fn main() {
    let mut val: u8 = 1;
    val = val; //~ clippy::self_assignment dead_code
    sink(val);
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}
