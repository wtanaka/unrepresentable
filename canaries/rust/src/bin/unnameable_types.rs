//! Canary.
#[rustfmt::skip]
/// Canary item.
mod inner { //~ clippy::inline_modules
    #[derive(Clone, Copy, Debug)]
    #[non_exhaustive]
    /// Canary item.
    pub struct Hidden; //~ unnameable_types

    #[inline]
    #[must_use]
    /// Canary item.
    pub const fn make() -> Hidden {
        Hidden
    }
}

#[rustfmt::skip]
pub use inner::make; //~ clippy::pub_use

#[rustfmt::skip]
fn main() {
    let _hidden = make();
}
