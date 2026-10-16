//! Canary.
use std::collections::HashSet;

#[rustfmt::skip]
fn main() {
    let set: HashSet<u8> = HashSet::new();
    for item in &set { //~ clippy::iter_over_hash_type
        assert_eq!(*item, 1, "one");
    }
}
