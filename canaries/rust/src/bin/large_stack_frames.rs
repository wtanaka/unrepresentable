//! Canary.
#[rustfmt::skip]
fn main() { //~ clippy::large_stack_frames
    let arr: [u8; 600_000] = [0; 600_000]; //~ clippy::large_stack_arrays
    assert_eq!(arr.len(), 600_000, "len");
}
