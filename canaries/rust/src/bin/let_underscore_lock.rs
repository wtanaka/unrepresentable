//! Canary.
use std::sync::Mutex;

#[rustfmt::skip]
fn main() {
    let mtx: Mutex<()> = Mutex::new(());
    let _: Result<_, _> = mtx.lock(); //~ let_underscore_lock clippy::let_underscore_must_use
}
