//! Canary.
use core::hint::black_box;
use core::ptr::NonNull;

fn main() {
    let value: u8 = black_box(1);
    let boxed = Box::new(value);
    // SAFETY: Box::into_raw never returns null.
    let ptr = unsafe { NonNull::new_unchecked(Box::into_raw(boxed)) }; //~ unsafe_code clippy::nonnull_unchecked_on_box_ptr
    let _: NonNull<u8> = black_box(ptr);
}
