//! Canary.
/// Canary item.
fn same(left: &str, right: &str) -> bool {
    left.to_ascii_lowercase() == right.to_ascii_lowercase() //~ clippy::manual_ignore_case_cmp
}

fn main() {
    assert!(same("A", "a"), "same");
}
