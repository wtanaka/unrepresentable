//! Canary.
use std::rc::Rc; //~ clippy::std_instead_of_alloc

#[rustfmt::skip]
/// Canary item.
struct Holder {
    /// Canary item.
    text: Rc<String>, //~ clippy::rc_buffer
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}

#[rustfmt::skip]
fn main() {
    let holder = Holder {
        text: Rc::new(String::new()),
    };
    sink(&holder.text);
}
