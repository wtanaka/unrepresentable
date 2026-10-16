//! Canary.
#[rustfmt::skip]
#[derive(Clone, Copy)]
/// Canary item.
struct Unit;

#[rustfmt::skip]
impl PartialEq for Unit {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
    fn ne(&self, _other: &Self) -> bool { //~ clippy::partialeq_ne_impl
        false
    }
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}

#[rustfmt::skip]
fn main() {
    let first = Unit;
    let second = Unit;
    sink(first == second);
}
