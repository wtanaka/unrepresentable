//! Canary.
fn main() {
    let chr = '7';
    assert!(chr.is_digit(10), "digit"); //~ clippy::is_digit_ascii_radix
}
