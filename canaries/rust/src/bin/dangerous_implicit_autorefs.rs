//! Canary.
#[rustfmt::skip]
fn main() {
    let arr: [u8; 1] = [1];
    let ptr: *const [u8] = &arr;
    // SAFETY: canary only.
    let len: usize = unsafe { (*ptr).len() }; //~ unsafe_code dangerous_implicit_autorefs
    assert_eq!(len, 1, "len");
}
