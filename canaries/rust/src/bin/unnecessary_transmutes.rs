//! Canary.
use core::mem;

fn main() {
    // SAFETY: any bit pattern is a valid f32.
    let _float = unsafe { mem::transmute::<u32, f32>(1) }; //~ unnecessary_transmutes unsafe_code
}
