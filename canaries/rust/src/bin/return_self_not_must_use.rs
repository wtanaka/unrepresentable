//! Canary.
#[rustfmt::skip]
#[derive(Debug)]
#[non_exhaustive]
/// Canary item.
pub struct Builder;

#[rustfmt::skip]
impl Builder {
    #[inline]
    /// Canary item.
    pub const fn chain(&self) -> Self { //~ clippy::return_self_not_must_use clippy::must_use_candidate clippy::unused_self
        Self
    }
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}

#[rustfmt::skip]
fn main() {
    sink(Builder.chain());
}
