//! Canary.
use core::hint::black_box;

fn main() {
    let value: u8 = 1;
    let _: bool = black_box(value >= 0); //~ unused_comparisons clippy::absurd_extreme_comparisons
}
