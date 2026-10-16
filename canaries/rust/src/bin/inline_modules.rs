//! Canary.
#[rustfmt::skip]
/// Canary item.
mod inner { //~ clippy::inline_modules
    /// Canary item.
    pub(crate) const VALUE: u8 = 1;
}

fn main() {
    assert_eq!(inner::VALUE, 1, "one");
}
