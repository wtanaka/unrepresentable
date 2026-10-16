//! Canary.
use std::process::exit;

fn main() {
    let val: u8 = 3;
    let bytes = val.to_be_bytes(); //~ clippy::big_endian_bytes
    exit(i32::from(bytes.is_empty()));
}
