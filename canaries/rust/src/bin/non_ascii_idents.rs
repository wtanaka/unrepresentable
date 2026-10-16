//! Canary.
use core::hint::black_box;

fn main() {
    let é: u8 = 1; //~ non_ascii_idents
    let _: u8 = black_box(é);
}
