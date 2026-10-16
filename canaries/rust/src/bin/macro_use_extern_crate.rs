//! Canary.
#[rustfmt::skip]
#[macro_use] //~ macro_use_extern_crate
extern crate alloc;

fn main() {
    let vec: Vec<u8> = vec![];
    assert!(vec.is_empty(), "empty");
}
