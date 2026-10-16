//! Canary.
use core::fmt::*; //~ clippy::wildcard_imports
/// Canary item.
const fn make() -> Option<Error> {
    None
}
fn main() {
    let _v = make();
}
