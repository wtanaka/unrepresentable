//! Canary.
use std::process::exit;

fn main() {
    let vec: Vec<u8> = Vec::new();
    let mut iter = vec.into_iter();
    exit(i32::from(iter.by_ref().peekable().peek().is_none())); //~ clippy::by_ref_peekable_peek
}
