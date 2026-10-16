//! Canary.
#![no_std]
extern crate std;
#[rustfmt::skip]
use std::*; //~ clippy::wildcard_imports

#[rustfmt::skip]
fn main() {
    panic!("boom"); //~ ambiguous_panic_imports clippy::panic
}
