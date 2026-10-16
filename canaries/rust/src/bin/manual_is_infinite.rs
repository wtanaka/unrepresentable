//! Canary.
/// Canary item.
const fn infinite(value: f64) -> bool {
    value == f64::INFINITY || value == f64::NEG_INFINITY //~ clippy::manual_is_infinite
}

fn main() {
    assert!(!infinite(1.0), "infinite");
}
