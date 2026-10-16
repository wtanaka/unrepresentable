//! Canary.
use core::num::TryFromIntError;

#[rustfmt::skip]
#[must_use]
/// Canary item.
fn parse(input: u32) -> Result<u8, TryFromIntError> { //~ clippy::double_must_use
    u8::try_from(input)
}

fn main() {
    assert!(parse(1).is_ok(), "ok");
}
