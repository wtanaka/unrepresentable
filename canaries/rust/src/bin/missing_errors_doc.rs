//! Canary.

/// Fails always.
pub fn fails() -> Result<(), u8> { //~ clippy::missing_errors_doc clippy::missing_const_for_fn clippy::missing_inline_in_public_items
    Err(1)
}

/// Entry point.
fn main() {
    assert!(fails().is_err(), "error");
}
