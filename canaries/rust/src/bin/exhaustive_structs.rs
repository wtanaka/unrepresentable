//! Canary.
#[rustfmt::skip]
#[derive(Clone, Copy, Debug)]
/// Canary item.
pub struct Point { //~ clippy::exhaustive_structs
    /// Canary item.
    pub x_pos: u8,
}

fn main() {
    let point = Point { x_pos: 1 };
    assert!(point.x_pos == 1, "one");
}
