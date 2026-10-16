//! Canary.
fn main() {
    let text = String::new();
    let other = text.to_owned(); //~ clippy::implicit_clone
    assert_eq!(text, other, "same");
}
