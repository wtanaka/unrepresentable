//! Canary.
use core::future::ready;

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}

#[rustfmt::skip]
fn main() {
    let fut = ready::<u8>(1);
    sink(async move { fut.await }); //~ clippy::redundant_async_block
}
