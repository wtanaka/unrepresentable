//! Canary.
fn main() {
    let value: u32 = 8;
    assert!((value & 7) == 0, "mask"); //~ clippy::verbose_bit_mask
}
