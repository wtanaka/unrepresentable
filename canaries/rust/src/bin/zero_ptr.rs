//! Canary.
fn main() {
    let value: *const i32 = 0 as *const i32; //~ clippy::zero_ptr clippy::as_conversions
    let _v = value.addr();
}
