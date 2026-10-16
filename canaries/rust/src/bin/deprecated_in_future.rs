//! Canary.
#[deprecated(since = "TBD")]
/// Canary item.
const fn old() {}

#[rustfmt::skip]
fn main() {
    old(); //~ deprecated_in_future
}
