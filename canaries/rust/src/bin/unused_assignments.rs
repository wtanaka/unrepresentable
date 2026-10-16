//! Canary.
use core::hint::black_box;

fn main() {
    let mut value: u8 = 0; //~ unused_assignments
    value = 1; //~ unused_assignments
    value = 2;
    let _: u8 = black_box(value);
}
