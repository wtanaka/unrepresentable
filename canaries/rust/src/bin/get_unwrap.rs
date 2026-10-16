//! Canary.
#[rustfmt::skip]
fn main() {
    let items: Vec<u8> = vec![1, 2];
    let first = items.get(1).unwrap(); //~ clippy::get_unwrap clippy::unwrap_used
    assert!(*first == 2, "two");
}
