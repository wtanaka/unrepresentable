//! Canary.
use core::ops::Range;

#[rustfmt::skip]
fn main() {
    let more: Vec<Range<u8>> = vec![0..5]; //~ clippy::single_range_in_vec_init
    assert!(more.is_empty(), "empty");
}
