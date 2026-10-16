//! Canary.
use std::process::exit;

fn main() {
    let bytes = &[b'a', b'b', b'c']; //~ clippy::byte_char_slices
    exit(i32::from(bytes.is_empty()));
}
