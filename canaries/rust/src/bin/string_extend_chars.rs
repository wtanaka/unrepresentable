//! Canary.
#[rustfmt::skip]
fn main() {
    let mut text = String::new();
    text.extend("abc".chars()); //~ clippy::string_extend_chars
    assert!(!text.is_empty(), "text");
}
