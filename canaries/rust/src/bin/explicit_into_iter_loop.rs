//! Canary.
#[rustfmt::skip]
fn main() {
    let items: Vec<u8> = vec![1, 2];
    for item in items.into_iter() { //~ clippy::explicit_into_iter_loop
        assert!(item > 0, "positive");
    }
}
