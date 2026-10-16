//! Canary.
use core::future::ready;

/// Canary item.
async fn inner() -> usize { //~ clippy::large_stack_frames
    let arr: [u8; 20_000] = [0; 20_000]; //~ clippy::large_stack_arrays
    ready(()).await;
    arr.len()
}

/// Canary item.
async fn outer() -> usize {
    inner().await //~ clippy::large_futures
}

fn main() {
    let fut = outer();
    assert!(size_of_val(&fut) > 1, "big");
}
