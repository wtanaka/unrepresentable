//! Canary.
#[rustfmt::skip]
fn main() {
    let mut data: Vec<u8> = Vec::with_capacity(4); //~ clippy::uninit_vec
                                                   // SAFETY: not sound, never relied upon.
    unsafe { data.set_len(4) } //~ unsafe_code
    assert!(data.len() == 4, "len");
}
