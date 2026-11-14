//! Canary.
use core::hint::black_box;

fn main() {
    let é: u8 = 1; //~ non_ascii_idents clippy::many_single_char_names clippy::min_ident_chars
    let _: u8 = black_box(é);
}
