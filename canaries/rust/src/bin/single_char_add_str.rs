//! Canary.
#[rustfmt::skip]
fn main() {
    let mut text = String::new();
    text.push_str("x"); //~ clippy::single_char_add_str
    assert!(text.is_empty(), "empty");
}
