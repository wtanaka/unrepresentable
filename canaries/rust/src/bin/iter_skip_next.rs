//! Canary.
use std::env::args;

fn main() {
    let data: Vec<u8> = Vec::new();
    let count = args().count();
    let item = data.iter().skip(count).next(); //~ clippy::iter_skip_next
    assert!(item.is_none(), "empty");
}
