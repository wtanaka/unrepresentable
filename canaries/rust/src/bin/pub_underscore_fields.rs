//! Canary.
#[rustfmt::skip]
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
/// Canary item.
pub struct Holder {
    /// Canary item.
    pub _hidden: u8, //~ clippy::pub_underscore_fields
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}

#[rustfmt::skip]
fn main() {
    sink(Holder { _hidden: 1 });
}
