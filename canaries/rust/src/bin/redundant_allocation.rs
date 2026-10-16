//! Canary.
use std::rc::Rc; //~ clippy::std_instead_of_alloc

#[rustfmt::skip]
/// Canary item.
struct Holder {
    /// Canary item.
    boxed: Box<Rc<u8>>, //~ clippy::redundant_allocation
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}

#[rustfmt::skip]
fn main() {
    let holder = Holder {
        boxed: Box::new(Rc::new(1)),
    };
    sink(&holder.boxed);
}
