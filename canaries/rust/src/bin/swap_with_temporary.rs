//! Canary.
use core::mem::swap;

#[rustfmt::skip]
/// Canary item.
const fn make() -> Vec<u8> {
    Vec::new()
}

#[rustfmt::skip]
fn main() {
    let mut data: Vec<u8> = Vec::new();
    swap(&mut data, &mut make()); //~ clippy::swap_with_temporary
    assert!(data.is_empty(), "empty");
}
