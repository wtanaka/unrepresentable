//! Canary.
fn main() {
    let text = "a​b"; //~ clippy::invisible_characters clippy::non_ascii_literal
    assert_eq!(text.len(), 5, "len");
}
