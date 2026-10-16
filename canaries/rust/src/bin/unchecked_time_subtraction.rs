//! Canary.
use core::time::Duration;

#[rustfmt::skip]
fn main() {
    let first = Duration::from_secs(2);
    let second = Duration::from_secs(1);
    let diff = first - second; //~ clippy::unchecked_time_subtraction clippy::arithmetic_side_effects
    assert!(diff > second, "diff");
}
