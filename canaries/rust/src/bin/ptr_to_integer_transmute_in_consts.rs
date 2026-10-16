//! Canary.
use core::hint::black_box;
use core::mem;
use core::ptr;

/// Canary item.
const fn address(pointer: *const u8) -> usize {
    // SAFETY: none; rejected in const evaluation.
    unsafe { mem::transmute::<*const u8, usize>(pointer) } //~ ptr_to_integer_transmute_in_consts unsafe_code
}

fn main() {
    let _: usize = black_box(address(ptr::null()));
}
