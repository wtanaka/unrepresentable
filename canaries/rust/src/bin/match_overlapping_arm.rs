//! Canary.
#[rustfmt::skip]
/// Canary item.
fn pick(value: u8) -> u8 { //~ clippy::missing_const_for_fn
    match value {
        0..=10 => 1, //~ clippy::match_overlapping_arm
        5..=15 => 2,
        _ => 3,
    }
}

fn main() {
    assert_eq!(pick(1), 1, "pick");
}
