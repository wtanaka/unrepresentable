//! Canary.
fn main() {
    let i: u8 = 1; //~ clippy::many_single_char_names clippy::min_ident_chars
    let j: u8 = 2; //~ clippy::min_ident_chars
    let x: u8 = 3; //~ clippy::min_ident_chars
    let y: u8 = 4; //~ clippy::min_ident_chars
    let z: u8 = 5; //~ clippy::min_ident_chars
    assert_eq!((i, j, x, y, z), (1, 2, 3, 4, 5), "tuple");
}
