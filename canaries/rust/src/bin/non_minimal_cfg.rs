//! Canary.
#[cfg(all(unix))] //~ clippy::non_minimal_cfg
fn main() {}
#[cfg(not(unix))]
fn main() {}
