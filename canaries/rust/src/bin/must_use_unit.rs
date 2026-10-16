//! Canary.
#[must_use]
/// Canary item.
const fn nothing() {} //~ clippy::must_use_unit
fn main() {
    nothing(); //~ unused_must_use
}
