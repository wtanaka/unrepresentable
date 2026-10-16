//! Canary.
use core::cell::Cell;

thread_local! {
    static COUNTER: Cell<u8> = Cell::new(1); //~ clippy::missing_const_for_thread_local
}

fn main() {
    assert_eq!(COUNTER.get(), 1, "counter");
}
