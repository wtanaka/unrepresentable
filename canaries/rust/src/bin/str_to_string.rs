//! Canary.
#[rustfmt::skip]
fn main() {
    let text = "a".to_string(); //~ clippy::str_to_string
    assert!(!text.is_empty(), "text");
}
