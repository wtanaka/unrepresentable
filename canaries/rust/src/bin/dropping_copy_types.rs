//! Canary.
#[rustfmt::skip]
fn main() {
    let val: u8 = 1;
    drop(val); //~ dropping_copy_types
}
