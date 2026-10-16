//! Canary.
#[rustfmt::skip]
fn main() {
    let mut val: u8 = 3;
    if val != 0 { //~ clippy::implicit_saturating_sub
        val -= 1; //~ clippy::arithmetic_side_effects
    }
    assert_eq!(val, 2, "dec");
}
