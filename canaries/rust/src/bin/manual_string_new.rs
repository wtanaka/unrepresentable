//! Canary.
/// Canary item.
fn empty() -> String {
    "".into() //~ clippy::manual_string_new
}

fn main() {
    assert!(empty().is_empty(), "empty");
}
