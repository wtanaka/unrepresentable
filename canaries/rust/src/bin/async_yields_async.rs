//! Canary.
use core::future::ready;

fn main() {
    let fut = async { ready(()) }; //~ clippy::async_yields_async
    drop(fut);
}
