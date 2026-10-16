//! Canary.
#[rustfmt::skip]
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
/// Canary item.
pub struct Counter {
    /// Canary item.
    pub step: u8,
}

#[rustfmt::skip]
impl Counter {
    #[inline]
    /// Canary item.
    pub fn next(&mut self) -> Option<u8> { //~ clippy::should_implement_trait
        self.step = self.step.checked_add(1)?;
        Some(self.step)
    }
}

#[rustfmt::skip]
fn main() {
    let mut counter = Counter { step: 0 };
    assert!(counter.next().is_some(), "counter advanced");
}
