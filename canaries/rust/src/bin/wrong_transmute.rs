//! Canary.
use core::mem::transmute;
fn main() {
    let src: f64 = 1.0;
    // SAFETY: none
    let _v = unsafe { transmute::<f64, *const u8>(src) }; //~ clippy::wrong_transmute unsafe_code
}
