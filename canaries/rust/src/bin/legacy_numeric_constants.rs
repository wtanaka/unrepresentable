//! Canary.
fn main() {
    let max = u32::max_value(); //~ clippy::legacy_numeric_constants deprecated
    assert!(max > 1, "big");
}
