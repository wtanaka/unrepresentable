//! Canary.
#[rustfmt::skip]
fn main() {
    let mut items: Vec<u8> = vec![1];
    let taken: Vec<u8> = items.drain(..).collect(); //~ clippy::drain_collect clippy::needless_collect clippy::iter_with_drain
    assert!(taken.len() == 1, "one");
}
