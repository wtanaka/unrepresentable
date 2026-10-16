//! Canary.
use core::cell::Cell;
use std::process::exit;

/// Canary item.
struct Wrap;

impl Wrap {
    /// Canary item.
    const CELL: Cell<u8> = Cell::new(1); //~ clippy::declare_interior_mutable_const
}

fn main() {
    let rf = &Wrap::CELL; //~ clippy::borrow_interior_mutable_const
    exit(i32::from(rf.get() == 2));
}
