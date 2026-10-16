//! Canary.
use std::process::exit;

#[rustfmt::skip]
fn main() {
    let mut aa: u8 = 1;
    let mut bb: u8 = 2;
    let before = aa == bb;
    aa = bb; bb = aa; //~ clippy::almost_swapped
    exit(i32::from(before != (aa == bb)));
}
