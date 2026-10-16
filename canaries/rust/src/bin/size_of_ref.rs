//! Canary.
#[rustfmt::skip]
fn main() {
    let value: u8 = 1;
    let reference = &value;
    assert_eq!(size_of_val(&reference), 8, "size"); //~ clippy::size_of_ref
}
