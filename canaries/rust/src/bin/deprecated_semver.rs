//! Canary.
#[deprecated(since = "1")] //~ clippy::deprecated_semver
/// Canary item.
const fn old() {}

fn main() {
    old(); //~ deprecated
}
