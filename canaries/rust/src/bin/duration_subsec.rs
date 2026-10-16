//! Canary.
use core::time::Duration;

#[rustfmt::skip]
fn main() {
    let span = Duration::new(1, 500);
    let millis = span.subsec_nanos() / 1_000_000; //~ clippy::duration_subsec clippy::integer_division clippy::integer_division_remainder_used
    assert!(millis == 0, "zero");
}
