//! Canary.
fn main() {
    let data: Vec<u8> = Vec::new();
    assert!(data.len() == 0, "empty"); //~ clippy::len_zero
}
