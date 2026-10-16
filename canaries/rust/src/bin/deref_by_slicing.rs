//! Canary.
use std::process::exit;

fn main() {
    let vec: Vec<u8> = Vec::new();
    let sl: &[u8] = &vec[..]; //~ clippy::deref_by_slicing
    exit(i32::from(sl.is_empty()));
}
