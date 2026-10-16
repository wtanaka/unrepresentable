//! Canary.
fn main() {
    // SAFETY: 65 is a valid scalar value.
    let _c = unsafe { char::from_u32_unchecked(65) }; //~ unsafe_code
}
