//! Canary.
use std::process::exit;

fn main() {
    let arr: [u8; 8] = [0; 8];
    exit(i32::from(arr.chunks_exact(4).next().is_none())); //~ clippy::chunks_exact_to_as_chunks
}
