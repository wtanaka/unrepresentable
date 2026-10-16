//! Canary.
use core::ops::{Deref, DerefMut};

#[rustfmt::skip]
/// Canary item.
fn make(val: &mut u8) -> impl DerefMut<Target = u8> + Deref<Target = u8> + '_ { //~ clippy::implied_bounds_in_impls
    val
}

fn main() {
    let mut num: u8 = 1;
    let guard = make(&mut num);
    assert_eq!(*guard, 1, "same");
}
