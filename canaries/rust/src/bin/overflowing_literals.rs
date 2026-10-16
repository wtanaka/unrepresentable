//! Canary.
use core::hint::black_box;

fn main() {
    let _: u8 = black_box(256); //~ overflowing_literals
}
