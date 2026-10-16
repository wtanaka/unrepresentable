//! Canary.
use core::future::ready;
use core::ptr::null;

#[rustfmt::skip]
/// Canary item.
async fn hold() -> u8 { //~ clippy::future_not_send
    let ptr: *const u8 = null();
    let other = ready(1).await;
    assert!(ptr.is_null(), "null");
    other
}

fn main() {
    let future = hold();
    assert!(size_of_val(&future) > 0, "sized");
}
