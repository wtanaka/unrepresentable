//! Canary.
use std::collections::HashMap;

fn main() {
    let map: HashMap<u8, u8> = HashMap::new();
    let top = map.iter().map(|(key, _)| *key).max(); //~ clippy::iter_kv_map
    assert!(top.is_none(), "empty");
}
