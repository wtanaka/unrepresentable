//! Canary.
fn main() {
    // SAFETY: nothing unsafe here.
    let _value: u8 = unsafe { 1 }; //~ unused_unsafe unsafe_code
}
