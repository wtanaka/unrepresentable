//! Canary.
#[derive(Debug)]
/// Canary item.
pub struct Bag {
    /// Canary item.
    items: Vec<u8>,
}

impl Bag {
    #[inline]
    #[must_use]
    #[rustfmt::skip]
    /// Canary item.
    pub fn iter(&self) -> Vec<u8> { //~ clippy::iter_not_returning_iterator
        self.items.clone()
    }
}

fn main() {
    let bag = Bag { items: Vec::new() };
    assert!(bag.iter().is_empty(), "empty");
}
