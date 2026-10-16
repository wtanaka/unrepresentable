//! Canary.
use core::time::Duration;
use std::time::Instant;

/// Canary item.
fn since(earlier: Instant) -> Duration {
    Instant::now() - earlier //~ clippy::arithmetic_side_effects clippy::manual_instant_elapsed
}

fn main() {
    assert!(since(Instant::now()).as_secs() < 5, "since");
}
