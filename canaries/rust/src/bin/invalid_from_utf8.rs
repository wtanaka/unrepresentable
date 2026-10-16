//! Canary.
use core::str;

/// Canary item.
fn sink<Ty>(_val: Ty) {}

#[rustfmt::skip]
fn main() {
    sink(str::from_utf8(b"\xff")); //~ invalid_from_utf8
}
