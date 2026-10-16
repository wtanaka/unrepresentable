//! Canary.
use core::hash::Hash;
use std::collections::HashMap;

/// Canary item.
pub trait Count {
    /// Canary item.
    fn count(&self) -> usize;
}

#[rustfmt::skip]
impl<Key, Val> Count for HashMap<Key, Val> //~ clippy::implicit_hasher
where
    Key: Hash + Eq,
{
    #[inline]
    fn count(&self) -> usize {
        self.len()
    }
}

fn main() {
    let map: HashMap<u8, u8> = HashMap::new();
    assert_eq!(map.count(), 0, "empty");
}
