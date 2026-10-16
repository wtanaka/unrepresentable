//! Canary.
use core::iter;

/// Canary item.
fn many(count: usize) -> Vec<u8> {
    iter::repeat(1).take(count).collect() //~ clippy::manual_repeat_n
}

fn main() {
    assert_eq!(many(2).len(), 2, "many");
}
