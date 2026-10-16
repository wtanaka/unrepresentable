//! Canary.
use core::ops::AddAssign;

#[rustfmt::skip]
/// Canary item.
struct Num(u8);

#[rustfmt::skip]
impl AddAssign for Num {
    fn add_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0; //~ clippy::suspicious_op_assign_impl clippy::arithmetic_side_effects
    }
}

#[rustfmt::skip]
fn main() {
    let mut num = Num(2);
    num += Num(1); //~ clippy::arithmetic_side_effects
    assert_eq!(num.0, 1, "one");
}
