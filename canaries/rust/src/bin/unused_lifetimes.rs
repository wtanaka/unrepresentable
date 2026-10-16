//! Canary.
/// Canary item.
const fn function<'life>() {} //~ unused_lifetimes clippy::extra_unused_lifetimes

fn main() {
    function();
}
