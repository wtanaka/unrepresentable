//! Canary.
use core::hash::{BuildHasher as _, Hash as _, Hasher as _};
use std::collections::hash_map::RandomState;

/// Canary item.
fn digest(value: u8) -> u64 {
    let mut hasher = RandomState::new().build_hasher();
    value.hash(&mut hasher);
    let hash = hasher.finish(); //~ clippy::manual_hash_one
    hash.max(1)
}

fn main() {
    assert!(digest(1) > 0, "digest");
}
