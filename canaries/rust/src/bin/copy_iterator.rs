//! Canary.
use std::process::exit;

#[derive(Clone, Copy)]
/// Canary item.
struct Counter;

#[rustfmt::skip]
impl Iterator for Counter { //~ clippy::copy_iterator
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        None
    }
}

#[rustfmt::skip]
fn main() {
    exit(i32::from(Counter.next().is_none()));
}
