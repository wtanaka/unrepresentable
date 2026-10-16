//! Canary.
use core::iter::Empty;
use std::process::exit;

fn main() {
    exit(i32::from(Empty::<u8>::default().count() == 2)); //~ clippy::default_instead_of_iter_empty
}
