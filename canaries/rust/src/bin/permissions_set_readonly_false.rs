//! Canary.
use std::fs;

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}

#[rustfmt::skip]
fn main() {
    let Ok(meta) = fs::metadata(".") else { return };
    let mut perms = meta.permissions();
    perms.set_readonly(false); //~ clippy::permissions_set_readonly_false
    sink(perms);
}
