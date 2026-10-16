//! Canary.
use std::rc::Rc; //~ clippy::std_instead_of_alloc

/// Canary item.
struct Thing(Rc<u8>); //~ dead_code

                      // SAFETY: not actually safe.
unsafe impl Send for Thing {} //~ unsafe_code clippy::non_send_fields_in_send_ty

/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    sink(Thing(Rc::new(1)));
}
