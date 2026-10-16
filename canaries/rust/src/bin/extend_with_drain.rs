//! Canary.
#[rustfmt::skip]
fn main() {
    let mut left: Vec<u8> = vec![1];
    let mut right: Vec<u8> = vec![2];
    left.extend(right.drain(..)); //~ clippy::extend_with_drain clippy::iter_with_drain
    assert!(left.len() == 2, "two");
}
