//! Canary.
#[rustfmt::skip]
use std::collections::{HashMap}; //~ unused_import_braces

fn main() {
    let _unused: HashMap<u8, u8> = HashMap::new();
}
