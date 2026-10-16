//! Canary.
use core::mem::transmute;
use core::ptr::null;

#[rustfmt::skip]
fn main() {
    // SAFETY: never called.
    let function = unsafe { transmute::<*const (), fn()>(null()) }; //~ unsafe_code clippy::transmute_null_to_fn
    function();
}
