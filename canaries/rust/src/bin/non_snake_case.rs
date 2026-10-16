//! Canary.
use core::hint::black_box;

fn main() {
    let myVar: u8 = 1; //~ non_snake_case
    let _: u8 = black_box(myVar);
}
