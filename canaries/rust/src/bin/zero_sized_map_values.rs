//! Canary.
use std::collections::HashMap;
fn main() {
    let src: HashMap<i32, ()> = HashMap::new(); //~ clippy::zero_sized_map_values
    let _v = src.len();
}
