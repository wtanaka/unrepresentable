//! Canary.
#[rustfmt::skip]
/// Canary item.
mod foo { //~ clippy::inline_modules
    /// Canary item.
    mod foo {} //~ clippy::inline_modules clippy::module_inception
}
#[rustfmt::skip]
fn main() {}
