//! Canary.
extern crate alloc;

use alloc::collections::LinkedList;

/// Canary item.
struct Holder {
    /// Canary item.
    list: LinkedList<u8>, //~ clippy::linkedlist
}

fn main() {
    let holder = Holder {
        list: LinkedList::new(),
    };
    assert!(holder.list.is_empty(), "empty");
}
