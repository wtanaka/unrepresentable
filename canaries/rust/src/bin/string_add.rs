//! Canary.
#[rustfmt::skip]
fn main() {
    let text = String::new() + "a"; //~ clippy::string_add
    assert!(!text.is_empty(), "text");
}
