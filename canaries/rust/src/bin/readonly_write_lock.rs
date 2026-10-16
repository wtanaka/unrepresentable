//! Canary.
use std::sync::RwLock;

#[rustfmt::skip]
fn main() {
    let lock: RwLock<u8> = RwLock::new(1);
    let guard = lock.write().unwrap(); //~ clippy::readonly_write_lock clippy::unwrap_used
    sink(*guard);
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}
