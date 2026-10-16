//! Canary.
fn main() { //~ clippy::large_stack_frames
    let arr: [u8; 20_000] = [0; 20_000]; //~ clippy::large_stack_arrays
    assert_eq!(arr.len(), 20_000, "len");
}
