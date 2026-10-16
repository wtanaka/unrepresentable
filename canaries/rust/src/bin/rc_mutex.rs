//! Canary.
use std::rc::Rc; //~ clippy::std_instead_of_alloc
use std::sync::Mutex;

#[rustfmt::skip]
/// Canary item.
struct Holder {
    /// Canary item.
    lock: Rc<Mutex<u8>>, //~ clippy::rc_mutex
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}

#[rustfmt::skip]
fn main() {
    let holder = Holder {
        lock: Rc::new(Mutex::new(1)),
    };
    sink(&holder.lock);
}
