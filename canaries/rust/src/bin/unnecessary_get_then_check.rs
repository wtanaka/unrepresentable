//! Canary.
use std::collections::HashSet;
fn main() {
    let src: HashSet<i32> = HashSet::new();
    let key: i32 = 1;
    let _v = src.get(&key).is_some(); //~ clippy::unnecessary_get_then_check
}
