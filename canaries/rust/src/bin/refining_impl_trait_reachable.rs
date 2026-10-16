//! Canary.
#[rustfmt::skip]
use core::hint::black_box;

#[rustfmt::skip]
/// Canary item.
pub trait Tr {
    /// Canary item.
    fn make() -> impl Sized;
}

#[rustfmt::skip]
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
/// Canary item.
pub struct Item;

#[rustfmt::skip]
impl Tr for Item {
    #[inline]
    fn make() -> u8 { //~ refining_impl_trait_reachable
        1
    }
}

#[rustfmt::skip]
fn main() {
    let _: u8 = black_box(Item::make());
}
