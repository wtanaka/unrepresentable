//! Canary.
/// Canary item.
fn drain(items: &mut Vec<u8>) {
    while !items.is_empty() {
        let item = items.pop().unwrap(); //~ clippy::manual_while_let_some clippy::unwrap_used
        assert!(item > 0, "item");
    }
}

fn main() {
    let mut items: Vec<u8> = Vec::new();
    items.push(1);
    drain(&mut items);
}
