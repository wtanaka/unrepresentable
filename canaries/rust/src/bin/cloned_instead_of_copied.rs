//! Canary.
use std::process::exit;

fn main() {
    let vec: Vec<u8> = Vec::new();
    exit(i32::from(vec.iter().cloned().next().is_none())); //~ clippy::cloned_instead_of_copied
}
