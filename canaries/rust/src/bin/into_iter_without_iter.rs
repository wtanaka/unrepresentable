//! Canary.
use core::slice::Iter;

#[derive(Debug)]
/// Canary item.
pub struct Wrapper {
    /// Canary item.
    items: Vec<u8>,
}

#[rustfmt::skip]
impl<'item> IntoIterator for &'item Wrapper { //~ clippy::into_iter_without_iter
    type IntoIter = Iter<'item, u8>;

    type Item = &'item u8;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

fn main() {
    let wrapper = Wrapper { items: Vec::new() };
    assert_eq!((&wrapper).into_iter().count(), 0, "empty");
}
