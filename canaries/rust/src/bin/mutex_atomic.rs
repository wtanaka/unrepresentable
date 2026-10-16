//! Canary.
use std::sync::Mutex;

fn main() {
    let flag = Mutex::new(true); //~ clippy::mutex_atomic
    assert!(!flag.is_poisoned(), "poisoned");
}
