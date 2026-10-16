//! Canary.
use std::sync::Mutex;

#[rustfmt::skip]
fn main() {
    let lock = Mutex::new(Vec::<u8>::new());
    let text = match lock.lock().unwrap().len() { //~ clippy::significant_drop_in_scrutinee clippy::unwrap_used
        0 => "empty",
        _ => "full",
    };
    assert!(!text.is_empty(), "text");
}
