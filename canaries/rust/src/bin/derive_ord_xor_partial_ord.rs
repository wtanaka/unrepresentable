//! Canary.
#[rustfmt::skip]
use core::cmp::Ordering;

#[derive(PartialEq, Eq, Ord)] //~ clippy::derive_ord_xor_partial_ord
/// Canary item.
struct Wrapper(u8);

#[rustfmt::skip]
impl PartialOrd for Wrapper { //~ clippy::non_canonical_partial_ord_impl
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.0.cmp(&other.0))
    }
}

fn main() {
    let left = Wrapper(1);
    let right = Wrapper(2);
    assert!(left < right, "ordered");
}
