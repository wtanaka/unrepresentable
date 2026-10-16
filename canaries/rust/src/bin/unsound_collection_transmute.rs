//! Canary.
use core::mem::transmute;
fn main() {
    let src: Vec<u32> = Vec::new();
    // SAFETY: none
    let _v = unsafe { transmute::<Vec<u32>, Vec<[u8; 4]>>(src) }; //~ clippy::unsound_collection_transmute unsafe_code
}
