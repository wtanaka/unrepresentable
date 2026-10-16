//! Canary.
use core::hint::black_box;

#[rustfmt::skip]
fn main() {
    let opt: Option<u8> = black_box(None);
    if opt.is_none() {
        opt.unwrap(); //~ clippy::panicking_unwrap clippy::unwrap_used unused_results
    }
}
