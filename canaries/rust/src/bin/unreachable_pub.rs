//! Canary.
#[rustfmt::skip]
/// Canary item.
mod inner { //~ clippy::inline_modules
    /// Canary item.
    pub const fn function() {} //~ unreachable_pub
}

#[rustfmt::skip]
fn main() {
    inner::function();
}
