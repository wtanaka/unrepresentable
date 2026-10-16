//! Canary.
use core::hint::black_box;

use std::vec::Vec; //~ redundant_imports clippy::std_instead_of_alloc

fn main() {
    let _unused: Vec<u8> = black_box(Vec::new());
}
