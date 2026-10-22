//! Canary.
fn main() {
    let a: u8 = 1; //~ clippy::min_ident_chars clippy::many_single_char_names
    assert!(a > 0, "a");
}
