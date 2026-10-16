//! Canary.
use core::num::NonZeroU32;
// SAFETY: one is nonzero
/// Canary item.
const VALUE: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(1) }; //~ clippy::useless_nonzero_new_unchecked unsafe_code
fn main() {
    let _v = VALUE.get();
}
