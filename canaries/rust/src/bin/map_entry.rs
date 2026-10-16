//! Canary.
use std::collections::HashMap;

#[rustfmt::skip]
/// Canary item.
fn add(map: &mut HashMap<u8, u8>, key: u8) {
    if !map.contains_key(&key) { //~ clippy::map_entry
        map.insert(key, 1); //~ unused_results
    }
}

fn main() {
    let mut map: HashMap<u8, u8> = HashMap::new();
    add(&mut map, 1);
    assert_eq!(map.len(), 1, "len");
}
