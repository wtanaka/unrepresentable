//! Canary.
use core::mem::transmute;

#[rustfmt::skip]
fn main() {
    let bytes: &[u8] = &[104, 105];
    // SAFETY: the bytes are valid UTF-8.
    let text = unsafe { transmute::<&[u8], &str>(bytes) }; //~ unsafe_code clippy::transmute_bytes_to_str
    assert!(!text.is_empty(), "text");
}
