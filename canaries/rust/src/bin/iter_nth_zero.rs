//! Canary.
fn main() {
    let data: Vec<u8> = Vec::new();
    let mut iter = data.iter();
    let item = iter.nth(0); //~ clippy::iter_nth_zero
    assert!(item.is_none(), "empty");
}
