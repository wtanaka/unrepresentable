//! Canary.
use core::mem::transmute;

#[derive(Clone, Copy)]
#[repr(u8)]
/// Canary item.
enum Small {
    /// Canary item.
    Zero,
    /// Canary item.
    One,
}

#[rustfmt::skip]
fn main() {
    let both = [Small::Zero, Small::One];
    assert!(both.len() == 2, "two");
    let raw: u8 = 1;
    let converted = (raw < 2).then_some(unsafe { transmute::<u8, Small>(raw) }); //~ clippy::eager_transmute unsafe_code clippy::undocumented_unsafe_blocks
    assert!(converted.is_some(), "some");
}
