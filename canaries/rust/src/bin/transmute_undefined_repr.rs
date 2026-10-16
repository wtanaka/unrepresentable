//! Canary.
use core::mem::transmute;

#[rustfmt::skip]
fn main() {
    let data: Vec<u32> = Vec::new();
    // SAFETY: not sound in general, never relied upon.
    let parts = unsafe { transmute::<Vec<u32>, (usize, usize, usize)>(data) }; //~ unsafe_code clippy::transmute_undefined_repr
    assert!(parts.0 == parts.1, "same");
}
