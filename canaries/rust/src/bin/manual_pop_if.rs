//! Canary.
#[rustfmt::skip]
/// Canary item.
fn trim(items: &mut Vec<u8>) {
    if items.last().is_some_and(|last| *last > 1) { //~ clippy::manual_pop_if
        items.pop().unwrap(); //~ clippy::unwrap_used unused_results
    }
}

fn main() {
    let mut items: Vec<u8> = Vec::new();
    items.push(2);
    trim(&mut items);
    assert!(items.is_empty(), "empty");
}
