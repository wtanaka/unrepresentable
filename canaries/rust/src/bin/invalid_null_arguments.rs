//! Canary.
use core::{ptr, slice};

#[rustfmt::skip]
fn main() {
    // SAFETY: canary only.
    let sli: &[u8] = unsafe { slice::from_raw_parts::<u8>(ptr::null(), 1) }; //~ unsafe_code invalid_null_arguments
    assert!(sli.is_empty(), "empty");
}
