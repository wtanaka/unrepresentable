//! Canary.
fn main() {
    let data: Vec<u8> = Vec::new();
    let first = data.iter().next(); //~ clippy::iter_next_slice
    assert!(first.is_none(), "empty");
}
