//! Canary.
#[rustfmt::skip]
/// Canary item.
trait Private {}

#[rustfmt::skip]
/// Canary item.
struct Item;

#[rustfmt::skip]
impl Private for Item {}

#[rustfmt::skip]
#[inline]
/// Canary item.
pub const fn bounded<Kind>() //~ private_bounds
where
    Kind: Private,
{
}

#[rustfmt::skip]
fn main() {
    bounded::<Item>();
}
