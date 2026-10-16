//! Canary.
#[rustfmt::skip]
fn main() {
    let mut items: Vec<u8> = Vec::new();
    items.push(1);
    for item in &mut items { //~ clippy::manual_slice_fill
        *item = 0;
    }
    assert_eq!(items.len(), 1, "len");
}
