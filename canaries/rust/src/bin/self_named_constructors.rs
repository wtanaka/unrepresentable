//! Canary.
#[rustfmt::skip]
/// Canary item.
struct Unit;

#[rustfmt::skip]
impl Unit {
    /// Canary item.
    fn unit() -> Self { //~ clippy::self_named_constructors clippy::missing_const_for_fn
        Self
    }
}

#[rustfmt::skip]
fn main() {
    sink(Unit::unit());
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}
