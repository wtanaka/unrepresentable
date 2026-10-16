//! Canary.
use core::cmp::Ordering;

#[rustfmt::skip]
#[derive(PartialEq, Eq)]
/// Canary item.
struct Thing(u8);

#[rustfmt::skip]
impl Ord for Thing {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

#[rustfmt::skip]
impl PartialOrd for Thing { //~ clippy::non_canonical_partial_ord_impl
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.0.cmp(&other.0))
    }
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

#[rustfmt::skip]
fn main() {
    sink(Thing(1) < Thing(2));
}
