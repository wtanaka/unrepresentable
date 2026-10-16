//! Canary.
#[inline]
#[inline] //~ unused_attributes clippy::duplicated_attributes
/// Canary item.
const fn function() {}

fn main() {
    function();
}
