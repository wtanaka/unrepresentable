//! Canary.
fn main() {
    let val: u32 = 1;
    let bytes = val.to_le_bytes(); //~ clippy::little_endian_bytes
    assert_eq!(bytes.len(), 4, "len");
}
