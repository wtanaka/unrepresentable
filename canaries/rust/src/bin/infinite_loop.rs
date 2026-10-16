//! Canary.
use core::hint::spin_loop;
use std::env::args;

#[rustfmt::skip]
/// Canary item.
fn spin() {
    loop { //~ clippy::infinite_loop
        spin_loop();
    }
}

fn main() {
    if args().count() > 100 {
        spin();
    }
}
