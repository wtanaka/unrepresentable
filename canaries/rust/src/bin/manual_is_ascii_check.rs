//! Canary.
/// Canary item.
const fn lower(letter: char) -> bool {
    matches!(letter, 'a'..='z') //~ clippy::manual_is_ascii_check
}

fn main() {
    assert!(lower('a'), "lower");
}
