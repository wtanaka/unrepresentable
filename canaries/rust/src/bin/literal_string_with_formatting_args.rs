//! Canary.
fn main() {
    let name = "x";
    let text = "{name} {} {:?}"; //~ clippy::literal_string_with_formatting_args
    assert_ne!(name, text, "differ");
}
