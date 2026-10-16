//! Canary.
use core::slice::Iter;

#[derive(Debug)]
/// Canary item.
pub struct Bag {
    /// Canary item.
    items: Vec<u8>,
}

impl Bag {
    #[inline]
    #[rustfmt::skip]
    /// Canary item.
    pub fn iter(&self) -> Iter<'_, u8> { //~ clippy::iter_without_into_iter
        self.items.iter()
    }
}

fn main() {
    let bag = Bag { items: Vec::new() };
    assert_eq!(bag.iter().count(), 0, "empty");
}
