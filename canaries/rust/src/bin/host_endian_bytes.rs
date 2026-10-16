//! Canary.
#[rustfmt::skip]
fn main() {
    let value: u32 = 1;
    let bytes = value.to_ne_bytes(); //~ clippy::host_endian_bytes
    assert!(bytes.len() == 4, "four");
}
