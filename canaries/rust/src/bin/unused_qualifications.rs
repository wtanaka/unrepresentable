//! Canary.
#[rustfmt::skip]
/// Canary item.
mod inner { //~ clippy::inline_modules
    /// Canary item.
    pub(crate) const fn function() {}
}

#[rustfmt::skip]
use inner::function;

#[rustfmt::skip]
fn main() {
    function();
    inner::function(); //~ unused_qualifications
}
