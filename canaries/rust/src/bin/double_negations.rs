//! Canary.
/// Canary item.
fn sink<Ty>(_val: Ty) {}

#[rustfmt::skip]
fn main() {
    let val: i32 = --1; //~ double_negations
    sink(val);
}
