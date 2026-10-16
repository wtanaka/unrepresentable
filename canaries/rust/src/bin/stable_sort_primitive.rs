//! Canary.
#[rustfmt::skip]
fn main() {
    let mut data: [u8; 2] = [3, 1];
    data.sort(); //~ clippy::stable_sort_primitive
    assert!(data.is_empty(), "empty");
}
