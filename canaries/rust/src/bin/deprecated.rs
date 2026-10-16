//! Canary.
#[deprecated]
/// Canary item.
const fn old() {}

#[rustfmt::skip]
fn main() {
    old(); //~ deprecated
}
