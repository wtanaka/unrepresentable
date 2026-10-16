//! Canary.
#[rustfmt::skip]
fn main() {
    let value: u8 = 1;
    let text = format!("x{}", value.to_string()); //~ clippy::to_string_in_format_args
    assert!(!text.is_empty(), "text");
}
