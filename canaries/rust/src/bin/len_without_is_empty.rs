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
    pub const fn len(&self) -> usize { //~ clippy::len_without_is_empty
        self.items.len()
    }
}

fn main() {
    let bag = Bag { items: Vec::new() };
    assert_eq!(bag.len(), 0, "empty");
}
