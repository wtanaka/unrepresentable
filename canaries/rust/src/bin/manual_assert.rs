//! Canary.
use std::env::args;

#[rustfmt::skip]
fn main() {
    let count = args().count();
    if count > 100 { panic!("big"); } //~ clippy::manual_assert clippy::panic
}
