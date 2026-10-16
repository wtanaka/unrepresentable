//! Canary.
fn main() {
    let a: u8 = 1; //~ clippy::min_ident_chars
    assert!(a > 0, "a");
}
