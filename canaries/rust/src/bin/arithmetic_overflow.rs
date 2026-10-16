//! Canary.
/// Canary item.
fn sink<Ty>(_val: Ty) {}

#[rustfmt::skip]
fn main() {
    let val: u8 = 255 + 1; //~ arithmetic_overflow
    sink(val);
}
