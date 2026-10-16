//! Canary.
use core::hint::black_box;

fn main() {
    let divisor: u8 = 0;
    let _: u8 = black_box(1 / divisor); //~ unconditional_panic clippy::integer_division clippy::integer_division_remainder_used
}
