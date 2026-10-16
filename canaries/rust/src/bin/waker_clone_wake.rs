//! Canary.
use core::task::Waker;
fn main() {
    Waker::noop().clone().wake(); //~ clippy::waker_clone_wake
}
