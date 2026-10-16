//! Canary.
use core::mem;

fn main() {
    let value: u8 = 1;
    // SAFETY: deliberately unsound, never written through.
    let _alias: &mut u8 = unsafe { mem::transmute::<&u8, &mut u8>(&value) }; //~ mutable_transmutes unsafe_code clippy::transmute_ptr_to_ptr
}
