//! Canary.
#[rustfmt::skip]
fn main() {
    let items: Vec<u8> = vec![1, 2];
    for item in items.iter() { //~ clippy::explicit_iter_loop
        assert!(*item > 0, "positive");
    }
}
