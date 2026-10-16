//! Canary.
#[rustfmt::skip]
use Direction::*; //~ clippy::enum_glob_use

/// Canary item.
enum Direction {
    /// Canary item.
    Down,
    /// Canary item.
    Up,
}

fn main() {
    assert!(matches!(Up, Up), "up");
    assert!(!matches!(Down, Up), "down");
}
