//! Canary.
use core::mem;

#[rustfmt::skip]
fn main() {
    let addr: usize = 8;
    // SAFETY: canary only.
    let ptr: *const u8 = unsafe { mem::transmute::<usize, *const u8>(addr) }; //~ unsafe_code integer_to_ptr_transmutes
    assert!(!ptr.is_null(), "non-null");
}
