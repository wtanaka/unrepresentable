//! Canary.
#[rustfmt::skip]
fn main() {
    let target = 'a';
    let found = "abc".chars().any(|each| each == target); //~ clippy::string_lit_chars_any
    assert!(found, "found");
}
