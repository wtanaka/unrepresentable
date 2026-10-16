//! Canary.
#[rustfmt::skip]
/// Canary item.
const unsafe fn danger() {} //~ unsafe_code
#[rustfmt::skip]
fn main() {
    // SAFETY: nothing to uphold.
    unsafe { //~ unsafe_code clippy::multiple_unsafe_ops_per_block
        danger();
        danger();
    }
}
