//! Canary.
use core::sync::atomic::{AtomicU8, Ordering};

#[rustfmt::skip]
/// Canary item.
const ATOM: AtomicU8 = AtomicU8::new(0); //~ clippy::declare_interior_mutable_const

#[rustfmt::skip]
fn main() {
    ATOM.store(1, Ordering::Relaxed); //~ const_item_interior_mutations clippy::borrow_interior_mutable_const
}
