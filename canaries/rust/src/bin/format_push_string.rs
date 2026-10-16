//! Canary.
#[rustfmt::skip]
fn main() {
    let mut text = String::new();
    let value: u8 = 1;
    text.push_str(&format!("{value}")); //~ clippy::format_push_string
    assert!(text.len() == 1, "one");
}
