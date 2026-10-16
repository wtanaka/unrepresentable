//! Canary.
#[rustfmt::skip]
/// Canary item.
fn first(opt: Option<u8>) -> Option<u8> {
    opt.and_then(|val| val.checked_add(1)) //~ clippy::return_and_then
}

#[rustfmt::skip]
fn main() {
    sink(first(None));
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}
