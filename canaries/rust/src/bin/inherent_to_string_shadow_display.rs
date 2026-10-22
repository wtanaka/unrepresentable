//! Canary.
use core::fmt::{Display, Formatter, Result};

/// Canary item.
struct Thing(u8);

impl Display for Thing {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        formatter.write_str("thing")
    }
}

#[rustfmt::skip]
impl Thing {
    /// Canary item.
    fn to_string(&self) -> String { //~ clippy::inherent_to_string_shadow_display
        self.0.to_string()
    }
}

fn main() {
    let thing = Thing(1);
    assert!(thing.to_string().is_empty(), "empty");
}
