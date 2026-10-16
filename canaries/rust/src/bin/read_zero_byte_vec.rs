//! Canary.
use std::io::{stdin, Read as _};

#[rustfmt::skip]
fn main() {
    let mut data: Vec<u8> = Vec::new();
    drop(stdin().read(&mut data)); //~ clippy::read_zero_byte_vec
}
