//! Canary.
use core::iter;

/// Canary item.
trait Ext: Iterator {
    /// Canary item.
    fn intersperse(self) -> u8
    where
        Self: Sized,
    {
        0
    }
}

impl<I: Iterator> Ext for I {}

fn main() {
    let _: u8 = iter::empty::<u8>().intersperse(); //~ unstable_name_collisions
}
