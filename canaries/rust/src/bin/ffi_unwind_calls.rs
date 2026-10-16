//! Canary.
#[rustfmt::skip]
unsafe extern "C-unwind" { //~ unsafe_code
    fn go();
}

#[rustfmt::skip]
fn main() {
    // SAFETY: canary only.
    unsafe { go() } //~ unsafe_code ffi_unwind_calls
}
