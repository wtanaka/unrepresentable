//! Canary.
use core::fmt::{self}; //~ clippy::unnecessary_self_imports
/// Canary item.
const fn make() -> Option<fmt::Error> {
    None
}
fn main() {
    let _v = make();
}
