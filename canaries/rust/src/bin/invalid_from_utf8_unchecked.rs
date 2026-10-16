//! Canary.
use core::str;

#[rustfmt::skip]
fn main() {
    // SAFETY: canary only.
    let text: &str = unsafe { str::from_utf8_unchecked(b"\xff") }; //~ unsafe_code invalid_from_utf8_unchecked
    assert!(text.is_empty(), "empty");
}
