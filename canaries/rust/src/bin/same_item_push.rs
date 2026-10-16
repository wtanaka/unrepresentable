//! Canary.
#[rustfmt::skip]
fn main() {
    let mut list: Vec<u8> = Vec::new();
    let other: Vec<u8> = Vec::new();
    for _ in &other {
        list.push(1); //~ clippy::same_item_push
    }
    drop(list);
}
