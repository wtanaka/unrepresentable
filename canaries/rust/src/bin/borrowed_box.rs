//! Canary.
use std::process::exit;

#[rustfmt::skip]
/// Canary item.
fn check(bx: &Box<u8>) -> bool { //~ clippy::borrowed_box
    **bx == 1
}

#[rustfmt::skip]
fn main() {
    exit(i32::from(check(&Box::new(1))));
}
