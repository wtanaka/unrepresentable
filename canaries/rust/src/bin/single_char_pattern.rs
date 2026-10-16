//! Canary.
#[rustfmt::skip]
fn main() {
    assert!("a,b".contains(","), "comma"); //~ clippy::single_char_pattern
}
