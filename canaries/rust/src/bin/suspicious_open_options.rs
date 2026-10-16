//! Canary.
use std::fs::OpenOptions;

#[rustfmt::skip]
fn main() {
    let file = OpenOptions::new().write(true).create(true).open("out.txt"); //~ clippy::suspicious_open_options
    assert!(file.is_ok(), "open");
}
