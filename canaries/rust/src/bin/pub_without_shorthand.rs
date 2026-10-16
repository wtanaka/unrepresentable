//! Canary.
#[rustfmt::skip]
/// Canary item.
mod inner { //~ clippy::inline_modules
    /// Canary item.
    pub(in super) const fn go() {} //~ clippy::pub_without_shorthand
}

fn main() {
    inner::go();
}
