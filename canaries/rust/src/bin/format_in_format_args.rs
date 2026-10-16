//! Canary.
#[rustfmt::skip]
fn main() {
    let value: u8 = 1;
    let text = format!("{}", format!("{value}")); //~ clippy::format_in_format_args clippy::useless_format
    assert!(text.len() == 1, "one");
}
