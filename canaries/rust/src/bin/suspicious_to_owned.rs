//! Canary.
extern crate alloc;
use alloc::borrow::Cow;

#[rustfmt::skip]
fn main() {
    let value: Cow<'_, str> = Cow::Borrowed("a");
    let owned = value.to_owned(); //~ clippy::suspicious_to_owned
    assert!(!owned.is_empty(), "owned");
}
