//! Canary.
use std::env::args;

fn main() {
    let data: Vec<u8> = Vec::new();
    let count = args().count();
    let item = data.iter().nth(count); //~ clippy::iter_nth
    assert!(item.is_none(), "empty");
}
