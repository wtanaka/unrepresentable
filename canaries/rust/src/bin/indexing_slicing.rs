//! Canary.
fn main() {
    let data: Vec<u8> = Vec::new();
    let first = data[0]; //~ clippy::indexing_slicing
    assert_eq!(first, 0, "zero");
}
