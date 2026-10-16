//! Canary.
use std::collections::HashMap;

#[rustfmt::skip]
fn main() {
    let mut map: HashMap<u8, u8> = HashMap::new();
    assert!(map.insert(1, 2).is_none(), "new");
    for (key, _) in &map { //~ clippy::for_kv_map clippy::iter_over_hash_type
        assert!(*key > 0, "positive");
    }
}
