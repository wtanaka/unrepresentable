//! Canary.
use core::sync::atomic::{AtomicU8, Ordering};

#[rustfmt::skip]
fn main() {
    let atom = AtomicU8::new(0);
    let val = atom.load(Ordering::Release); //~ invalid_atomic_ordering
    assert_eq!(val, 0, "value");
}
