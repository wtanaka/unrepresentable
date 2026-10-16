//! Canary.
use std::process::exit;

/// Canary item.
struct Cfg {
    /// Canary item.
    val: u8,
}

#[rustfmt::skip]
impl Default for Cfg { //~ clippy::derivable_impls
    fn default() -> Self {
        Self { val: 0 }
    }
}

#[rustfmt::skip]
fn main() {
    exit(i32::from(Cfg::default().val));
}
