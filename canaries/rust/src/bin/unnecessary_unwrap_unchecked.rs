//! Canary.
use core::str;
fn main() {
    let bytes: &[u8] = b"a";
    // SAFETY: none
    let _v = unsafe { str::from_utf8(bytes).unwrap_unchecked() }; //~ clippy::unnecessary_unwrap_unchecked unsafe_code
}
