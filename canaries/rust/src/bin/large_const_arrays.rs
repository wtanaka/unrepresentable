//! Canary.
/// Canary item.
const BIG: [u8; 1_000_000] = [0; 1_000_000]; //~ clippy::large_const_arrays

#[rustfmt::skip]
fn main() { //~ clippy::large_stack_frames
    assert_eq!(BIG.len(), 1_000_000, "len");
}
