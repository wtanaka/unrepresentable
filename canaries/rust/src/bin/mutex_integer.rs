//! Canary.
use std::sync::Mutex;

fn main() {
    let count: Mutex<u8> = Mutex::new(0); //~ clippy::mutex_integer
    assert!(!count.is_poisoned(), "poisoned");
}
