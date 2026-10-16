//! Canary.
use core::hint::black_box;

fn main() {
    let item: u8 = 1;
    let _: usize = black_box(Box::new([item]).len()); //~ unused_allocation
}
