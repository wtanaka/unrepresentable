//! Canary.
use core::hint::black_box;

fn main() {
    let value: u8 = (1); //~ unused_parens
    let _: u8 = black_box(value);
}
