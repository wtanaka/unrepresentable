//! Canary.
use std::sync::Mutex;

#[rustfmt::skip]
fn main() {
    let lock = Mutex::new(Vec::<u8>::new());
    {
        let guard = lock.lock().unwrap(); //~ clippy::significant_drop_tightening clippy::unwrap_used
        assert!(guard.is_empty(), "empty");
        assert!(lock.is_poisoned(), "poisoned");
    }
}
