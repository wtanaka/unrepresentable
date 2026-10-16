//! Canary.
use core::cell::Cell;
use std::collections::BTreeSet; //~ clippy::std_instead_of_alloc

#[rustfmt::skip]
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

#[rustfmt::skip]
/// Canary item.
fn takes(set: BTreeSet<Cell<u8>>) { //~ clippy::mutable_key_type
    sink(set);
}
#[rustfmt::skip]
fn main() {
    takes(BTreeSet::new());
}
