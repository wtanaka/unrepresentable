//! Canary.
#[rustfmt::skip]
fn main() {
    let mut val: u8 = 3;
    if val != u8::MAX { //~ clippy::implicit_saturating_add
        val += 1; //~ clippy::arithmetic_side_effects
    }
    assert_eq!(val, 4, "inc");
}
