//! Canary.
use std::env::args;

fn main() {
    let count = args().count();
    let top = [count].into_iter().max(); //~ clippy::iter_on_single_items
    assert!(top.is_some(), "one");
}
