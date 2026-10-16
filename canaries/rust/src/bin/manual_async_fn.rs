//! Canary.
use core::future::Future;

#[rustfmt::skip]
/// Canary item.
fn compute() -> impl Future<Output = u8> { //~ clippy::manual_async_fn
    async { 1 }
}

fn main() {
    let _fut = compute();
}
