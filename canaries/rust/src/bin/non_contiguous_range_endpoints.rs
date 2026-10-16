//! Canary.
fn main() {
    let value: u8 = 3;
    let _: u8 = match value {
        0..5 => 1, //~ non_contiguous_range_endpoints
        6..=9 => 2,
        _ => 3,
    };
}
