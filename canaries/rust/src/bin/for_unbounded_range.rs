//! Canary.
use core::cell::Cell;

#[rustfmt::skip]
fn main() {
    let total: Cell<usize> = Cell::new(0);
    let start: usize = 0;
    for index in start.. { //~ clippy::for_unbounded_range
        total.set(index);
    }
}
