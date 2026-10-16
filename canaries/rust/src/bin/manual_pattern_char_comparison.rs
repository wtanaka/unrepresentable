//! Canary.
/// Canary item.
fn find(text: &str) -> Option<usize> {
    text.find(|chr| chr == 'a' || chr == 'b') //~ clippy::manual_pattern_char_comparison
}

fn main() {
    assert_eq!(find("xa"), Some(1), "find");
}
