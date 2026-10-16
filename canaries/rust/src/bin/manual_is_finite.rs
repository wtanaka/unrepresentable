//! Canary.
/// Canary item.
const fn finite(value: f64) -> bool {
    value != f64::INFINITY && value != f64::NEG_INFINITY //~ clippy::manual_is_finite
}

fn main() {
    assert!(finite(1.0), "finite");
}
