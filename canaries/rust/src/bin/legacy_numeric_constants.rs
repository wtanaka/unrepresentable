//! Canary.
fn main() {
    let max = u32::max_value(); //~ clippy::legacy_numeric_constants deprecated_in_future
    assert!(max > 1, "big");
}
