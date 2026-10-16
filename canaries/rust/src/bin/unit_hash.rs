//! Canary.
use core::hash::Hash as _;
use core::hash::Hasher as _;
use std::collections::hash_map::DefaultHasher;

#[rustfmt::skip]
fn main() {
    let mut hasher = DefaultHasher::new();
    ().hash(&mut hasher); //~ clippy::unit_hash
    assert!(hasher.finish() > 0, "hash");
}
