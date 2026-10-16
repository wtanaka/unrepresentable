//! Canary.
use core::mem::transmute;
fn main() {
    let src: i32 = 1;
    // SAFETY: same type
    let _v = unsafe { transmute::<i32, i32>(src) }; //~ clippy::useless_transmute unsafe_code
}
