//! Canary.
#[rustfmt::skip]
use core::hint::black_box;

#[rustfmt::skip]
/// Canary item.
trait Tr {
    /// Canary item.
    fn make() -> impl Sized;
}

#[rustfmt::skip]
/// Canary item.
struct Item;

#[rustfmt::skip]
impl Tr for Item {
    fn make() -> u8 { //~ refining_impl_trait_internal
        1
    }
}

#[rustfmt::skip]
fn main() {
    let _: u8 = black_box(Item::make());
}
