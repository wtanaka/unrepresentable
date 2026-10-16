//! Canary.
/// Canary item.
fn wipe(items: &mut Vec<u8>) {
    items.truncate(0); //~ clippy::manual_clear
}

fn main() {
    let mut items: Vec<u8> = Vec::new();
    items.push(1);
    wipe(&mut items);
    assert!(items.is_empty(), "empty");
}
