//! Canary.
use std::sync::Mutex;

#[rustfmt::skip]
/// Canary item.
fn locker(mtx: &mut Mutex<u8>) { //~ clippy::needless_pass_by_ref_mut
    drop(mtx.lock().unwrap()); //~ clippy::unwrap_used clippy::mut_mutex_lock
}
#[rustfmt::skip]
fn main() {
    let mut mtx = Mutex::new(0); //~ clippy::mutex_integer
    locker(&mut mtx);
}
