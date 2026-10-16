//! Canary.
use core::hint::black_box;

fn main() {
    let value: u8 = 1;
    let _: u8 = black_box({ value }); //~ unused_braces
}
