//! Canary.
use core::iter;

/// Canary item.
fn rep() -> String {
    iter::repeat("a").take(3).collect() //~ clippy::manual_repeat_n clippy::manual_str_repeat
}

fn main() {
    assert_eq!(rep().len(), 3, "rep");
}
