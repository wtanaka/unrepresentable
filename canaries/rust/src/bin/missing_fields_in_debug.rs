//! Canary.
use core::fmt;

/// Canary item.
struct Pair {
    /// Canary item.
    left: u8,
    /// Canary item.
    right: u8,
}

#[rustfmt::skip]
impl fmt::Debug for Pair { //~ clippy::missing_fields_in_debug
    fn fmt(&self, fmtr: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmtr.debug_struct("Pair").field("left", &self.left).finish()
    }
}

fn main() {
    let pair = Pair { left: 1, right: 2 };
    assert_eq!(pair.right, 2, "right");
    assert!(format!("{pair:?}").len() > 1, "debug");
}
