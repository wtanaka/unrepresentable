//! Canary.
use std::sync::Mutex;

#[rustfmt::skip]
fn main() {
    let mutex: Mutex<Option<u8>> = Mutex::new(Some(1));
    let lock = mutex.lock();
    drop(lock);
    if let Some(value) = *mutex.lock().unwrap() { //~ clippy::if_let_mutex clippy::unwrap_used clippy::significant_drop_in_scrutinee if_let_rescope
        assert!(value == 1, "one");
    } else {
        *mutex.lock().unwrap() = Some(2); //~ clippy::unwrap_used
    }
    assert!(mutex.is_poisoned(), "poisoned");
}
