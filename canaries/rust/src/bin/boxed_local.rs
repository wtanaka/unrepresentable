//! Canary.
use std::process::exit;

#[rustfmt::skip]
/// Canary item.
fn check(bx: Box<u8>) -> bool { //~ clippy::boxed_local clippy::needless_pass_by_value
    *bx == 1
}

#[rustfmt::skip]
fn main() {
    exit(i32::from(check(Box::new(1))));
}
