//! Canary.
use core::sync::atomic::{AtomicBool, Ordering};

fn main() {
    let flag = AtomicBool::new(true);
    while flag.load(Ordering::Acquire) {} //~ clippy::missing_spin_loop
}
