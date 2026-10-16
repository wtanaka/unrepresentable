//! Canary.
#[rustfmt::skip]
fn main() {
    let one: u8 = 1;
    let ptr: *const u8 = &raw const one;
    let value = unsafe { *ptr }; //~ unsafe_code clippy::undocumented_unsafe_blocks
    assert!(value > 0, "positive");
}
