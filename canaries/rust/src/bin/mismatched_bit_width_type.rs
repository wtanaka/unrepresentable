//! Canary.
use core::hint::black_box;

fn main() {
    let number: u32 = black_box(7);
    let width = u8::BITS - number.leading_zeros(); //~ clippy::arithmetic_side_effects clippy::mismatched_bit_width_type
    let _: u32 = black_box(width);
}
