//! Canary.
use std::process::exit;

#[rustfmt::skip]
/// Canary item.
fn check(bx: Box<u8>) -> bool { //~ clippy::boxed_local clippy::needless_pass_by_value
    *bx == 1
}

#[rustfmt::skip]
/// Canary item: the boxed value is above 64 bytes and below 1024.
fn check_large(bx: Box<[u8; 512]>) -> bool { //~ clippy::boxed_local clippy::needless_pass_by_value
    bx.len() == 1
}

#[rustfmt::skip]
fn main() {
    exit(i32::from(check(Box::new(1)) && check_large(Box::new([1; 512]))));
}
