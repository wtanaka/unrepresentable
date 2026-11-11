//! Canary.
use core::hint::black_box;

fn main() {
    let number: u8 = black_box(1);
    let _: u8 = match { number } { //~ clippy::blocks_in_conditions clippy::block_scrutinee
        1 => 2,
        _ => 3,
    };
}
