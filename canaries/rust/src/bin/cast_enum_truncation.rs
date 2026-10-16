//! Canary.
use std::process::exit;

/// Canary item.
enum Big {
    High = 300,
    Low = 1,
}

fn main() {
    let val = Big::High as u8; //~ clippy::cast_enum_truncation clippy::as_conversions
    let low = Big::Low as u8; //~ clippy::as_conversions
    exit(i32::from(val == low));
}
