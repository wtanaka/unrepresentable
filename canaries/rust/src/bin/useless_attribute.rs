//! Canary.
#[allow(clippy::needless_pass_by_value, reason = "canary")] //~ clippy::useless_attribute clippy::allow_attributes
use core::fmt;
/// Canary item.
const fn make() -> Option<fmt::Error> {
    None
}
fn main() {
    let _v = make();
}
