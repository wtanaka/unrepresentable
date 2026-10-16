//! Canary.
/// Canary item.
static mut COUNTER: u8 = 0;

fn main() {
    // SAFETY: single-threaded, never read.
    let _reference = unsafe { &COUNTER }; //~ static_mut_refs unsafe_code
}
