//! Canary.
use core::cell::Cell;
use std::process::exit;

/// Canary item.
const CELL: Cell<u8> = Cell::new(1); //~ clippy::declare_interior_mutable_const

fn main() {
    let cell = CELL;
    exit(i32::from(cell.get() == 2));
}
