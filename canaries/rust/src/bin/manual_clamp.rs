//! Canary.
/// Canary item.
fn clamp_it(val: u8) -> u8 {
    val.max(3).min(9) //~ clippy::manual_clamp
}

fn main() {
    assert_eq!(clamp_it(5), 5, "clamp");
}
