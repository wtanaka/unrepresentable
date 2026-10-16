//! Canary.
use core::str::from_utf8;

#[rustfmt::skip]
fn main() {
    let text = "hello";
    let parsed = from_utf8(&text.as_bytes()[1..3]); //~ clippy::string_from_utf8_as_bytes clippy::indexing_slicing
    assert!(parsed.is_ok(), "utf8");
}
