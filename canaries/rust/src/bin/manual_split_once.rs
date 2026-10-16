//! Canary.
/// Canary item.
fn value(text: &str) -> Option<&str> {
    text.splitn(2, '=').nth(1) //~ clippy::manual_split_once
}

fn main() {
    assert_eq!(value("a=b"), Some("b"), "value");
}
