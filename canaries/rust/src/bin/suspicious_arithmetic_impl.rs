//! Canary.
use core::ops::Add;

#[rustfmt::skip]
#[derive(Clone, Copy)]
/// Canary item.
struct Num(u8);

#[rustfmt::skip]
impl Add for Num {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0) //~ clippy::suspicious_arithmetic_impl clippy::arithmetic_side_effects
    }
}

#[rustfmt::skip]
fn main() {
    let sum = Num(2) + Num(1); //~ clippy::arithmetic_side_effects
    assert_eq!(sum.0, 1, "one");
}
