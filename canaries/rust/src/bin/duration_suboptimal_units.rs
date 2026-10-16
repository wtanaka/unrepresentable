//! Canary.
use core::time::Duration;

#[rustfmt::skip]
fn main() {
    let span = Duration::from_millis(30_000); //~ clippy::duration_suboptimal_units
    assert!(span.as_secs() == 30, "thirty");
}
