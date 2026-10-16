//! Canary.
#[rustfmt::skip]
use core::hint::black_box;

#[rustfmt::skip]
fn main() {
    let _: u8 = black_box(1);; //~ redundant_semicolons
}
