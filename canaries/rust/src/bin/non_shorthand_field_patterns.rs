//! Canary.
use core::hint::black_box;

/// Canary item.
struct Point {
    /// Canary item.
    val: u8,
}

fn main() {
    let point = Point { val: 1 };
    let Point { val: val } = point; //~ non_shorthand_field_patterns
    let _: u8 = black_box(val);
}
