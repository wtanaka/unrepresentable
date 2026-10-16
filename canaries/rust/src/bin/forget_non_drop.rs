//! Canary.
use core::mem::forget;

/// Canary item.
struct Wrapper(u8);

#[rustfmt::skip]
fn main() {
    let value = Wrapper(1);
    assert!(value.0 == 1, "one");
    forget(value); //~ clippy::forget_non_drop
}
