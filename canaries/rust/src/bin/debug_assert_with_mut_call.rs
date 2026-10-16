//! Canary.
fn main() {
    let mut vec: Vec<u8> = Vec::new();
    debug_assert!(vec.pop().is_none(), "canary"); //~ clippy::debug_assert_with_mut_call
}
