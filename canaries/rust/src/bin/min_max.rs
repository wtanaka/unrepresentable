//! Canary.
/// Canary item.
fn clamp(value: u8) -> u8 {
    value.max(9).min(3) //~ clippy::min_max
}

fn main() {
    assert_eq!(clamp(1), 3, "clamp");
}
