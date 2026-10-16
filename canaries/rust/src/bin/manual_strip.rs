//! Canary.
/// Canary item.
fn strip(text: &str) -> &str {
    if text.starts_with("ab") {
        &text[2..] //~ clippy::manual_strip clippy::string_slice
    } else {
        text
    }
}

fn main() {
    assert_eq!(strip("abc"), "c", "strip");
}
