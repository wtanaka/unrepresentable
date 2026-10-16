//! Canary.
#[rustfmt::skip]
fn main() {
    let value: u8 = 1;
    let text = format!("{}", value); //~ clippy::uninlined_format_args
    assert!(!text.is_empty(), "text");
}
