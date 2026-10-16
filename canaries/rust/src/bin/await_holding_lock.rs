//! Canary.
use core::future::ready;
use std::sync::Mutex;

/// Canary item.
async fn other() {
    ready(()).await;
}

#[rustfmt::skip]
fn main() {
    let mtx = Mutex::new(String::new());
    let fut = async {
        if let Ok(guard) = mtx.lock() { //~ clippy::await_holding_lock
            other().await;
            drop(guard);
        }
    };
    drop(fut);
}
