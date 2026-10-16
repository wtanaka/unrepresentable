//! Canary.
#[rustfmt::skip]
fn main() {
    let value: u8 = 3;
    let _: u8 = match value {
        0..=10 => 1, //~ clippy::match_overlapping_arm
        10..=20 => 2, //~ overlapping_range_endpoints
        _ => 3,
    };
}
