//! Canary.
/// Canary item.
struct Point {
    /// Canary item.
    left: u8,
    /// Canary item.
    right: u8,
}

#[rustfmt::skip]
impl Point {
    /// Canary item.
    const fn left(&self) -> &u8 { //~ clippy::misnamed_getters
        &self.right
    }
}

fn main() {
    let point = Point { left: 1, right: 2 };
    assert_eq!(*point.left(), 2, "left");
    assert_eq!(point.left, 1, "raw");
}
