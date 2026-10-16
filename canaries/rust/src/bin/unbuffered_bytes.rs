//! Canary.
use std::fs::File;
use std::io::Read as _;

#[rustfmt::skip]
fn main() {
    let count = File::open("a").map(|file| file.bytes().count()); //~ clippy::unbuffered_bytes
    assert!(count.is_err(), "missing");
}
